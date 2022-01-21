use anyhow::{anyhow, ensure, Result};
use log::debug;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::HashMap};

pub struct Fighter {
    pub data: FighterData,
    pub state: String,
    pub time: f32,
    pub translate_x: f32,
    pub translate_y: f32,
    pub hp: f32,
    pub facing: f32,
    pub input_state: InputState,
    pub velocity_x: f32,
    pub velocity_y: f32,
    pub gravity: f32,
    pub on_ground: bool,
    pub stun: f32,
}
impl Fighter {
    pub fn new(data: FighterData) -> Fighter {
        Fighter {
            data,
            state: "stand".to_string(),
            time: 0.0,
            translate_x: 0.0,
            translate_y: 0.0,
            hp: 1.0,
            facing: 1.0,
            input_state: Default::default(),
            velocity_x: 0.0,
            velocity_y: 0.0,
            gravity: -980.0 * 5.0,
            on_ground: false,
            stun: 0.0,
        }
    }

    pub fn on_hit(&mut self, hit: HitData, facing: f32) -> Result<()> {
        let state = &self.data.states[&self.state];
        if state.blocking {
            self.hp -= hit.block_damage;
            self.stun = self.stun.max(hit.block_stun);
            let t = (-2.0 * hit.block_knockback_x / self.gravity).sqrt();
            self.velocity_x += (2.0 * hit.block_knockback_x / t) * facing;
        } else {
            self.hp -= hit.hit_damage;
            self.stun = self.stun.max(hit.hit_stun);
            let t = (-2.0 * hit.hit_knockback_x / self.gravity).sqrt();
            self.velocity_x += (2.0 * hit.hit_knockback_x / t) * facing;
        }
        debug!("self.velocity_x: {}", self.velocity_x);
        if let Some(next) = &state.on_hit_transition {
            let next = next.clone();
            self.change_state(next)?;
        }

        Ok(())
    }

    pub fn transition_check(&mut self) -> Result<()> {
        let current_state = self
            .data
            .states
            .get(&self.state)
            .ok_or(anyhow!("State not find in data, state: {}", self.state))?;
        // Input

        let next = self.check_input_transition(&current_state.input_transition);
        if let Some(next) = next {
            self.change_state(next)?;
            return Ok(());
        }
        // Ground / Air
        if self.on_ground {
            // on ground
            if let Some(next) = &current_state.on_ground_transition {
                let next = next.clone();
                self.change_state(next)?;
                return Ok(());
            }
        } else {
            // on air
            if let Some(next) = &current_state.on_air_transition {
                let next = next.clone();
                self.change_state(next)?;
                return Ok(());
            }
        }
        Ok(())
    }

    fn change_state(&mut self, next: String) -> Result<()> {
        ensure!(
            self.data.states.contains_key(&next),
            "Fighter does not have state: {}",
            next
        );
        debug!("change_state: {}", next);
        // exit state
        // ...

        let state = self.data.states.get(&next).unwrap();
        self.state = next;
        self.time = 0.0;

        // enter state
        if state.jump_height != 0.0 {
            // let t = 0.75;
            // self.velocity_y = 2.0 * state.jump_height / t;
            // self.gravity = -2.0 * state.jump_height / (t * t);
            // debug!("vel:{} g:{} t:{}", self.velocity_y, self.gravity, t);
            let t = (-2.0 * state.jump_height / self.gravity).sqrt();
            self.velocity_y = 2.0 * state.jump_height / t;
            debug!("vel:{} g:{} t:{}", self.velocity_y, self.gravity, t);
        }

        Ok(())
    }

    pub fn update(&mut self, delta: f32) -> Result<()> {
        let state_data = self
            .data
            .states
            .get(&self.state)
            .ok_or(anyhow!("State not find in data, state: {}", self.state))?;

        // state update
        // translate
        if self.velocity_x > 0.0 {
            self.velocity_x = self.velocity_x - self.gravity.abs() * delta;
            if self.velocity_x < 0.0 {
                self.velocity_x = 0.0;
            }
        } else if self.velocity_x < 0.0 {
            self.velocity_x = self.velocity_x + self.gravity.abs() * delta;
            if self.velocity_x > 0.0 {
                self.velocity_x = 0.0;
            }
        }
        let x =
            (state_data.move_forward - state_data.move_backward) * self.facing + self.velocity_x;
        self.translate_x = x * delta;

        let y = self.velocity_y;
        self.velocity_y = self.velocity_y + self.gravity * delta;
        if self.velocity_y <= 0.0 && self.on_ground {
            self.velocity_y = 0.0
        }
        self.translate_y = y * delta;
        // debug!("translate_y: {}", self.translate_y);

        // Facing
        // if self.translate_x > 0.0 {
        //     self.facing = 1.0
        // } else if self.translate_x < 0.0 {
        //     self.facing = -1.0
        // }

        // Stun
        if self.stun > 0.0 {
            self.stun -= delta;
        }
        if self.stun <= 0.0 {
            self.stun = 0.0;
            if let Some(next) = &state_data.on_stun_end_transition {
                let next = next.clone();
                self.change_state(next)?;
                return Ok(());
            }
        }

        // Duration
        self.time += delta;
        if self.time >= state_data.duration {
            self.time = self.time % state_data.duration;
            if let Some(next) = &state_data.auto_transition {
                let next = next.clone();
                self.change_state(next)?;
                return Ok(());
            }

            let next = self.check_input_transition(&state_data.on_duration_end_input_transition);
            if let Some(next) = next {
                self.change_state(next)?;
                return Ok(());
            }
        }
        Ok(())
    }

    fn check_input_transition(&self, tran_map: &HashMap<String, String>) -> Option<String> {
        let nexts: Vec<(&String, &String)> = tran_map
            .iter()
            .filter(|(input, _)| {
                input.split(",").all(|input| match input {
                    "backward" => {
                        if self.facing == 1.0 {
                            self.input_state.left
                        } else {
                            self.input_state.right
                        }
                    }
                    "forward" => {
                        if self.facing == 1.0 {
                            self.input_state.right
                        } else {
                            self.input_state.left
                        }
                    }
                    "up" => self.input_state.up,
                    "down" => self.input_state.down,
                    "throw" => self.input_state.throw,
                    "light" => self.input_state.light,
                    "heavy" => self.input_state.heavy,
                    "!backward" => {
                        if self.facing == 1.0 {
                            !self.input_state.left
                        } else {
                            !self.input_state.right
                        }
                    }
                    "!forward" => {
                        if self.facing == 1.0 {
                            !self.input_state.right
                        } else {
                            !self.input_state.left
                        }
                    }
                    "!up" => !self.input_state.up,
                    "!down" => !self.input_state.down,
                    "!throw" => !self.input_state.throw,
                    "!light" => !self.input_state.light,
                    "!heavy" => !self.input_state.heavy,
                    "" => true,
                    _ => false,
                })
            })
            .collect();
        if !nexts.is_empty() {
            let next = nexts
                .iter()
                .max_by(|(a, _), (b, _)| {
                    let ord = a.matches(",").count().cmp(&b.matches(",").count());
                    if ord == Ordering::Equal {
                        return a.len().cmp(&b.len());
                    } else {
                        return ord;
                    }
                })
                .unwrap()
                .1
                .to_string();
            return Some(next);
        } else {
            return None;
        }
    }
    pub fn collision_boxes(&self) -> Result<HashMap<String, CollisionBox>> {
        let state_data = self
            .data
            .states
            .get(&self.state)
            .ok_or(anyhow!("State not find in data, state: {}", self.state))?;

        let mut boxes = self.data.collision_boxes.clone();

        for (k, v) in &state_data.collision_boxes {
            let c = boxes
                .get_mut(k)
                .ok_or(anyhow!("Collision box not found: {}", k))?;
            c.extent = v.extent;
            c.position = v.position;
        }
        Ok(boxes)
    }
    pub fn hurt_boxes(&self) -> Result<HashMap<String, HurtBox>> {
        let state_data = self
            .data
            .states
            .get(&self.state)
            .ok_or(anyhow!("State not find in data, state: {}", self.state))?;

        Ok(state_data.hurt_boxes.clone())
    }
    pub fn hit_boxes(&self) -> Result<HashMap<String, HitBox>> {
        let state_data = self
            .data
            .states
            .get(&self.state)
            .ok_or(anyhow!("State not find in data, state: {}", self.state))?;

        Ok(state_data.hit_boxes.clone())
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct FighterData {
    pub name: String,
    pub resources: HashMap<String, String>,
    pub collision_boxes: HashMap<String, CollisionBox>,
    pub states: HashMap<String, State>,
}

impl FighterData {
    fn validate(&self) -> Result<(), Vec<String>> {
        let valid_input = vec![
            "backward",
            "forward",
            "up",
            "down",
            "throw",
            "light",
            "heavy",
            "!backward",
            "!forward",
            "!up",
            "!down",
            "!throw",
            "!light",
            "!heavy",
        ];
        // let mut errors: Vec<String> = Vec::new();
        let errors: Vec<String> = self
            .states
            .iter()
            .map(|(name, state)| {
                let mut input_not_found = state
                    .input_transition
                    .keys()
                    .filter(|input| input.split(",").any(|s| !valid_input.contains(&s)))
                    .map(|s| format!("State({}) input_transition key({}) not correct. ", name, s))
                    .collect();
                let mut state_not_found = state
                    .input_transition
                    .values()
                    .filter(|state| !self.states.keys().any(|s| &s == state))
                    .map(|s| {
                        format!(
                            "State({}) input_transition value({}) not correct. ",
                            name, s
                        )
                    })
                    .collect();
                let mut resource_not_found = state
                    .animation
                    .iter()
                    .filter(|name| !self.resources.contains_key(*name))
                    .map(|s| format!("Animation resource missing: {}", s))
                    .collect();
                let mut e = Vec::new();
                e.append(&mut input_not_found);
                e.append(&mut state_not_found);
                e.append(&mut resource_not_found);
                e
            })
            .filter(|e| !e.is_empty())
            .flatten()
            .collect();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InputState {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub throw: bool,
    pub light: bool,
    pub heavy: bool,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct State {
    pub duration: f32,
    pub animation: Vec<String>,
    // Transition
    pub input_transition: HashMap<String, String>,
    pub on_duration_end_input_transition: HashMap<String, String>,
    pub auto_transition: Option<String>,
    pub on_air_transition: Option<String>,
    pub on_ground_transition: Option<String>,
    pub on_hit_transition: Option<String>,
    pub on_stun_end_transition: Option<String>,
    // Collision
    pub collision_boxes: HashMap<String, CollisionBox>,
    // Hurt
    pub hurt_boxes: HashMap<String, HurtBox>,
    // Hit
    pub hit_boxes: HashMap<String, HitBox>,
    pub hit_data: HitData,
    // Block
    pub blocking: bool,
    // Throw
    pub throw_boxes: HashMap<String, ThrowBox>,
    pub throw_trasition: Option<String>,
    pub throw_escape: bool,
    pub throw_lock: bool,
    pub throw_damage: f32,
    pub throw_target_x: f32,
    pub throw_target_y: f32,
    pub throw_target_state: f32,
    pub throw_interrupt_transition: Option<String>,
    // Walk
    pub move_backward: f32,
    pub move_forward: f32,
    // Jump
    pub jump_height: f32,
    // Facing
    pub auto_facing: bool,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct CollisionBox {
    pub position: (f32, f32),
    pub extent: (f32, f32),
    pub disable: bool,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct HitBox {
    pub position: (f32, f32),
    pub extent: (f32, f32),
    pub disable: bool,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct HitData {
    pub hit_stun: f32,
    pub hit_damage: f32,
    pub hit_knockback_x: f32,
    pub hit_knockback_y: f32,
    pub knockdown: bool,
    pub block_stun: f32,
    pub block_damage: f32,
    pub block_knockback_x: f32,
    pub block_knockback_y: f32,
}
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct HurtBox {
    pub position: (f32, f32),
    pub extent: (f32, f32),
    pub disable: bool,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ThrowBox {
    pub position: (f32, f32),
    pub extent: (f32, f32),
}

#[cfg(test)]
mod tests {
    use super::FighterData;
    use ron::de;
    use std::fs::File;

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn de_from_file() -> Result<(), ron::Error> {
        let input_path = format!("{}/assets/fighter.ron", env!("CARGO_MANIFEST_DIR"));
        let f = File::open(&input_path).expect("Failed opening file");
        let _fighter: FighterData = de::from_reader(f)?;
        Ok(())
    }

    #[test]
    fn valid_data() -> Result<(), anyhow::Error> {
        let input_path = format!("{}/assets/fighter.ron", env!("CARGO_MANIFEST_DIR"));
        let f = File::open(&input_path).expect("Failed opening file");
        let data: FighterData = de::from_reader(f)?;
        data.validate().map_err(|v| anyhow::anyhow!(v.join("\n")))?;
        Ok(())
    }
}

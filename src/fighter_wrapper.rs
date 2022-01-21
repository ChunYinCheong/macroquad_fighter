use std::{collections::HashMap, fs::File};

use anyhow::Result;
use macroquad::prelude::*;
use macroquad_platformer::{Actor, World};

use crate::{
    fighter::{CollisionBox, Fighter, FighterData, HitBox, HurtBox},
    fighter_ai::{FighterAi, Setting},
};

pub struct FighterWrapper {
    pub fighter: Fighter,
    pub collider: Actor,
    pub ai: Option<FighterAi>,
    pub data_path: String,
    pub ai_path: Option<String>,
    pub resources: HashMap<String, Texture2D>,
}

impl FighterWrapper {
    pub fn new(
        world: &mut World,
        x: f32,
        y: f32,
        data_path: String,
        ai_path: Option<String>,
    ) -> Self {
        let f = File::open(&data_path).expect("Failed opening file");
        let data: FighterData = ron::de::from_reader(f).unwrap();
        let collision = data.collision_boxes.values().next().unwrap().clone();

        let ai = match &ai_path {
            Some(input_path) => {
                let f = File::open(&input_path).expect("Failed opening file");
                let settings: Vec<Setting> = ron::de::from_reader(f).unwrap();
                Some(FighterAi::new(settings))
            }
            None => None,
        };
        Self {
            fighter: Fighter::new(data.clone()),
            collider: world.add_actor(
                vec2(x, y),
                collision.extent.0 as i32 * 2,
                collision.extent.1 as i32 * 2,
            ),
            ai,
            data_path,
            ai_path,
            resources: Default::default(),
        }
    }

    pub async fn load_resource(&mut self) -> Result<()> {
        self.resources = HashMap::new();
        for (k, v) in &self.fighter.data.resources {
            let path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), v);
            let texture: Texture2D = load_texture(&path).await?;
            self.resources.insert(k.clone(), texture);
        }
        Ok(())
    }

    pub fn update(&mut self, world: &mut World) -> Result<()> {
        // On ground
        let pos = world.actor_pos(self.collider);
        let on_ground = world.collide_check(self.collider, pos + vec2(0., 1.));
        self.fighter.on_ground = on_ground;

        self.fighter.transition_check()?;
        self.fighter.update(get_frame_time())?;
        world.move_h(self.collider, self.fighter.translate_x);
        world.move_v(self.collider, -self.fighter.translate_y);
        Ok(())
    }

    pub fn position(&self, world: &World) -> (f32, f32) {
        let collision = self.fighter.collision_boxes().unwrap();
        let collision = collision.values().next().unwrap();
        let pos = world.actor_pos(self.collider);
        let px = pos.x + collision.extent.0;
        let py = pos.y + collision.extent.1 * 2.0;
        (px, -py)
    }

    pub fn world_xywh_by_x1x2y1y2(
        &self,
        world: &World,
        x1: f32,
        x2: f32,
        y1: f32,
        y2: f32,
    ) -> (f32, f32, f32, f32) {
        let (px, py) = self.position(world);
        let x = px + x1 * self.fighter.facing;
        let w = (x2 - x1) * self.fighter.facing;
        let y = py + y1;
        let h = y2 - y1;
        (x, -y, w, -h)
    }

    pub fn world_xywh_by_rect(&self, world: &World, rect: Rect) -> (f32, f32, f32, f32) {
        let (px, py) = self.position(world);
        let x = px + rect.position.0 * self.fighter.facing - rect.extent.0;
        let y = py + rect.position.1 + rect.extent.1;
        let w = rect.extent.0 * 2.0;
        let h = rect.extent.1 * 2.0;
        (x, -y, w, h)
    }

    pub fn x1x2y1y2_by_rect(&self, world: &World, rect: Rect) -> (f32, f32, f32, f32) {
        let (px, py) = self.position(world);
        let x1 = px + rect.position.0 * self.fighter.facing - rect.extent.0;
        let x2 = x1 + rect.extent.0 * 2.0;
        let y1 = py + rect.position.1 + rect.extent.1;
        let y2 = y1 + rect.extent.1 * 2.0;
        (x1, x2, y1, y2)
    }

    pub fn xxyy(&self, world: &World, rect: Rect) -> (f32, f32, f32, f32) {
        let (px, py) = self.position(world);
        let x1 = px + rect.position.0 * self.fighter.facing - rect.extent.0;
        let x2 = px + rect.position.0 * self.fighter.facing + rect.extent.0;
        let y1 = py + rect.position.1 - rect.extent.1;
        let y2 = py + rect.position.1 + rect.extent.1;
        (x1, x2, y1, y2)
    }
}

pub struct Rect {
    pub position: (f32, f32),
    pub extent: (f32, f32),
}

impl From<CollisionBox> for Rect {
    fn from(b: CollisionBox) -> Self {
        Self {
            position: b.position,
            extent: b.extent,
        }
    }
}

impl From<HurtBox> for Rect {
    fn from(b: HurtBox) -> Self {
        Self {
            position: b.position,
            extent: b.extent,
        }
    }
}

impl From<HitBox> for Rect {
    fn from(b: HitBox) -> Self {
        Self {
            position: b.position,
            extent: b.extent,
        }
    }
}
impl From<&CollisionBox> for Rect {
    fn from(b: &CollisionBox) -> Self {
        Self {
            position: b.position,
            extent: b.extent,
        }
    }
}

impl From<&HurtBox> for Rect {
    fn from(b: &HurtBox) -> Self {
        Self {
            position: b.position,
            extent: b.extent,
        }
    }
}

impl From<&HitBox> for Rect {
    fn from(b: &HitBox) -> Self {
        Self {
            position: b.position,
            extent: b.extent,
        }
    }
}

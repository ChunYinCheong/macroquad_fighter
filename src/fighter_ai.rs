use log::debug;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct FighterAi {
    pub history: Vec<Frame>,
    pub delay: i32,
    pub settings: Vec<Setting>,
}

impl Default for FighterAi {
    fn default() -> Self {
        Self {
            delay: 30,
            history: Default::default(),
            settings: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct Frame {
    pub opponent_state: Vec<String>,
    pub opponent_x: f32,
    pub opponent_y: f32,
    pub state: Vec<String>,
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct Setting {
    pub distance_x: (f32, f32),
    pub distance_y: (f32, f32),
    pub state: Option<String>,
    pub opponent_state: Option<String>,
    pub ai_input: AiInput,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct AiInput {
    pub backward: bool,
    pub forward: bool,
    pub up: bool,
    pub down: bool,
    pub throw: bool,
    pub light: bool,
    pub heavy: bool,
}

impl FighterAi {
    pub fn new(settings: Vec<Setting>) -> Self {
        Self {
            settings,
            ..Default::default()
        }
    }

    pub fn add_frame(&mut self, frame: Frame) {
        self.history.push(frame);
    }

    pub fn input(&self, dx: f32, dy: f32) -> AiInput {
        let list: Vec<_> = self
            .settings
            .iter()
            .filter(|s| {
                s.distance_x.0 <= dx
                    && dx < s.distance_x.1
                    && s.distance_y.0 <= dy
                    && dy < s.distance_y.1
            })
            .collect();
        debug!("{:?}", list);

        self.settings
            .iter()
            .filter(|s| {
                s.distance_x.0 <= dx
                    && dx < s.distance_x.1
                    && s.distance_y.0 <= dy
                    && dy < s.distance_y.1
            })
            .next()
            .map(|s| s.ai_input.clone())
            .unwrap_or_default()
    }
}

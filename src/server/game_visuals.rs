//! Visual variants keep animation/metasprite/CHR assets shared and use NES palette/flip bits.
use super::game_combat::AttackMode;
use super::game_design::GameDesign;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct SpriteVisual {
    pub palette: Option<u8>,
    pub flip_x: bool,
    pub flip_y: bool,
    pub face_movement: bool,
}

impl SpriteVisual {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.palette.is_some_and(|palette| palette > 3) {
            return Err("Sprite variant palette must be 0..3".into());
        }
        Ok(())
    }
    pub fn draw(&self, x: &str, y: &str, state: &str, facing: &str) -> String {
        if self.is_default() {
            return format!("Animation.Draw({x}, {y}, {state})");
        }
        let base = u8::from(self.flip_x) * 64 + u8::from(self.flip_y) * 128;
        let flags = if self.face_movement {
            format!("{base} XOR ({facing} AND 64)")
        } else {
            base.to_string()
        };
        format!(
            "Animation.DrawVariant({x}, {y}, {state}, {}, {flags})",
            self.palette.unwrap_or(255)
        )
    }
}

impl GameDesign {
    pub fn visual_ram_bytes(&self) -> usize {
        usize::from(
            (self.animation.is_some() && self.player_visual.face_movement)
                || self
                    .combat
                    .as_ref()
                    .is_some_and(|combat| combat.attack_mode == AttackMode::Facing),
        )
    }
}

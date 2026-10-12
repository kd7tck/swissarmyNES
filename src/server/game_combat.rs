//! Optional combat appends state without moving existing Studio variables or room slots.
use super::game_design::{EntityBehavior, EntityDefinition, GameDesign, Trigger};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Write};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Hitbox {
    pub x: u8,
    pub y: u8,
    pub width: u8,
    pub height: u8,
}

impl Default for Hitbox {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 16,
            height: 16,
        }
    }
}

impl Hitbox {
    pub fn validate(self) -> Result<(), String> {
        if self.width == 0
            || self.height == 0
            || u16::from(self.x) + u16::from(self.width) > 16
            || u16::from(self.y) + u16::from(self.height) > 16
        {
            return Err(
                "Hitboxes need positive dimensions and must fit inside the 16x16 footprint".into(),
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatSettings {
    pub health: u8,
    pub invulnerability_frames: u8,
    pub attack_button: Trigger,
    pub attack_damage: u8,
    pub attack_reach: u8,
    pub death_scene: String,
    #[serde(default)]
    pub attack_mode: AttackMode,
    #[serde(default)]
    pub attack_cooldown_frames: u8,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttackMode {
    #[default]
    AllDirections,
    Facing,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EnemyMotion {
    #[default]
    Patrol,
    Pursue,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct EnemySettings {
    pub health: u8,
    pub damage: u8,
    pub motion: EnemyMotion,
    pub detection_range: u8,
}

impl Default for EnemySettings {
    fn default() -> Self {
        Self {
            health: 1,
            damage: 1,
            motion: EnemyMotion::Patrol,
            detection_range: 64,
        }
    }
}

impl GameDesign {
    pub fn attack_ram_bytes(&self) -> usize {
        if self
            .combat
            .as_ref()
            .is_some_and(|combat| combat.attack_cooldown_frames > 0)
        {
            2
        } else {
            0
        }
    }

    // Optional attack state follows every previous declaration, preserving all old offsets.
    pub fn attack_address(&self) -> Option<usize> {
        (self.attack_ram_bytes() > 0).then(|| 0x5c0 + self.generated_ram_bytes() - 2)
    }

    pub(super) fn attack_declarations(&self, source: &mut String) {
        if self.attack_ram_bytes() > 0 {
            source.push_str("DIM gf_attack_cooldown AS BYTE\nDIM gf_attacking AS BYTE\n");
        }
    }

    pub(super) fn attack_tick(&self, source: &mut String) {
        if self.attack_ram_bytes() > 0 {
            source.push_str("    IF gf_attack_cooldown > 0 THEN\n        gf_attack_cooldown = gf_attack_cooldown - 1\n    END IF\n");
        }
    }

    pub(super) fn attack_update(&self, source: &mut String) {
        if self.attack_ram_bytes() == 0 {
            return;
        }
        let combat = self.combat.as_ref().unwrap();
        let room_guard = if self.entity_slots() > 0 || !self.events.is_empty() {
            "(gf_changed = 0) AND "
        } else {
            ""
        };
        // Compute one swing before dispatch so every enemy/pool sees the same decision.
        // A press during cooldown is discarded; held input cannot queue a later attack.
        writeln!(source, "    gf_attacking = 0\n    IF {room_guard}gf_movable AND (gf_health > 0) AND (gf_attack_cooldown = 0) AND Controller.IsPressed(Button.{:?}) THEN\n        gf_attacking = 255\n        gf_attack_cooldown = {}\n    END IF", combat.attack_button, combat.attack_cooldown_frames).unwrap();
    }

    // Combat follows all legacy declarations, keeping variable_address stable.
    pub fn combat_address(&self) -> Option<usize> {
        self.combat
            .as_ref()
            .map(|_| 0x5c0 + self.entity_ram_bytes() + self.event_ram_bytes())
    }

    pub fn combat_ram_bytes(&self) -> usize {
        if self.combat.is_some() {
            4 + self.entity_slots()
        } else {
            0
        }
    }

    pub(super) fn validate_combat(&self) -> Result<(), String> {
        self.player_hitbox.validate()?;
        for entity in &self.entities {
            if entity.behavior != EntityBehavior::Spawner {
                entity.hitbox.validate()?;
            }
            if entity.behavior == EntityBehavior::Enemy {
                if self.combat.is_none() {
                    return Err("Enable combat before using enemy definitions".into());
                }
                if entity.enemy.health == 0 || entity.enemy.damage == 0 {
                    return Err("Enemies need health and contact damage of 1..255".into());
                }
                if entity.enemy.motion == EnemyMotion::Pursue && entity.enemy.detection_range == 0 {
                    return Err("Pursuing enemies need a detection range of 1..255 pixels".into());
                }
            }
        }
        if let Some(combat) = &self.combat {
            if combat.health == 0
                || combat.invulnerability_frames == 0
                || combat.attack_damage == 0
                || combat.attack_reach > 32
                || !matches!(combat.attack_button, Trigger::A | Trigger::B)
            {
                return Err("Combat needs health/damage/immunity of 1..255, reach 0..32 and attack button A or B".into());
            }
            if !self
                .scenes
                .iter()
                .any(|scene| scene.id == combat.death_scene)
            {
                return Err("Combat needs an existing defeat scene".into());
            }
        }
        for scene in &self.scenes {
            // Expanded roles also validate the bounds inherited by spawned enemies.
            for role in self.scene_entities(scene)? {
                if role.definition.behavior == EntityBehavior::Enemy
                    && role.definition.enemy.motion == EnemyMotion::Pursue
                    && !role
                        .instance
                        .patrol_end_x
                        .is_some_and(|end| end > role.instance.x && end <= 240)
                {
                    return Err(format!(
                        "Pursuing enemy '{}' needs a right endpoint greater than X and at most 240",
                        role.definition.name
                    ));
                }
            }
        }
        Ok(())
    }

    pub(super) fn combat_declarations(&self, source: &mut String) {
        if self.combat.is_none() {
            return;
        }
        source.push_str(
            "DIM gf_health AS BYTE\nDIM gf_invulnerable AS BYTE\nDIM gf_defeated AS WORD\n",
        );
        for slot in 0..self.entity_slots() {
            writeln!(source, "DIM gf_e{slot}_health AS BYTE").unwrap();
        }
    }

    pub(super) fn enemy_source(
        &self,
        source: &mut String,
        entity: &EntityDefinition,
        slot: usize,
        y: u8,
        indices: &HashMap<&str, usize>,
    ) {
        let combat = self.combat.as_ref().unwrap();
        let p = self.player_hitbox;
        let e = entity.hitbox;
        let reach = combat.attack_reach;
        let pressed = if combat.attack_cooldown_frames > 0 {
            "gf_attacking".into()
        } else {
            format!("Controller.IsPressed(Button.{:?})", combat.attack_button)
        };
        // Expanded intervals avoid subtracting reach near screen edges (WORD underflow).
        let pr = u16::from(p.x) + u16::from(p.width);
        let pb = u16::from(p.y) + u16::from(p.height);
        let er = u16::from(e.x) + u16::from(e.width);
        let et = u16::from(y) + u16::from(e.y);
        let eb = et + u16::from(e.height);
        let attack = match combat.attack_mode {
            AttackMode::AllDirections => format!("(gf_x + {} < gf_e{slot}_x + {er} + {reach}) AND (gf_e{slot}_x + {} < gf_x + {pr} + {reach}) AND (gf_y + {} < {eb} + {reach}) AND ({et} < gf_y + {pb} + {reach})", p.x, e.x, p.y),
            // Cover the front half of the player box plus reach, with strict vertical overlap.
            // Add reach to opposite endpoints rather than subtracting at the left screen edge.
            AttackMode::Facing => {
                let center = u16::from(p.x) + u16::from(p.width) / 2;
                // Round each front half outward so a one-pixel hitbox can attack either way.
                let left_center = u16::from(p.x) + u16::from(p.width).div_ceil(2);
                format!("(gf_y + {} < {eb}) AND ({et} < gf_y + {pb}) AND (((gf_player_facing = 0) AND (gf_e{slot}_x + {} < gf_x + {pr} + {reach}) AND (gf_x + {center} < gf_e{slot}_x + {er})) OR ((gf_player_facing <> 0) AND (gf_x + {} < gf_e{slot}_x + {er} + {reach}) AND (gf_e{slot}_x + {} < gf_x + {left_center})))", p.y, e.x, p.x, e.x)
            }
        };
        writeln!(source, "            IF gf_movable AND (gf_health > 0) THEN\n                IF {pressed} THEN\n                    IF {attack} THEN\n                        IF gf_e{slot}_health <= {} THEN\n                            gf_e{slot}_health = 0\n                            gf_e{slot}_active = 0\n                            IF gf_defeated < 65535 THEN\n                                gf_defeated = gf_defeated + 1\n                            END IF\n                        ELSE\n                            gf_e{slot}_health = gf_e{slot}_health - {}\n                        END IF\n                    END IF\n                END IF",
            combat.attack_damage, combat.attack_damage).unwrap();
        writeln!(source, "                IF gf_e{slot}_active AND (gf_invulnerable = 0) THEN\n                    IF Collision.Rect(gf_x + {}, gf_y + {}, {}, {}, gf_e{slot}_x + {}, {}, {}, {}) THEN\n                        gf_invulnerable = {}\n                        IF gf_health <= {} THEN\n                            gf_health = 0\n                            gf_scene = {}\n                            GameFlowEnter()\n                        ELSE\n                            gf_health = gf_health - {}\n                        END IF\n                    END IF\n                END IF\n            END IF",
            p.x,p.y,p.width,p.height,e.x,u16::from(y)+u16::from(e.y),e.width,e.height,combat.invulnerability_frames,entity.enemy.damage,indices[combat.death_scene.as_str()],entity.enemy.damage).unwrap();
    }
}

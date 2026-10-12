//! Fixed pools keep authored spawn waves within NES RAM/OAM and compile to SANBasic.
use super::game_design::{EntityBehavior, EntityDefinition, EntityInstance, GameDesign, Scene};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpawnerSettings {
    pub entity: String,
    pub capacity: u8,
    pub interval_frames: u16,
    #[serde(default)]
    pub initial_delay_frames: u16,
    #[serde(default)]
    pub total: u16,
}

pub struct RuntimeEntity<'a> {
    pub slot: usize,
    pub id: String,
    pub instance: &'a EntityInstance,
    pub definition: &'a EntityDefinition,
    pub spawned_by: Option<&'a EntityInstance>,
}

impl GameDesign {
    pub fn scene_entities<'a>(
        &'a self,
        scene: &'a Scene,
    ) -> Result<Vec<RuntimeEntity<'a>>, String> {
        let mut roles = Vec::new();
        // Preserve all authored slot indices; generated pools follow those slots.
        for instance in &scene.instances {
            let definition = self
                .entities
                .iter()
                .find(|entity| entity.id == instance.entity)
                .ok_or("An instance refers to a missing entity definition")?;
            roles.push(RuntimeEntity {
                slot: roles.len(),
                id: instance.id.clone(),
                instance,
                definition,
                spawned_by: None,
            });
        }
        for instance in &scene.instances {
            let definition = self
                .entities
                .iter()
                .find(|entity| entity.id == instance.entity)
                .unwrap();
            if definition.behavior != EntityBehavior::Spawner {
                continue;
            }
            let spawn = definition
                .spawner
                .as_ref()
                .ok_or("Spawner needs enemy and timing settings")?;
            if !(1..=8).contains(&spawn.capacity) {
                return Err("Spawner capacity must be 1..8".into());
            }
            let target = self
                .entities
                .iter()
                .find(|entity| {
                    entity.id == spawn.entity && entity.behavior == EntityBehavior::Enemy
                })
                .ok_or("Spawner must reference an existing enemy definition")?;
            for index in 0..spawn.capacity {
                roles.push(RuntimeEntity {
                    slot: roles.len(),
                    id: format!("@spawn:{}:{}", instance.id, index + 1),
                    instance,
                    definition: target,
                    spawned_by: Some(instance),
                });
            }
        }
        Ok(roles)
    }

    pub fn scene_spawners<'a>(
        &'a self,
        scene: &'a Scene,
    ) -> Vec<(&'a EntityInstance, &'a SpawnerSettings)> {
        scene
            .instances
            .iter()
            .filter_map(|instance| {
                self.entities
                    .iter()
                    .find(|entity| {
                        entity.id == instance.entity && entity.behavior == EntityBehavior::Spawner
                    })
                    .and_then(|definition| definition.spawner.as_ref())
                    .map(|settings| (instance, settings))
            })
            .collect()
    }

    pub fn spawner_slots(&self) -> usize {
        self.scenes
            .iter()
            .map(|scene| self.scene_spawners(scene).len())
            .max()
            .unwrap_or(0)
    }

    pub fn spawner_ram_bytes(&self) -> usize {
        self.spawner_slots() * 4 + if self.spawner_slots() > 0 { 5 } else { 0 }
    }

    pub fn spawner_address(&self, scene: &Scene, id: &str) -> Option<usize> {
        self.scene_spawners(scene)
            .iter()
            .position(|(instance, _)| instance.id == id)
            .map(|index| {
                0x5c0
                    + self.entity_ram_bytes()
                    + self.event_ram_bytes()
                    + self.combat_ram_bytes()
                    + index * 4
            })
    }

    pub(super) fn validate_spawners(&self) -> Result<(), String> {
        for definition in &self.entities {
            if definition.behavior != EntityBehavior::Spawner {
                continue;
            }
            let settings = definition
                .spawner
                .as_ref()
                .ok_or("Spawner needs enemy and timing settings")?;
            if !(1..=8).contains(&settings.capacity) || settings.interval_frames == 0 {
                return Err("Spawner needs capacity 1..8 and interval 1..65535 frames".into());
            }
            if !self.entities.iter().any(|entity| {
                entity.id == settings.entity && entity.behavior == EntityBehavior::Enemy
            }) {
                return Err(format!(
                    "Spawner '{}' must reference an existing enemy definition",
                    definition.name
                ));
            }
        }
        let mut ids = HashSet::new();
        for scene in &self.scenes {
            let roles = self.scene_entities(scene)?;
            if roles.len() > 16 {
                return Err(format!("Scene '{}' needs {} runtime slots including spawn pools; at most 16 are supported",scene.name,roles.len()));
            }
            if roles.iter().any(|role| !ids.insert(role.id.clone())) {
                return Err("Authored and generated spawn instance IDs must not collide".into());
            }
        }
        Ok(())
    }

    pub(super) fn spawner_declarations(&self, source: &mut String) {
        for index in 0..self.spawner_slots() {
            writeln!(
                source,
                "DIM gf_sp{index}_timer AS WORD\nDIM gf_sp{index}_spawned AS WORD"
            )
            .unwrap();
        }
        if self.spawner_slots() > 0 {
            // Shared worker avoids copying large combat/patrol procedures for every pool slot.
            source.push_str("DIM gf_actor_active AS BYTE\nDIM gf_actor_x AS WORD\nDIM gf_actor_dir AS BYTE\nDIM gf_actor_health AS BYTE\n");
        }
    }

    pub(super) fn spawn_enemy_call(
        &self,
        source: &mut String,
        scene: &Scene,
        role: &RuntimeEntity,
        room: usize,
    ) {
        let parent = role.spawned_by.unwrap();
        let index = self
            .scene_spawners(scene)
            .iter()
            .position(|(instance, _)| instance.id == parent.id)
            .unwrap();
        let slot = role.slot;
        writeln!(source,"            gf_actor_active = gf_e{slot}_active\n            gf_actor_x = gf_e{slot}_x\n            gf_actor_dir = gf_e{slot}_dir\n            gf_actor_health = gf_e{slot}_health\n            GameFlowSpawnEnemy{room}_{index}()\n            IF gf_changed = 0 THEN\n                gf_e{slot}_active = gf_actor_active\n                gf_e{slot}_x = gf_actor_x\n                gf_e{slot}_dir = gf_actor_dir\n                gf_e{slot}_health = gf_actor_health\n            END IF").unwrap();
    }

    pub(super) fn spawn_enemy_templates(
        &self,
        source: &mut String,
        indices: &HashMap<&str, usize>,
    ) {
        for (room, scene) in self.scenes.iter().enumerate() {
            for (index, (instance, _)) in self.scene_spawners(scene).iter().enumerate() {
                let roles = self.scene_entities(scene).unwrap();
                let target = roles
                    .iter()
                    .find(|role| {
                        role.spawned_by
                            .is_some_and(|parent| parent.id == instance.id)
                    })
                    .unwrap()
                    .definition;
                let mut body = String::new();
                if instance.patrol_end_x.is_some() {
                    self.enemy_movement_source(&mut body, target, instance, 0);
                }
                self.enemy_source(&mut body, target, 0, instance.y, indices);
                for field in ["active", "x", "dir", "health"] {
                    body = body.replace(&format!("gf_e0_{field}"), &format!("gf_actor_{field}"));
                }
                writeln!(
                    source,
                    "\nSUB GameFlowSpawnEnemy{room}_{index}()\n{body}END SUB"
                )
                .unwrap();
            }
        }
    }

    pub(super) fn spawner_entry(&self, source: &mut String) {
        if self.spawner_slots() == 0 {
            return;
        }
        for index in 0..self.spawner_slots() {
            writeln!(
                source,
                "    gf_sp{index}_timer = 0\n    gf_sp{index}_spawned = 0"
            )
            .unwrap();
        }
        source.push_str("    SELECT CASE gf_scene\n");
        for (room, scene) in self.scenes.iter().enumerate() {
            writeln!(source, "    CASE {room}").unwrap();
            for (index, (_, settings)) in self.scene_spawners(scene).iter().enumerate() {
                writeln!(
                    source,
                    "        gf_sp{index}_timer = {}",
                    settings.initial_delay_frames
                )
                .unwrap();
            }
        }
        source.push_str("    END SELECT\n");
    }

    pub(super) fn spawner_source(&self, source: &mut String) {
        if self.spawner_slots() == 0 {
            return;
        }
        source.push_str("\nSUB GameFlowSpawners()\n    IF (gf_changed = 0) AND (gf_health > 0) THEN\n        SELECT CASE gf_scene\n");
        for (room, scene) in self.scenes.iter().enumerate() {
            let spawners = self.scene_spawners(scene);
            if spawners.is_empty() {
                continue;
            }
            writeln!(source, "        CASE {room}").unwrap();
            let roles = self.scene_entities(scene).unwrap();
            for (index, (instance, settings)) in spawners.iter().enumerate() {
                let quota = if settings.total == 0 {
                    "255".into()
                } else {
                    format!("gf_sp{index}_spawned < {}", settings.total)
                };
                writeln!(source,"            IF {quota} THEN\n                IF gf_sp{index}_timer > 0 THEN\n                    gf_sp{index}_timer = gf_sp{index}_timer - 1\n                END IF\n                IF gf_sp{index}_timer = 0 THEN").unwrap();
                let pool: Vec<_> = roles
                    .iter()
                    .filter(|role| {
                        role.spawned_by
                            .is_some_and(|parent| parent.id == instance.id)
                    })
                    .collect();
                for (choice, role) in pool.iter().enumerate() {
                    if choice > 0 {
                        source.push_str("                    ELSE\n");
                    }
                    let slot = role.slot;
                    let target = role.definition;
                    writeln!(source,"                    IF gf_e{slot}_active = 0 THEN\n                        gf_e{slot}_x = {}\n                        gf_e{slot}_dir = 0\n                        gf_e{slot}_health = {}\n                        Animation.Play(gf_e{slot}_anim, {})\n                        gf_e{slot}_active = 255\n                        gf_sp{index}_timer = {}\n                        IF gf_sp{index}_spawned < 65535 THEN\n                            gf_sp{index}_spawned = gf_sp{index}_spawned + 1\n                        END IF",instance.x,target.enemy.health,target.animation,settings.interval_frames).unwrap();
                }
                for _ in &pool {
                    source.push_str("                    END IF\n");
                }
                source.push_str("                END IF\n            END IF\n");
            }
        }
        source.push_str("        END SELECT\n    END IF\nEND SUB\n");
    }
}

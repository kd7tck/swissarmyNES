//! Scene authoring produces ordinary source through the same compiler as hand-written games.
use super::game_combat::{CombatSettings, EnemyMotion, EnemySettings, Hitbox};
use super::game_events::{GameEvent, GameVariable};
use super::game_spawners::SpawnerSettings;
use super::game_visuals::SpriteVisual;
use super::project::ProjectAssets;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write;

pub const FLOW_FILE: &str = "game_flow.swiss";
pub const MAIN_SOURCE: &str = "INCLUDE \"game_flow.swiss\"\n\nSUB Main()\n    GameFlowStart()\n    DO\n        WAIT_VBLANK\n        GameFlowUpdate()\n        ' Add custom game systems here. The flow has already read the controller.\n    LOOP WHILE 1\nEND SUB\n";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameDesign {
    pub version: u8,
    pub title: String,
    #[serde(default)]
    pub core_loop: String,
    pub start_scene: String,
    #[serde(default)]
    pub animation: Option<String>,
    pub scenes: Vec<Scene>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entities: Vec<EntityDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tests: Vec<FlowTest>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<GameVariable>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<GameEvent>,
    #[serde(default)]
    pub player_hitbox: Hitbox,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub combat: Option<CombatSettings>,
    #[serde(default, skip_serializing_if = "SpriteVisual::is_default")]
    pub player_visual: SpriteVisual,
}

pub const TEST_BOOT_FRAMES: u16 = 10;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FlowTest {
    pub id: String,
    pub name: String,
    pub steps: Vec<TestStep>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TestStep {
    pub frames: u16,
    #[serde(default)]
    pub buttons: Vec<Trigger>,
    #[serde(default)]
    pub expected_scene: Option<String>,
    #[serde(default)]
    pub expected_x: Option<u16>,
    #[serde(default)]
    pub expected_y: Option<u16>,
    #[serde(default)]
    pub expected_pickups: Option<u16>,
    #[serde(default)]
    pub expected_active_entities: Option<u8>,
    #[serde(default)]
    pub expected_health: Option<u8>,
    #[serde(default)]
    pub expected_defeated: Option<u16>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub expected_variables: BTreeMap<String, u16>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub expected_spawns: BTreeMap<String, u16>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Scene {
    pub id: String,
    pub name: String,
    pub map_col: u8,
    pub map_row: u8,
    pub spawn_x: u8,
    pub spawn_y: u8,
    #[serde(default)]
    pub movement: bool,
    #[serde(default)]
    pub music: Option<u8>,
    #[serde(default)]
    pub sfx: Option<u8>,
    #[serde(default)]
    pub transitions: Vec<Transition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instances: Vec<EntityInstance>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EntityDefinition {
    pub id: String,
    pub name: String,
    pub animation: String,
    pub behavior: EntityBehavior,
    #[serde(default)]
    pub hitbox: Hitbox,
    #[serde(default)]
    pub enemy: EnemySettings,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spawner: Option<SpawnerSettings>,
    #[serde(default, skip_serializing_if = "SpriteVisual::is_default")]
    pub visual: SpriteVisual,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EntityBehavior {
    Decoration,
    Pickup,
    Door,
    Patrol,
    Enemy,
    Spawner,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EntityInstance {
    pub id: String,
    pub entity: String,
    pub x: u8,
    pub y: u8,
    #[serde(default)]
    pub target_scene: Option<String>,
    #[serde(default)]
    pub patrol_end_x: Option<u8>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Transition {
    pub target: String,
    pub trigger: Trigger,
    #[serde(default)]
    pub frames: u16,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    A,
    B,
    Start,
    Select,
    Up,
    Down,
    Left,
    Right,
    Timer,
}

impl Trigger {
    pub fn button_index(self) -> Option<u8> {
        match self {
            Self::A => Some(0),
            Self::B => Some(1),
            Self::Select => Some(2),
            Self::Start => Some(3),
            Self::Up => Some(4),
            Self::Down => Some(5),
            Self::Left => Some(6),
            Self::Right => Some(7),
            Self::Timer => None,
        }
    }
}

impl GameDesign {
    pub fn entity_slots(&self) -> usize {
        self.scenes
            .iter()
            .map(|scene| {
                self.scene_entities(scene)
                    .map(|roles| roles.len())
                    .unwrap_or(scene.instances.len())
            })
            .max()
            .unwrap_or(0)
    }

    // These addresses describe only MAIN_SOURCE's isolated harness, never custom code.
    pub fn pickups_address(&self) -> Option<usize> {
        (self.entity_slots() > 0).then_some(0x5c9 + usize::from(self.animation.is_some()) * 5)
    }

    pub fn generated_ram_bytes(&self) -> usize {
        self.entity_ram_bytes()
            + self.event_ram_bytes()
            + self.combat_ram_bytes()
            + self.spawner_ram_bytes()
            + self.visual_ram_bytes()
            + self.attack_ram_bytes()
    }

    pub(super) fn entity_ram_bytes(&self) -> usize {
        9 + usize::from(self.animation.is_some()) * 5
            + if self.entity_slots() > 0 {
                3 + self.entity_slots() * 9
            } else {
                0
            }
    }

    pub fn scene_sprites(&self, scene: &Scene, assets: &ProjectAssets) -> Result<usize, String> {
        let mut count = self
            .animation
            .as_ref()
            .map(|name| animation_sprites(name, assets))
            .transpose()?
            .unwrap_or(0);
        for role in self.scene_entities(scene)? {
            if role.definition.behavior != EntityBehavior::Spawner {
                count += animation_sprites(&role.definition.animation, assets)?;
            }
        }
        Ok(count)
    }

    pub fn validate(&self, assets: &ProjectAssets) -> Result<(), String> {
        if self.version != 1 {
            return Err("Unsupported game design version; expected 1".into());
        }
        if self.title.trim().is_empty() || self.title.len() > 120 || self.core_loop.len() > 4000 {
            return Err("Game title needs 1..120 bytes and design brief at most 4000 bytes".into());
        }
        if self.scenes.is_empty() || self.scenes.len() > 255 {
            return Err("Game flow needs 1..255 scenes".into());
        }
        let world = assets
            .world
            .as_ref()
            .ok_or("Place maps in the World editor before assigning scenes")?;
        let mut ids = HashSet::new();
        for scene in &self.scenes {
            if scene.id.is_empty() || scene.id.len() > 64 || !ids.insert(scene.id.as_str()) {
                return Err("Scene IDs must be unique and contain 1..64 bytes".into());
            }
            // Display names are emitted inside BASIC string literals and fit one screen row.
            if scene.name.is_empty()
                || scene.name.len() > 28
                || !scene
                    .name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || " -_!? .".contains(c))
            {
                return Err("Scene names need 1..28 letters, digits, spaces or -_!?.".into());
            }
            if scene.spawn_x > 240 || scene.spawn_y > 224 {
                return Err(format!(
                    "Scene '{}' spawn must fit a 16x16 player inside 256x240",
                    scene.name
                ));
            }
            let cell =
                usize::from(scene.map_row) * world.width as usize + usize::from(scene.map_col);
            if u32::from(scene.map_col) >= world.width
                || u32::from(scene.map_row) >= world.height
                || !world
                    .data
                    .get(cell)
                    .is_some_and(|index| *index >= 0 && (*index as usize) < assets.nametables.len())
            {
                return Err(format!(
                    "Scene '{}' refers to an empty or missing world cell",
                    scene.name
                ));
            }
            if scene
                .music
                .is_some_and(|id| usize::from(id) >= assets.audio_tracks.len())
            {
                return Err(format!("Scene '{}' refers to missing music", scene.name));
            }
            if scene
                .sfx
                .is_some_and(|id| usize::from(id) >= assets.sound_effects.len())
            {
                return Err(format!(
                    "Scene '{}' refers to a missing sound effect",
                    scene.name
                ));
            }
            if scene.transitions.len() > 16 {
                return Err(format!(
                    "Scene '{}' supports at most 16 transitions",
                    scene.name
                ));
            }
            let mut triggers = HashSet::new();
            for edge in &scene.transitions {
                if !triggers.insert(edge.trigger)
                    || (edge.trigger == Trigger::Timer && edge.frames == 0)
                {
                    return Err(format!(
                        "Scene '{}' needs unique triggers and timers of 1..65535 frames",
                        scene.name
                    ));
                }
            }
        }
        if !ids.contains(self.start_scene.as_str()) {
            return Err("Choose an existing start scene".into());
        }
        if self
            .scenes
            .iter()
            .flat_map(|scene| &scene.transitions)
            .any(|edge| !ids.contains(edge.target.as_str()))
        {
            return Err("A transition refers to a missing scene".into());
        }
        if self.entities.len() > 32 {
            return Err("A project supports at most 32 reusable entity definitions".into());
        }
        let mut entity_ids = HashSet::new();
        for entity in &self.entities {
            if entity.id.is_empty()
                || entity.id.len() > 64
                || !entity_ids.insert(&entity.id)
                || entity.name.trim().is_empty()
                || entity.name.len() > 120
            {
                return Err("Entities need unique 1..64-byte IDs and 1..120-byte names".into());
            }
            if entity.behavior != EntityBehavior::Spawner {
                animation_sprites(&entity.animation, assets)?;
            }
        }
        let mut instance_ids = HashSet::new();
        for scene in &self.scenes {
            if scene.instances.len() > 16 {
                return Err(format!(
                    "Scene '{}' supports at most 16 entity instances",
                    scene.name
                ));
            }
            for instance in &scene.instances {
                if instance.id.is_empty()
                    || instance.id.len() > 64
                    || !instance_ids.insert(&instance.id)
                {
                    return Err(
                        "Instance IDs must be unique across scenes and contain 1..64 bytes".into(),
                    );
                }
                if instance.x > 240 || instance.y > 224 {
                    return Err("Entity positions must fit a 16x16 object inside 256x240".into());
                }
                let entity = self
                    .entities
                    .iter()
                    .find(|entity| entity.id == instance.entity)
                    .ok_or("An instance refers to a missing entity definition")?;
                if entity.behavior == EntityBehavior::Door
                    && !instance
                        .target_scene
                        .as_ref()
                        .is_some_and(|target| ids.contains(target.as_str()))
                {
                    return Err(format!(
                        "Door '{}' needs an existing destination scene",
                        entity.name
                    ));
                }
                if (entity.behavior == EntityBehavior::Patrol
                    || (matches!(
                        entity.behavior,
                        EntityBehavior::Enemy | EntityBehavior::Spawner
                    ) && instance.patrol_end_x.is_some()))
                    && !instance
                        .patrol_end_x
                        .is_some_and(|end| end > instance.x && end <= 240)
                {
                    return Err(format!(
                        "Patrol '{}' needs a right endpoint greater than X and at most 240",
                        entity.name
                    ));
                }
            }
            let sprites = self.scene_sprites(scene, assets)?;
            if sprites > 64 {
                return Err(format!(
                    "Scene '{}' needs {sprites} sprites; NES OAM holds 64",
                    scene.name
                ));
            }
        }
        if let Some(name) = &self.animation {
            if name.is_empty()
                || !name.as_bytes()[0].is_ascii_alphabetic()
                || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                || !assets
                    .animations
                    .iter()
                    .any(|animation| animation.name == *name)
            {
                return Err("Choose an existing animation with a valid source identifier".into());
            }
        }
        self.validate_spawners()?;
        self.validate_combat()?;
        self.player_visual.validate()?;
        for entity in &self.entities {
            entity.visual.validate()?;
        }
        self.validate_events(assets)?;
        if self.tests.len() > 32 {
            return Err("A project supports at most 32 saved flow tests".into());
        }
        let mut test_ids = HashSet::new();
        for test in &self.tests {
            if test.id.is_empty()
                || test.id.len() > 64
                || !test_ids.insert(&test.id)
                || test.name.trim().is_empty()
                || test.name.len() > 120
            {
                return Err("Flow tests need unique 1..64-byte IDs and 1..120-byte names".into());
            }
            if test.steps.is_empty() || test.steps.len() > 64 {
                return Err(format!("Test '{}' needs 1..64 steps", test.name));
            }
            let mut frames = u32::from(TEST_BOOT_FRAMES);
            for step in &test.steps {
                if step.expected_variables.len() > 32
                    || step.expected_spawns.len() > 16
                    || step
                        .expected_spawns
                        .keys()
                        .any(|id| id.is_empty() || id.len() > 64)
                    || step
                        .expected_variables
                        .keys()
                        .any(|id| id.is_empty() || id.len() > 64)
                {
                    return Err(format!(
                        "Test '{}' has invalid variable expectations",
                        test.name
                    ));
                }
                frames += u32::from(step.frames);
                let mut buttons = HashSet::new();
                if step.frames == 0
                    || frames > 600
                    || step
                        .buttons
                        .iter()
                        .any(|button| button.button_index().is_none() || !buttons.insert(*button))
                {
                    return Err(format!("Test '{}' needs positive step durations, unique controller buttons and at most 600 total frames (including boot)",test.name));
                }
                // Keep missing scene expectations: removing a scene must not silently remove an assertion.
                if step
                    .expected_scene
                    .as_ref()
                    .is_some_and(|id| id.is_empty() || id.len() > 64)
                {
                    return Err(format!(
                        "Test '{}' has an invalid scene expectation",
                        test.name
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn source(&self, assets: &ProjectAssets) -> Result<String, String> {
        self.validate(assets)?;
        let indices: HashMap<_, _> = self
            .scenes
            .iter()
            .enumerate()
            .map(|(index, scene)| (scene.id.as_str(), index))
            .collect();
        let mut source = String::from("' Generated from Game Studio. Edit the design and entities, not this virtual file.\nDIM gf_scene AS BYTE\nDIM gf_ticks AS WORD\nDIM gf_x AS WORD\nDIM gf_y AS WORD\nDIM gf_movable AS BYTE\nDIM gf_loaded AS BYTE\n");
        if self.animation.is_some() {
            source.push_str("DIM gf_anim AS AnimState\n");
        }
        if self.entity_slots() > 0 {
            source.push_str("DIM gf_pickups AS WORD\nDIM gf_changed AS BYTE\n");
            // Slots are reused between rooms, bounding RAM by the largest room, not the whole game.
            for slot in 0..self.entity_slots() {
                writeln!(source,"DIM gf_e{slot}_active AS BYTE\nDIM gf_e{slot}_x AS WORD\nDIM gf_e{slot}_dir AS BYTE\nDIM gf_e{slot}_anim AS AnimState").unwrap();
            }
        }
        self.event_declarations(&mut source);
        self.combat_declarations(&mut source);
        self.spawner_declarations(&mut source);
        if self.visual_ram_bytes() > 0 {
            source.push_str("DIM gf_player_facing AS BYTE\n");
        }
        self.attack_declarations(&mut source);
        source.push_str("\nSUB GameFlowStart()\n");
        if self.attack_ram_bytes() > 0 {
            source.push_str("    gf_attack_cooldown = 0\n    gf_attacking = 0\n");
        }
        if let Some(combat) = &self.combat {
            writeln!(
                source,
                "    gf_health = {}\n    gf_invulnerable = 0\n    gf_defeated = 0",
                combat.health
            )
            .unwrap();
        }
        for variable in &self.variables {
            writeln!(source, "    gv_{} = {}", variable.name, variable.initial).unwrap();
        }
        if self.entity_slots() > 0 {
            source.push_str("    gf_pickups = 0\n");
        }
        writeln!(
            source,
            "    gf_scene = {}\n    GameFlowEnter()\nEND SUB",
            indices[self.start_scene.as_str()]
        )
        .unwrap();
        source.push_str("\nSUB GameFlowEnter()\n    PPU.Ctrl($10)\n    PPU.Mask(0)\n    Sound.Stop()\n    gf_ticks = 0\n");
        if self.attack_ram_bytes() > 0 {
            source.push_str("    gf_attacking = 0\n");
        }
        if self.visual_ram_bytes() > 0 {
            source.push_str("    gf_player_facing = 0\n");
        }
        if self.entity_slots() > 0 || !self.events.is_empty() {
            source.push_str("    gf_changed = 255\n");
        }
        if !self.events.is_empty() {
            source.push_str("    gf_event_entry = 255\n");
            for index in 0..self.events.len() {
                writeln!(source, "    gf_event{index} = 0").unwrap();
            }
        }
        if self.entity_slots() > 0 {
            for slot in 0..self.entity_slots() {
                writeln!(source, "    gf_e{slot}_active = 0").unwrap();
            }
        }
        source.push_str("    SELECT CASE gf_scene\n");
        for (index, scene) in self.scenes.iter().enumerate() {
            writeln!(source, "    CASE {index}\n        gf_loaded = World.Load({}, {})\n        gf_x = {}\n        gf_y = {}\n        gf_movable = {}\n        Text.Print(1, 1, \"{}\")", scene.map_col, scene.map_row, scene.spawn_x, scene.spawn_y, if scene.movement {255} else {0}, scene.name.to_ascii_uppercase()).unwrap();
            if let Some(id) = scene.music {
                writeln!(source, "        Sound.Play({id})").unwrap();
            }
            if let Some(id) = scene.sfx {
                writeln!(source, "        PLAY_SFX({id})").unwrap();
            }
            for role in self.scene_entities(scene).unwrap() {
                if role.spawned_by.is_some() || role.definition.behavior == EntityBehavior::Spawner
                {
                    continue;
                }
                let slot = role.slot;
                let instance = role.instance;
                let entity = role.definition;
                writeln!(source,"        gf_e{slot}_active = 255\n        gf_e{slot}_x = {}\n        gf_e{slot}_dir = 0\n        Animation.Play(gf_e{slot}_anim, {})",instance.x,entity.animation).unwrap();
            }
        }
        source.push_str("    END SELECT\n");
        if self.combat.is_some() {
            // Room entry never heals the player; restart explicitly restores initial combat state.
            source.push_str("    IF gf_health = 0 THEN\n        gf_movable = 0\n    END IF\n");
            for slot in 0..self.entity_slots() {
                writeln!(source, "    gf_e{slot}_health = 0").unwrap();
            }
            source.push_str("    SELECT CASE gf_scene\n");
            for (index, scene) in self.scenes.iter().enumerate() {
                writeln!(source, "    CASE {index}").unwrap();
                for role in self.scene_entities(scene).unwrap() {
                    let slot = role.slot;
                    let entity = role.definition;
                    if entity.behavior == EntityBehavior::Enemy && role.spawned_by.is_none() {
                        writeln!(
                            source,
                            "        gf_e{slot}_health = {}",
                            entity.enemy.health
                        )
                        .unwrap();
                    }
                }
            }
            source.push_str("    END SELECT\n");
        }
        self.spawner_entry(&mut source);
        if let Some(name) = &self.animation {
            writeln!(source, "    Animation.Play(gf_anim, {name})").unwrap();
        }
        source.push_str("    PPU.Mask($1E)\n    PPU.Ctrl($90)\nEND SUB\n\nSUB GameFlowUpdate()\n    Controller.Read()\n");
        self.attack_tick(&mut source);
        if self.combat.is_some() {
            source.push_str("    IF gf_invulnerable > 0 THEN\n        gf_invulnerable = gf_invulnerable - 1\n    END IF\n");
        }
        if self.entity_slots() > 0 || !self.events.is_empty() {
            source.push_str("    gf_changed = 0\n");
        }
        source.push_str(
            "    IF gf_ticks < 65535 THEN\n        gf_ticks = gf_ticks + 1\n    END IF\n",
        );
        if self
            .scenes
            .iter()
            .any(|scene| !scene.transitions.is_empty())
        {
            source.push_str("    SELECT CASE gf_scene\n");
            for (index, scene) in self
                .scenes
                .iter()
                .enumerate()
                .filter(|(_, scene)| !scene.transitions.is_empty())
            {
                writeln!(source, "    CASE {index}").unwrap();
                for (edge_index, edge) in scene.transitions.iter().enumerate() {
                    let condition = match edge.trigger {
                        Trigger::Timer => format!("gf_ticks >= {}", edge.frames),
                        button => format!("Controller.IsPressed(Button.{button:?})"),
                    };
                    // Nested IF/ELSE preserves first-match priority in the existing grammar.
                    if edge_index != 0 {
                        source.push_str("        ELSE\n");
                    }
                    writeln!(source, "        IF {condition} THEN\n            gf_scene = {}\n            GameFlowEnter()", indices[edge.target.as_str()]).unwrap();
                }
                for _ in &scene.transitions {
                    source.push_str("        END IF\n");
                }
            }
            source.push_str("    END SELECT\n");
        }
        if self.scenes.iter().any(|scene| scene.movement) {
            source.push_str("    IF gf_movable THEN\n        GameFlowMove()\n    END IF\n");
        }
        self.attack_update(&mut source);
        if self.entity_slots() > 0 {
            source.push_str("    GameFlowEntities()\n");
        }
        if self.spawner_slots() > 0 {
            source.push_str("    GameFlowSpawners()\n");
        }
        if !self.events.is_empty() {
            source.push_str("    GameFlowEvents()\n");
        }
        if self.animation.is_some() || self.entity_slots() > 0 {
            source.push_str("    Sprite.Clear()\n");
        }
        if self.animation.is_some() {
            if self.combat.is_some() {
                source.push_str("    IF gf_health > 0 THEN\n");
            }
            writeln!(
                source,
                "    Animation.Update(gf_anim)\n    {}",
                self.player_visual
                    .draw("gf_x", "gf_y", "gf_anim", "gf_player_facing")
            )
            .unwrap();
            if self.combat.is_some() {
                source.push_str("    END IF\n");
            }
        }
        if self.entity_slots() > 0 {
            source.push_str("    GameFlowDrawEntities()\n");
        }
        source.push_str("    Scroll.Set(0, 0)\nEND SUB\n");
        if self.scenes.iter().any(|scene| scene.movement) {
            source.push_str("\nSUB GameFlowMove()\n");
            if self.visual_ram_bytes() > 0 {
                source.push_str("    IF Controller.IsHeld(Button.Left) THEN\n        gf_player_facing = 255\n    END IF\n    IF Controller.IsHeld(Button.Right) THEN\n        gf_player_facing = 0\n    END IF\n");
            }
            let p = self.player_hitbox;
            let right = u16::from(p.x) + u16::from(p.width);
            let bottom = u16::from(p.y) + u16::from(p.height);
            let middle_x = u16::from(p.x) + u16::from((p.width - 1) / 2);
            let middle_y = u16::from(p.y) + u16::from((p.height - 1) / 2);
            // A 16px edge at an unaligned origin may span three tiles, including an interior tile.
            for (button, coordinate, guard, x1, y1, x2, y2, x3, y3, operation) in [
                (
                    "Left",
                    "gf_x",
                    "gf_x > 0",
                    format!("gf_x + {} - 1", p.x),
                    format!("gf_y + {}", p.y),
                    format!("gf_x + {} - 1", p.x),
                    format!("gf_y + {}", bottom - 1),
                    format!("gf_x + {} - 1", p.x),
                    format!("gf_y + {middle_y}"),
                    "-",
                ),
                (
                    "Right",
                    "gf_x",
                    "gf_x < 240",
                    format!("gf_x + {right}"),
                    format!("gf_y + {}", p.y),
                    format!("gf_x + {right}"),
                    format!("gf_y + {}", bottom - 1),
                    format!("gf_x + {right}"),
                    format!("gf_y + {middle_y}"),
                    "+",
                ),
                (
                    "Up",
                    "gf_y",
                    "gf_y > 0",
                    format!("gf_x + {}", p.x),
                    format!("gf_y + {} - 1", p.y),
                    format!("gf_x + {}", right - 1),
                    format!("gf_y + {} - 1", p.y),
                    format!("gf_x + {middle_x}"),
                    format!("gf_y + {} - 1", p.y),
                    "-",
                ),
                (
                    "Down",
                    "gf_y",
                    "gf_y < 224",
                    format!("gf_x + {}", p.x),
                    format!("gf_y + {bottom}"),
                    format!("gf_x + {}", right - 1),
                    format!("gf_y + {bottom}"),
                    format!("gf_x + {middle_x}"),
                    format!("gf_y + {bottom}"),
                    "+",
                ),
            ] {
                writeln!(source, "    IF Controller.IsHeld(Button.{button}) AND {guard} THEN\n        IF Collision.Tile({x1}, {y1}) = 0 AND Collision.Tile({x2}, {y2}) = 0 AND Collision.Tile({x3}, {y3}) = 0 THEN\n            {coordinate} = {coordinate} {operation} 1\n        END IF\n    END IF").unwrap();
            }
            source.push_str("END SUB\n");
        }
        if self.entity_slots() > 0 {
            self.entity_source(&mut source, &indices);
        }
        self.event_source(&mut source);
        self.spawner_source(&mut source);
        self.spawn_enemy_templates(&mut source, &indices);
        Ok(source)
    }

    fn entity_source(&self, source: &mut String, indices: &HashMap<&str, usize>) {
        source.push_str("\nSUB GameFlowEntities()\n    SELECT CASE gf_scene\n");
        for (index, scene) in self
            .scenes
            .iter()
            .enumerate()
            .filter(|(_, scene)| !scene.instances.is_empty())
        {
            writeln!(source, "    CASE {index}").unwrap();
            for role in self.scene_entities(scene).unwrap() {
                if role.definition.behavior == EntityBehavior::Spawner {
                    continue;
                }
                let slot = role.slot;
                let instance = role.instance;
                let entity = role.definition;
                writeln!(
                    source,
                    "        IF gf_changed = 0 AND gf_e{slot}_active THEN"
                )
                .unwrap();
                if role.spawned_by.is_some() {
                    // A worker may enter another room on death. Never restore old slot state afterward.
                    self.spawn_enemy_call(source, scene, &role, index);
                    source.push_str("        END IF\n");
                    continue;
                }
                if entity.behavior == EntityBehavior::Patrol
                    || (entity.behavior == EntityBehavior::Enemy && instance.patrol_end_x.is_some())
                {
                    self.enemy_movement_source(source, entity, instance, slot);
                }
                if matches!(
                    entity.behavior,
                    EntityBehavior::Pickup | EntityBehavior::Door
                ) {
                    let p = self.player_hitbox;
                    let e = entity.hitbox;
                    writeln!(source,"            IF gf_movable AND Collision.Rect(gf_x + {}, gf_y + {}, {}, {}, gf_e{slot}_x + {}, {}, {}, {}) THEN",p.x,p.y,p.width,p.height,e.x,u16::from(instance.y)+u16::from(e.y),e.width,e.height).unwrap();
                    if entity.behavior == EntityBehavior::Pickup {
                        writeln!(source,"                gf_e{slot}_active = 0\n                IF gf_pickups < 65535 THEN\n                    gf_pickups = gf_pickups + 1\n                END IF").unwrap();
                    } else {
                        writeln!(source,"                IF Controller.IsPressed(Button.A) THEN\n                    gf_scene = {}\n                    GameFlowEnter()\n                END IF",indices[instance.target_scene.as_deref().unwrap()]).unwrap();
                    }
                    source.push_str("            END IF\n");
                }
                if entity.behavior == EntityBehavior::Enemy {
                    self.enemy_source(source, entity, slot, instance.y, indices);
                }
                source.push_str("        END IF\n");
            }
        }
        source.push_str(
            "    END SELECT\nEND SUB\n\nSUB GameFlowDrawEntities()\n    SELECT CASE gf_scene\n",
        );
        for (index, scene) in self
            .scenes
            .iter()
            .enumerate()
            .filter(|(_, scene)| !scene.instances.is_empty())
        {
            writeln!(source, "    CASE {index}").unwrap();
            for role in self.scene_entities(scene).unwrap() {
                if role.definition.behavior == EntityBehavior::Spawner {
                    continue;
                }
                let slot = role.slot;
                let instance = role.instance;
                writeln!(source,"        IF gf_e{slot}_active THEN\n            Animation.Update(gf_e{slot}_anim)\n            {}\n        END IF",role.definition.visual.draw(&format!("gf_e{slot}_x"),&instance.y.to_string(),&format!("gf_e{slot}_anim"),&format!("gf_e{slot}_dir"))).unwrap();
            }
        }
        source.push_str("    END SELECT\nEND SUB\n");
    }

    pub(super) fn enemy_movement_source(
        &self,
        source: &mut String,
        entity: &EntityDefinition,
        instance: &EntityInstance,
        slot: usize,
    ) {
        if entity.behavior != EntityBehavior::Enemy || entity.enemy.motion == EnemyMotion::Patrol {
            self.patrol_source(source, entity, instance, slot);
            return;
        }
        let end = instance.patrol_end_x.unwrap();
        let e = entity.hitbox;
        let top = u16::from(instance.y) + u16::from(e.y);
        let bottom = top + u16::from(e.height) - 1;
        let middle = top + u16::from((e.height - 1) / 2);
        let right = u16::from(e.x) + u16::from(e.width);
        let range = entity.enemy.detection_range;
        let y = instance.y;
        // Compare added endpoints so detection at X/Y=0 never underflows WORD arithmetic.
        // Pursuit shares slot direction/facing, needs no extra RAM, and stops outside range.
        writeln!(source, "            IF gf_movable AND (gf_health > 0) AND (gf_x <= gf_e{slot}_x + {range}) AND (gf_e{slot}_x <= gf_x + {range}) AND (gf_y <= {y} + {range}) AND ({y} <= gf_y + {range}) THEN\n                IF gf_x < gf_e{slot}_x THEN\n                    gf_e{slot}_dir = 255\n                    IF gf_e{slot}_x > {} THEN\n                        IF Collision.Tile(gf_e{slot}_x + {} - 1, {top}) = 0 AND Collision.Tile(gf_e{slot}_x + {} - 1, {bottom}) = 0 AND Collision.Tile(gf_e{slot}_x + {} - 1, {middle}) = 0 THEN\n                            gf_e{slot}_x = gf_e{slot}_x - 1\n                        END IF\n                    END IF\n                ELSE\n                    IF gf_x > gf_e{slot}_x THEN\n                        gf_e{slot}_dir = 0\n                        IF gf_e{slot}_x < {end} THEN\n                            IF Collision.Tile(gf_e{slot}_x + {right}, {top}) = 0 AND Collision.Tile(gf_e{slot}_x + {right}, {bottom}) = 0 AND Collision.Tile(gf_e{slot}_x + {right}, {middle}) = 0 THEN\n                                gf_e{slot}_x = gf_e{slot}_x + 1\n                            END IF\n                        END IF\n                    END IF\n                END IF\n            END IF", instance.x, e.x, e.x, e.x).unwrap();
    }

    fn patrol_source(
        &self,
        source: &mut String,
        entity: &EntityDefinition,
        instance: &EntityInstance,
        slot: usize,
    ) {
        let end = instance.patrol_end_x.unwrap();
        let e = entity.hitbox;
        let top = u16::from(instance.y) + u16::from(e.y);
        let bottom = top + u16::from(e.height) - 1;
        let middle = top + u16::from((e.height - 1) / 2);
        let right = u16::from(e.x) + u16::from(e.width);
        writeln!(source,"            IF gf_e{slot}_dir = 0 THEN\n                IF gf_e{slot}_x >= {end} THEN\n                    gf_e{slot}_dir = 255\n                ELSE\n                    IF Collision.Tile(gf_e{slot}_x + {right}, {top}) = 0 AND Collision.Tile(gf_e{slot}_x + {right}, {bottom}) = 0 AND Collision.Tile(gf_e{slot}_x + {right}, {middle}) = 0 THEN\n                        gf_e{slot}_x = gf_e{slot}_x + 1\n                    ELSE\n                        gf_e{slot}_dir = 255\n                    END IF\n                END IF\n            ELSE\n                IF gf_e{slot}_x <= {} THEN\n                    gf_e{slot}_dir = 0\n                ELSE\n                    IF Collision.Tile(gf_e{slot}_x + {} - 1, {top}) = 0 AND Collision.Tile(gf_e{slot}_x + {} - 1, {bottom}) = 0 AND Collision.Tile(gf_e{slot}_x + {} - 1, {middle}) = 0 THEN\n                        gf_e{slot}_x = gf_e{slot}_x - 1\n                    ELSE\n                        gf_e{slot}_dir = 0\n                    END IF\n                END IF\n            END IF",instance.x,e.x,e.x,e.x).unwrap();
    }
}

fn animation_sprites(name: &str, assets: &ProjectAssets) -> Result<usize, String> {
    if name.is_empty()
        || !name.as_bytes()[0].is_ascii_alphabetic()
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err("Choose an animation with a valid source identifier".into());
    }
    let animation = assets
        .animations
        .iter()
        .find(|animation| animation.name == name)
        .ok_or_else(|| format!("Missing animation: {name}"))?;
    if animation.frames.is_empty() {
        return Err(format!("Animation '{name}' needs frames"));
    }
    animation
        .frames
        .iter()
        .map(|frame| {
            assets
                .metasprites
                .iter()
                .find(|sprite| sprite.name == frame.metasprite)
                .map(|sprite| sprite.tiles.len())
                .ok_or_else(|| format!("Animation '{name}' refers to a missing metasprite"))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|counts| counts.into_iter().max().unwrap_or(0))
}

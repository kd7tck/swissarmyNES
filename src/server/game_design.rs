//! Scene authoring produces ordinary source through the same compiler as hand-written games.
use super::game_events::{GameEvent, GameVariable};
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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub expected_variables: BTreeMap<String, u16>,
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
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EntityBehavior {
    Decoration,
    Pickup,
    Door,
    Patrol,
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
            .map(|scene| scene.instances.len())
            .max()
            .unwrap_or(0)
    }

    // These addresses describe only MAIN_SOURCE's isolated harness, never custom code.
    pub fn pickups_address(&self) -> Option<usize> {
        (self.entity_slots() > 0).then_some(0x5c9 + usize::from(self.animation.is_some()) * 5)
    }

    pub fn generated_ram_bytes(&self) -> usize {
        self.entity_ram_bytes() + self.event_ram_bytes()
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
        for instance in &scene.instances {
            let definition = self
                .entities
                .iter()
                .find(|entity| entity.id == instance.entity)
                .ok_or("An instance refers to a missing entity definition")?;
            count += animation_sprites(&definition.animation, assets)?;
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
            animation_sprites(&entity.animation, assets)?;
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
                if entity.behavior == EntityBehavior::Patrol
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
        source.push_str("\nSUB GameFlowStart()\n");
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
            for (slot, instance) in scene.instances.iter().enumerate() {
                let entity = self
                    .entities
                    .iter()
                    .find(|entity| entity.id == instance.entity)
                    .unwrap();
                writeln!(source,"        gf_e{slot}_active = 255\n        gf_e{slot}_x = {}\n        gf_e{slot}_dir = 0\n        Animation.Play(gf_e{slot}_anim, {})",instance.x,entity.animation).unwrap();
            }
        }
        source.push_str("    END SELECT\n");
        if let Some(name) = &self.animation {
            writeln!(source, "    Animation.Play(gf_anim, {name})").unwrap();
        }
        source.push_str("    PPU.Mask($1E)\n    PPU.Ctrl($90)\nEND SUB\n\nSUB GameFlowUpdate()\n    Controller.Read()\n");
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
        if self.entity_slots() > 0 {
            source.push_str("    GameFlowEntities()\n");
        }
        if !self.events.is_empty() {
            source.push_str("    GameFlowEvents()\n");
        }
        if self.animation.is_some() || self.entity_slots() > 0 {
            source.push_str("    Sprite.Clear()\n");
        }
        if self.animation.is_some() {
            source.push_str(
                "    Animation.Update(gf_anim)\n    Animation.Draw(gf_x, gf_y, gf_anim)\n",
            );
        }
        if self.entity_slots() > 0 {
            source.push_str("    GameFlowDrawEntities()\n");
        }
        source.push_str("    Scroll.Set(0, 0)\nEND SUB\n");
        if self.scenes.iter().any(|scene| scene.movement) {
            source.push_str("\nSUB GameFlowMove()\n");
            for (button, coordinate, guard, x1, y1, x2, y2, operation) in [
                (
                    "Left",
                    "gf_x",
                    "gf_x > 0",
                    "gf_x - 1",
                    "gf_y",
                    "gf_x - 1",
                    "gf_y + 15",
                    "-",
                ),
                (
                    "Right",
                    "gf_x",
                    "gf_x < 240",
                    "gf_x + 16",
                    "gf_y",
                    "gf_x + 16",
                    "gf_y + 15",
                    "+",
                ),
                (
                    "Up",
                    "gf_y",
                    "gf_y > 0",
                    "gf_x",
                    "gf_y - 1",
                    "gf_x + 15",
                    "gf_y - 1",
                    "-",
                ),
                (
                    "Down",
                    "gf_y",
                    "gf_y < 224",
                    "gf_x",
                    "gf_y + 16",
                    "gf_x + 15",
                    "gf_y + 16",
                    "+",
                ),
            ] {
                writeln!(source, "    IF Controller.IsHeld(Button.{button}) AND {guard} THEN\n        IF Collision.Tile({x1}, {y1}) = 0 AND Collision.Tile({x2}, {y2}) = 0 THEN\n            {coordinate} = {coordinate} {operation} 1\n        END IF\n    END IF").unwrap();
            }
            source.push_str("END SUB\n");
        }
        if self.entity_slots() > 0 {
            self.entity_source(&mut source, &indices);
        }
        self.event_source(&mut source);
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
            for (slot, instance) in scene.instances.iter().enumerate() {
                let entity = self
                    .entities
                    .iter()
                    .find(|entity| entity.id == instance.entity)
                    .unwrap();
                writeln!(
                    source,
                    "        IF gf_changed = 0 AND gf_e{slot}_active THEN"
                )
                .unwrap();
                if entity.behavior == EntityBehavior::Patrol {
                    let end = instance.patrol_end_x.unwrap();
                    writeln!(source,"            IF gf_e{slot}_dir = 0 THEN\n                IF gf_e{slot}_x >= {end} THEN\n                    gf_e{slot}_dir = 255\n                ELSE\n                    IF Collision.Tile(gf_e{slot}_x + 16, {}) = 0 AND Collision.Tile(gf_e{slot}_x + 16, {}) = 0 THEN\n                        gf_e{slot}_x = gf_e{slot}_x + 1\n                    ELSE\n                        gf_e{slot}_dir = 255\n                    END IF\n                END IF\n            ELSE\n                IF gf_e{slot}_x <= {} THEN\n                    gf_e{slot}_dir = 0\n                ELSE\n                    IF Collision.Tile(gf_e{slot}_x - 1, {}) = 0 AND Collision.Tile(gf_e{slot}_x - 1, {}) = 0 THEN\n                        gf_e{slot}_x = gf_e{slot}_x - 1\n                    ELSE\n                        gf_e{slot}_dir = 0\n                    END IF\n                END IF\n            END IF",instance.y,u16::from(instance.y)+15,instance.x,instance.y,u16::from(instance.y)+15).unwrap();
                }
                if matches!(
                    entity.behavior,
                    EntityBehavior::Pickup | EntityBehavior::Door
                ) {
                    writeln!(source,"            IF gf_movable AND Collision.Rect(gf_x, gf_y, 16, 16, gf_e{slot}_x, {}, 16, 16) THEN",instance.y).unwrap();
                    if entity.behavior == EntityBehavior::Pickup {
                        writeln!(source,"                gf_e{slot}_active = 0\n                IF gf_pickups < 65535 THEN\n                    gf_pickups = gf_pickups + 1\n                END IF").unwrap();
                    } else {
                        writeln!(source,"                IF Controller.IsPressed(Button.A) THEN\n                    gf_scene = {}\n                    GameFlowEnter()\n                END IF",indices[instance.target_scene.as_deref().unwrap()]).unwrap();
                    }
                    source.push_str("            END IF\n");
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
            for (slot, instance) in scene.instances.iter().enumerate() {
                writeln!(source,"        IF gf_e{slot}_active THEN\n            Animation.Update(gf_e{slot}_anim)\n            Animation.Draw(gf_e{slot}_x, {}, gf_e{slot}_anim)\n        END IF",instance.y).unwrap();
            }
        }
        source.push_str("    END SELECT\nEND SUB\n");
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

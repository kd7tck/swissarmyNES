//! Bounded visual rules compile to ordinary SwissBASIC, with no second runtime.
use super::{game_design::GameDesign, game_design::Trigger, project::ProjectAssets};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fmt::Write};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameVariable {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub initial: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameEvent {
    pub id: String,
    pub name: String,
    pub scene: String,
    pub trigger: EventTrigger,
    #[serde(default)]
    pub once_per_visit: bool,
    #[serde(default)]
    pub conditions: Vec<EventCondition>,
    pub actions: Vec<EventAction>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventTrigger {
    Enter,
    Update,
    Pressed { button: Trigger },
    Held { button: Trigger },
    Timer { frames: u16 },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EventCondition {
    pub source: ConditionSource,
    pub comparison: Comparison,
    pub value: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConditionSource {
    Variable { variable: String },
    Pickups,
    X,
    Y,
    Ticks,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Comparison {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventAction {
    Set { variable: String, value: u16 },
    Add { variable: String, value: u16 },
    Subtract { variable: String, value: u16 },
    Scene { scene: String },
    Music { track: u8 },
    Sfx { effect: u8 },
    StopMusic,
}

impl GameDesign {
    pub fn variable_address(&self, id: &str) -> Option<usize> {
        // The isolated test harness preserves all earlier entity addresses.
        self.variables
            .iter()
            .position(|variable| variable.id == id)
            .map(|index| 0x5c0 + self.entity_ram_bytes() + index * 2)
    }

    pub fn event_ram_bytes(&self) -> usize {
        self.variables.len() * 2
            + if self.events.is_empty() {
                0
            } else {
                1 + usize::from(self.entity_slots() == 0) + self.events.len()
            }
    }

    pub(super) fn validate_events(&self, assets: &ProjectAssets) -> Result<(), String> {
        if self.variables.len() > 32 || self.events.len() > 32 {
            return Err("A project supports at most 32 variables and 32 events".into());
        }
        let mut variable_ids = HashSet::new();
        let mut names = HashSet::new();
        for variable in &self.variables {
            if !valid_id(&variable.id)
                || !variable_ids.insert(variable.id.as_str())
                || variable.name.is_empty()
                || variable.name.len() > 32
                || !variable.name.as_bytes()[0].is_ascii_alphabetic()
                || !variable
                    .name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
                || !names.insert(variable.name.to_ascii_lowercase())
            {
                return Err("Variables need unique 1..64-byte IDs and unique 1..32-character source names (letters, digits, underscores; start with a letter)".into());
            }
        }
        let scenes: HashSet<_> = self.scenes.iter().map(|scene| scene.id.as_str()).collect();
        let mut ids = HashSet::new();
        for event in &self.events {
            if !valid_id(&event.id)
                || !ids.insert(event.id.as_str())
                || event.name.trim().is_empty()
                || event.name.len() > 120
            {
                return Err("Events need unique 1..64-byte IDs and 1..120-byte names".into());
            }
            if !scenes.contains(event.scene.as_str()) {
                return Err(format!("Event '{}' refers to a missing scene", event.name));
            }
            match event.trigger {
                EventTrigger::Pressed { button } | EventTrigger::Held { button }
                    if button.button_index().is_none() =>
                {
                    return Err(format!("Event '{}' needs a controller button", event.name));
                }
                EventTrigger::Timer { frames: 0 } => {
                    return Err(format!(
                        "Event '{}' timer needs 1..65535 frames",
                        event.name
                    ))
                }
                _ => {}
            }
            if event.conditions.len() > 8 || event.actions.is_empty() || event.actions.len() > 8 {
                return Err(format!(
                    "Event '{}' needs at most 8 conditions and 1..8 actions",
                    event.name
                ));
            }
            for condition in &event.conditions {
                if let ConditionSource::Variable { variable } = &condition.source {
                    if !variable_ids.contains(variable.as_str()) {
                        return Err(format!(
                            "Event '{}' condition refers to a missing variable",
                            event.name
                        ));
                    }
                }
            }
            for (index, action) in event.actions.iter().enumerate() {
                match action {
                    EventAction::Set { variable, .. }
                    | EventAction::Add { variable, .. }
                    | EventAction::Subtract { variable, .. } => {
                        if !variable_ids.contains(variable.as_str()) {
                            return Err(format!(
                                "Event '{}' action refers to a missing variable",
                                event.name
                            ));
                        }
                    }
                    EventAction::Scene { scene } => {
                        if !scenes.contains(scene.as_str()) || index + 1 != event.actions.len() {
                            return Err(format!("Event '{}' scene change must be its final action and target an existing scene", event.name));
                        }
                    }
                    EventAction::Music { track }
                        if usize::from(*track) >= assets.audio_tracks.len() =>
                    {
                        return Err(format!("Event '{}' refers to missing music", event.name))
                    }
                    EventAction::Sfx { effect }
                        if usize::from(*effect) >= assets.sound_effects.len() =>
                    {
                        return Err(format!(
                            "Event '{}' refers to a missing sound effect",
                            event.name
                        ))
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    pub(super) fn event_declarations(&self, source: &mut String) {
        for variable in &self.variables {
            writeln!(source, "DIM gv_{} AS WORD", variable.name).unwrap();
        }
        if !self.events.is_empty() {
            if self.entity_slots() == 0 {
                source.push_str("DIM gf_changed AS BYTE\n");
            }
            source.push_str("DIM gf_event_entry AS BYTE\n");
            for index in 0..self.events.len() {
                writeln!(source, "DIM gf_event{index} AS BYTE").unwrap();
            }
        }
    }

    pub(super) fn event_source(&self, source: &mut String) {
        if self.events.is_empty() {
            return;
        }
        source.push_str("\nSUB GameFlowEvents()\n");
        for (index, event) in self.events.iter().enumerate() {
            let scene = self
                .scenes
                .iter()
                .position(|scene| scene.id == event.scene)
                .unwrap();
            // Recheck scene/guard for every rule: a room change stops the old room's remaining rules.
            writeln!(source, "    IF gf_changed = 0 AND gf_scene = {scene} THEN").unwrap();
            let trigger = match event.trigger {
                EventTrigger::Enter => "gf_event_entry".into(),
                EventTrigger::Update => "255".into(),
                EventTrigger::Pressed { button } => {
                    format!("Controller.IsPressed(Button.{button:?})")
                }
                EventTrigger::Held { button } => format!("Controller.IsHeld(Button.{button:?})"),
                EventTrigger::Timer { frames } => format!("gf_ticks >= {frames}"),
            };
            let mut conditions = vec![trigger];
            if event.once_per_visit {
                conditions.push(format!("gf_event{index} = 0"));
            }
            for condition in &event.conditions {
                let left = match &condition.source {
                    ConditionSource::Variable { variable } => self.variable_source(variable),
                    ConditionSource::Pickups => if self.entity_slots() > 0 {
                        "gf_pickups"
                    } else {
                        "0"
                    }
                    .into(),
                    ConditionSource::X => "gf_x".into(),
                    ConditionSource::Y => "gf_y".into(),
                    ConditionSource::Ticks => "gf_ticks".into(),
                };
                let op = match condition.comparison {
                    Comparison::Equal => "=",
                    Comparison::NotEqual => "<>",
                    Comparison::Less => "<",
                    Comparison::LessEqual => "<=",
                    Comparison::Greater => ">",
                    Comparison::GreaterEqual => ">=",
                };
                conditions.push(format!("{left} {op} {}", condition.value));
            }
            // Parentheses keep comparisons separate from bitwise boolean operations in BASIC.
            let expression = conditions
                .iter()
                .map(|condition| format!("({condition})"))
                .collect::<Vec<_>>()
                .join(" AND ");
            writeln!(
                source,
                "        IF {expression} THEN\n            gf_event{index} = 255"
            )
            .unwrap();
            for action in &event.actions {
                match action {
                    EventAction::Set { variable, value } => writeln!(
                        source,
                        "            {} = {value}",
                        self.variable_source(variable)
                    )
                    .unwrap(),
                    EventAction::Add { variable, value } => {
                        let name = self.variable_source(variable);
                        writeln!(source, "            IF {name} > {} THEN\n                {name} = 65535\n            ELSE\n                {name} = {name} + {value}\n            END IF", 65535 - value).unwrap();
                    }
                    EventAction::Subtract { variable, value } => {
                        let name = self.variable_source(variable);
                        writeln!(source, "            IF {name} < {value} THEN\n                {name} = 0\n            ELSE\n                {name} = {name} - {value}\n            END IF").unwrap();
                    }
                    EventAction::Scene { scene } => {
                        let target = self
                            .scenes
                            .iter()
                            .position(|item| item.id == *scene)
                            .unwrap();
                        writeln!(
                            source,
                            "            gf_scene = {target}\n            GameFlowEnter()"
                        )
                        .unwrap();
                    }
                    EventAction::Music { track } => {
                        writeln!(source, "            Sound.Play({track})").unwrap()
                    }
                    EventAction::Sfx { effect } => {
                        writeln!(source, "            PLAY_SFX({effect})").unwrap()
                    }
                    EventAction::StopMusic => source.push_str("            Sound.Stop()\n"),
                }
            }
            source.push_str("        END IF\n    END IF\n");
        }
        // Entry rules run once on the next eligible update, never recursively while loading maps.
        source.push_str(
            "    IF gf_changed = 0 THEN\n        gf_event_entry = 0\n    END IF\nEND SUB\n",
        );
    }

    fn variable_source(&self, id: &str) -> String {
        format!(
            "gv_{}",
            self.variables
                .iter()
                .find(|variable| variable.id == id)
                .unwrap()
                .name
        )
    }
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64
}

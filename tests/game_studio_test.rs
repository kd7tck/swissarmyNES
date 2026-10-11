use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use std::{
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};
use swiss_emulator::Emulator;
use swissarmynes::server::{
    self,
    api::compile_source,
    game_design::{GameDesign, MAIN_SOURCE},
    project::{self, ProjectAssets},
};
use tower::ServiceExt;

struct StudioProject {
    name: String,
    path: PathBuf,
    assets: ProjectAssets,
    design: GameDesign,
}
impl StudioProject {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let name = format!(
            "studio-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        // Use exactly the original source assets used by the browser's starter action.
        let output = Command::new("node").args(["-e", "process.stdout.write(JSON.stringify(require('./static/js/studio_starter.js').build()))"])
            .output().expect("Node is required by project verification");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let starter: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let assets: ProjectAssets = serde_json::from_value(starter["assets"].clone()).unwrap();
        let design: GameDesign = serde_json::from_value(starter["design"].clone()).unwrap();
        assert_eq!(
            starter["source"].as_str().unwrap(),
            MAIN_SOURCE,
            "Browser and native test harness must agree"
        );
        let path = PathBuf::from(project::PROJECTS_DIR).join(&name);
        assert!(!path.exists());
        project::create_project_with_content(
            &name,
            Some(MAIN_SOURCE),
            Some(&assets),
            Some(&design),
        )
        .unwrap();
        Self {
            name,
            path,
            assets,
            design,
        }
    }
    fn compile(&self) -> (Vec<u8>, swissarmynes::compiler::source_map::LinkedSourceMap) {
        compile_source(None, Some(self.name.clone()), None).expect("Authored starter must compile")
    }
}
impl Drop for StudioProject {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.path)
            .expect("Only this uniquely created test project is removed");
    }
}
fn frames(emulator: &mut Emulator, count: usize) {
    for _ in 0..count {
        emulator.step().unwrap();
        emulator.clear_audio_samples();
    }
}
fn word(emulator: &Emulator, offset: usize) -> u16 {
    let ram = emulator.ram_snapshot();
    u16::from_le_bytes([ram[offset], ram[offset + 1]])
}

#[test]
fn visual_events_execute_variables_timers_conditions_and_room_guards_in_rom() {
    let mut project = StudioProject::new();
    project.design.start_scene = "meadow".into();
    project.design.scenes[1].spawn_x = 64;
    project.design.variables = serde_json::from_value(serde_json::json!([
        {"id":"score","name":"Score","initial":0},
        {"id":"ceiling","name":"Ceiling","initial":65534},
        {"id":"floor","name":"Floor","initial":1},
        {"id":"held","name":"HeldFrames","initial":0}
    ]))
    .unwrap();
    project.design.events = serde_json::from_value(serde_json::json!([
        {"id":"entry","name":"Entry score","scene":"meadow","trigger":{"kind":"enter"},"actions":[{"kind":"add","variable":"score","value":1}]},
        {"id":"timer","name":"Timer bonus","scene":"meadow","trigger":{"kind":"timer","frames":4},"once_per_visit":true,"actions":[{"kind":"add","variable":"score","value":10}]},
        {"id":"clamp","name":"Clamp and audio","scene":"meadow","trigger":{"kind":"pressed","button":"a"},"actions":[{"kind":"add","variable":"ceiling","value":10},{"kind":"subtract","variable":"floor","value":2},{"kind":"music","track":0},{"kind":"sfx","effect":0},{"kind":"stop_music"}]},
        {"id":"hold","name":"Held input","scene":"meadow","trigger":{"kind":"held","button":"up"},"conditions":[{"source":{"kind":"x"},"comparison":"greater_equal","value":64},{"source":{"kind":"y"},"comparison":"less_equal","value":128}],"actions":[{"kind":"add","variable":"held","value":1}]},
        {"id":"win","name":"Coin wins","scene":"meadow","trigger":{"kind":"update"},"conditions":[{"source":{"kind":"pickups"},"comparison":"greater","value":0},{"source":{"kind":"variable","variable":"score"},"comparison":"equal","value":11}],"actions":[{"kind":"scene","scene":"ruins"}]},
        {"id":"stale","name":"Old room must stop","scene":"meadow","trigger":{"kind":"update"},"conditions":[{"source":{"kind":"pickups"},"comparison":"not_equal","value":0},{"source":{"kind":"variable","variable":"score"},"comparison":"less","value":12}],"actions":[{"kind":"set","variable":"score","value":666}]},
        {"id":"arrival","name":"Arrival bonus","scene":"ruins","trigger":{"kind":"enter"},"actions":[{"kind":"add","variable":"score","value":100}]}
    ])).unwrap();
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    let (rom, _) = project.compile();
    let mut emulator = Emulator::new();
    emulator.load_rom(&rom).unwrap();
    let value =
        |emulator: &Emulator, id| word(emulator, project.design.variable_address(id).unwrap());
    frames(&mut emulator, 12);
    assert_eq!(value(&emulator, "score"), 11, "Entry and timer fire once");
    emulator.set_button(0, 0, true);
    frames(&mut emulator, 3);
    assert_eq!(
        value(&emulator, "ceiling"),
        65535,
        "Addition saturates instead of wrapping"
    );
    assert_eq!(
        value(&emulator, "floor"),
        0,
        "Subtraction saturates instead of wrapping"
    );
    emulator.set_button(0, 0, false);
    emulator.set_button(0, 4, true);
    frames(&mut emulator, 3);
    assert!(
        (2..=4).contains(&value(&emulator, "held")),
        "Held input repeats each eligible update"
    );
    emulator.set_button(0, 4, false);
    emulator.set_button(0, 7, true);
    frames(&mut emulator, 4);
    assert_eq!(emulator.ram_snapshot()[0x5c0], 2);
    assert_eq!(
        value(&emulator, "score"),
        111,
        "Destination entry runs without old-room cascade"
    );
    emulator.set_button(0, 7, false);
    frames(&mut emulator, 4);
    assert_eq!(
        value(&emulator, "score"),
        111,
        "Entry never repeats while remaining in a room"
    );
    emulator.set_button(0, 1, true);
    frames(&mut emulator, 2);
    emulator.set_button(0, 1, false);
    frames(&mut emulator, 8);
    assert_eq!(
        value(&emulator, "score"),
        122,
        "Variables persist, entry and once-per-visit timers rearm"
    );
    assert_eq!(emulator.ram_snapshot()[0x5c0], 1);
    assert_eq!(project.design.generated_ram_bytes(), 60);
}

#[test]
fn entry_events_without_entity_slots_defer_room_chains_and_reset_on_restart() {
    let mut project = StudioProject::new();
    for scene in &mut project.design.scenes {
        scene.instances.clear();
    }
    project.design.variables =
        serde_json::from_value(serde_json::json!([{ "id":"state","name":"State","initial":7 }]))
            .unwrap();
    project.design.events = serde_json::from_value(serde_json::json!([
        {"id":"leave","name":"Leave title","scene":"title","trigger":{"kind":"enter"},"actions":[{"kind":"add","variable":"state","value":1},{"kind":"scene","scene":"meadow"}]},
        {"id":"arrive","name":"Arrive","scene":"meadow","trigger":{"kind":"enter"},"conditions":[{"source":{"kind":"ticks"},"comparison":"greater_equal","value":1}],"actions":[{"kind":"add","variable":"state","value":10}]}
    ])).unwrap();
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    let (rom, _) = project.compile();
    let mut emulator = Emulator::new();
    emulator.load_rom(&rom).unwrap();
    frames(&mut emulator, 12);
    let offset = project.design.variable_address("state").unwrap();
    assert_eq!(word(&emulator, offset), 18);
    assert_eq!(emulator.ram_snapshot()[0x5c0], 1);
    assert_eq!(project.design.generated_ram_bytes(), 20);
    emulator.reset();
    frames(&mut emulator, 12);
    assert_eq!(
        word(&emulator, offset),
        18,
        "Start initializes variables again"
    );
}

#[tokio::test]
async fn saved_scenarios_assert_variables_and_report_missing_assertions() {
    let mut project = StudioProject::new();
    project.design.variables = serde_json::from_value(
        serde_json::json!([{ "id":"score","name":"Score","initial":60000 }]),
    )
    .unwrap();
    project.design.events = serde_json::from_value(serde_json::json!([
        {"id":"add","name":"Large score","scene":"title","trigger":{"kind":"pressed","button":"a"},"actions":[{"kind":"add","variable":"score","value":5535}]}
    ])).unwrap();
    project.design.tests = serde_json::from_value(serde_json::json!([
        {"id":"variables","name":"Score test","steps":[{"frames":2,"buttons":[],"expected_variables":{"score":60000}},{"frames":2,"buttons":["a"],"expected_variables":{"score":65535}},{"frames":2,"buttons":["a"],"expected_variables":{"score":65535}}]}
    ])).unwrap();
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    let request = || {
        Request::builder()
            .method("POST")
            .uri(format!("/api/projects/{}/flow/test", project.name))
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"test_id":"variables"}"#))
            .unwrap()
    };
    let response = server::app().oneshot(request()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let report: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(report["passed"], true);
    assert_eq!(report["variables"]["score"], 65535);
    project.design.tests[0].steps[0]
        .expected_variables
        .insert("removed".into(), 0);
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    let response = server::app().oneshot(request()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let report: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(report["passed"], false);
    assert!(report["message"].as_str().unwrap().contains("removed"));
}

#[test]
fn malformed_event_references_are_rejected_before_source_writes() {
    let mut project = StudioProject::new();
    project.design.variables =
        serde_json::from_value(serde_json::json!([{ "id":"score","name":"Score","initial":0 }]))
            .unwrap();
    let base = serde_json::json!({"id":"event","name":"Rule","scene":"title","trigger":{"kind":"timer","frames":1},"conditions":[],"actions":[{"kind":"set","variable":"score","value":1}]});
    for invalid in [
        serde_json::json!({"scene":"missing"}),
        serde_json::json!({"trigger":{"kind":"timer","frames":0}}),
        serde_json::json!({"trigger":{"kind":"held","button":"timer"}}),
        serde_json::json!({"conditions":[{"source":{"kind":"variable","variable":"missing"},"comparison":"equal","value":0}]}),
        serde_json::json!({"actions":[{"kind":"add","variable":"missing","value":1}]}),
        serde_json::json!({"actions":[{"kind":"scene","scene":"missing"}]}),
        serde_json::json!({"actions":[{"kind":"scene","scene":"meadow"},{"kind":"stop_music"}]}),
        serde_json::json!({"actions":[{"kind":"music","track":255}]}),
        serde_json::json!({"actions":[{"kind":"sfx","effect":255}]}),
        serde_json::json!({"actions":[]}),
    ] {
        let mut event = base.clone();
        for (key, value) in invalid.as_object().unwrap() {
            event[key] = value.clone();
        }
        project.design.events = vec![serde_json::from_value(event).unwrap()];
        assert!(project::save_project_with_design(
            &project.name,
            Some("must not overwrite"),
            None,
            Some(&project.design)
        )
        .is_err());
        assert_eq!(
            project::read_file(&project.name, "main.swiss").unwrap(),
            MAIN_SOURCE
        );
    }
    project.design.events = vec![serde_json::from_value(base).unwrap()];
    project.design.variables[0].name = "bad\nsource".into();
    assert!(project.design.validate(&project.assets).is_err());
    project.design.variables[0].name = "Score".into();
    project
        .design
        .variables
        .push(serde_json::from_value(serde_json::json!({"id":"other","name":"score"})).unwrap());
    assert!(
        project.design.validate(&project.assets).is_err(),
        "Source names are case-insensitively unique"
    );
}

#[test]
fn starter_boots_transitions_collides_and_keeps_source_provenance() {
    let project = StudioProject::new();
    let (rom, map) = project.compile();
    assert!(map.sources.contains_key("game_flow.swiss"));
    assert!(map
        .entries
        .iter()
        .any(|entry| entry.file == "game_flow.swiss"));
    let mut emulator = Emulator::new();
    emulator.load_rom(&rom).unwrap();
    frames(&mut emulator, 12);
    assert_eq!(emulator.ram_snapshot()[0x5c0], 0);
    assert_eq!(emulator.ram_snapshot()[0x5c8], 255);
    assert_eq!(word(&emulator, 0x5c3), 32);
    assert_eq!(word(&emulator, 0x5c5), 128);
    assert_eq!(
        emulator.ram_snapshot()[0x1b],
        4,
        "16x16 player emits four sprite tiles"
    );
    emulator.set_button(0, 3, true);
    frames(&mut emulator, 3);
    assert_eq!(emulator.ram_snapshot()[0x5c0], 1, "START enters gameplay");
    emulator.set_button(0, 3, false);
    frames(&mut emulator, 2);
    emulator.set_button(0, 7, true);
    frames(&mut emulator, 200);
    assert_eq!(
        word(&emulator, 0x5c3),
        144,
        "Player stops before the authored x160 wall"
    );
    emulator.set_button(0, 7, false);
    emulator.set_button(0, 1, true);
    frames(&mut emulator, 3);
    assert_eq!(emulator.ram_snapshot()[0x5c0], 2);
    frames(&mut emulator, 5);
    assert_eq!(
        emulator.ram_snapshot()[0x5c0],
        2,
        "Held B must not bounce through scene links"
    );
    emulator.set_button(0, 1, false);
    frames(&mut emulator, 2);
    emulator.set_button(0, 2, true);
    frames(&mut emulator, 3);
    assert_eq!(emulator.ram_snapshot()[0x5c0], 0, "SELECT returns to title");
}

#[test]
fn timer_and_simultaneous_button_priority_execute_without_transition_cascades() {
    let mut project = StudioProject::new();
    project.design.scenes[2].transitions.push(
        serde_json::from_value(serde_json::json!({"trigger":"timer","target":"title","frames":20}))
            .unwrap(),
    );
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    let (rom, _) = project.compile();
    let mut emulator = Emulator::new();
    emulator.load_rom(&rom).unwrap();
    frames(&mut emulator, 10);
    emulator.set_button(0, 3, true);
    frames(&mut emulator, 2);
    emulator.set_button(0, 3, false);
    frames(&mut emulator, 2);
    emulator.set_button(0, 1, true);
    emulator.set_button(0, 2, true);
    frames(&mut emulator, 2);
    assert_eq!(
        emulator.ram_snapshot()[0x5c0],
        2,
        "First authored trigger B wins over SELECT"
    );
    frames(&mut emulator, 8);
    assert_eq!(emulator.ram_snapshot()[0x5c0], 2);
    frames(&mut emulator, 15);
    assert_eq!(
        emulator.ram_snapshot()[0x5c0],
        0,
        "Timer returns home with no new button edge"
    );
}

#[test]
fn design_persistence_virtual_source_and_rejected_edits_preserve_custom_code() {
    let project = StudioProject::new();
    assert!(project::list_files(&project.name)
        .unwrap()
        .contains(&"game_flow.swiss".into()));
    assert!(project::write_file(&project.name, "game_flow.swiss", "custom").is_err());
    assert!(project::delete_file(&project.name, "game_flow.swiss").is_err());
    assert!(
        !project.path.join("game_flow.swiss").exists(),
        "Generated source remains a virtual view"
    );
    let original = project::read_file(&project.name, "main.swiss").unwrap();
    let mut invalid = project.design.clone();
    invalid.scenes[0].transitions[0].target = "missing".into();
    assert!(project::save_project_with_design(
        &project.name,
        Some("must not overwrite"),
        Some(&project.assets),
        Some(&invalid)
    )
    .is_err());
    assert_eq!(
        project::read_file(&project.name, "main.swiss").unwrap(),
        original
    );
    project::write_file(
        &project.name,
        "main.swiss",
        "' Custom game\nSUB Main()\nEND SUB",
    )
    .unwrap();
    compile_source(Some(MAIN_SOURCE.into()), Some(project.name.clone()), None).unwrap();
    assert!(project::read_file(&project.name, "main.swiss")
        .unwrap()
        .starts_with("' Custom game"));
    assert_eq!(
        project::get_project(&project.name)
            .unwrap()
            .metadata
            .design
            .unwrap()
            .start_scene,
        "title"
    );
}

#[test]
fn invalid_graph_and_resource_references_have_actionable_errors() {
    let project = StudioProject::new();
    for modify in [
        |design: &mut GameDesign| design.version = 2,
        |design: &mut GameDesign| design.scenes[1].id = "title".into(),
        |design: &mut GameDesign| design.start_scene = "missing".into(),
        |design: &mut GameDesign| design.scenes[1].music = Some(255),
        |design: &mut GameDesign| design.scenes[1].sfx = Some(255),
        |design: &mut GameDesign| design.scenes[1].map_col = 9,
        |design: &mut GameDesign| design.scenes[1].spawn_y = 239,
        |design: &mut GameDesign| design.scenes[1].name = "bad\" source".into(),
        |design: &mut GameDesign| design.animation = Some("missing".into()),
    ] {
        let mut invalid = project.design.clone();
        modify(&mut invalid);
        assert!(invalid.source(&project.assets).is_err());
    }
    assert!(project::create_project("").is_err());
    assert!(project::create_project_with_content(
        &project.name,
        Some("overwrite"),
        Some(&project.assets),
        Some(&project.design)
    )
    .is_err());
    assert_eq!(
        project::read_file(&project.name, "main.swiss").unwrap(),
        MAIN_SOURCE
    );
}

#[tokio::test]
async fn production_http_smoke_test_runs_and_rejects_unbounded_requests() {
    let project = StudioProject::new();
    for (frames, status) in [
        (120, StatusCode::OK),
        (601, StatusCode::BAD_REQUEST),
        (0, StatusCode::BAD_REQUEST),
    ] {
        let response = server::app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/projects/{}/flow/test", project.name))
                    .header("content-type", "application/json")
                    .body(Body::from(format!("{{\"frames\":{frames}}}")))
                    .unwrap(),
            )
            .await
            .unwrap();
        let actual_status = response.status();
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        assert_eq!(actual_status, status, "{}", String::from_utf8_lossy(&body));
        if status == StatusCode::OK {
            let report: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(report["passed"], true);
            assert_eq!(report["scene_id"], "title");
            assert_eq!(report["frames"], 120);
        }
    }
}

async fn scenario_request(project: &StudioProject, id: &str) -> (StatusCode, serde_json::Value) {
    let response = server::app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/projects/{}/flow/test", project.name))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({"test_id":id}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let report = serde_json::from_slice(&body)
        .unwrap_or_else(|_| serde_json::json!({"error":String::from_utf8_lossy(&body)}));
    (status, report)
}

#[tokio::test]
async fn saved_controller_scenarios_execute_assertions_and_report_failures_without_editing_main() {
    let mut project = StudioProject::new();
    for id in ["enter-play", "round-trip", "wall-stop"] {
        let (status, report) = scenario_request(&project, id).await;
        assert_eq!(status, StatusCode::OK, "{report}");
        assert_eq!(report["passed"], true, "{report}");
        assert!(report["cpu_cycles"].as_u64().unwrap() > 0);
        assert!(!report["observations"].as_array().unwrap().is_empty());
        if id == "wall-stop" {
            assert_eq!(report["x"], 144);
            assert_eq!(report["frames"], 213);
        }
        if id == "round-trip" {
            assert_eq!(report["scene_id"], "title");
        }
    }
    project.design.tests[0].steps[0].expected_scene = Some("title".into());
    project::write_file(
        &project.name,
        "main.swiss",
        "' Custom main must survive\nSUB Main()\nEND SUB",
    )
    .unwrap();
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    let (status, report) = scenario_request(&project, "enter-play").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["passed"], false);
    assert!(report["message"]
        .as_str()
        .unwrap()
        .contains("Step 1: expected scene title"));
    assert_eq!(report["observations"][0]["passed"], false);
    assert!(project::read_file(&project.name, "main.swiss")
        .unwrap()
        .starts_with("' Custom main"));
    assert_eq!(
        scenario_request(&project, "missing").await.0,
        StatusCode::BAD_REQUEST
    );
}

#[test]
fn controller_scenario_limits_and_legacy_metadata_are_validated() {
    let project = StudioProject::new();
    for modify in [
        |design: &mut GameDesign| design.tests[0].steps[0].frames = 591,
        |design: &mut GameDesign| design.tests[0].steps[0].frames = 0,
        |design: &mut GameDesign| design.tests[0].steps.clear(),
        |design: &mut GameDesign| design.tests[1].id = design.tests[0].id.clone(),
        |design: &mut GameDesign| {
            design.tests[0].steps[0].buttons = vec![server::game_design::Trigger::Timer]
        },
        |design: &mut GameDesign| {
            design.tests[0].steps[0].buttons = vec![
                server::game_design::Trigger::A,
                server::game_design::Trigger::A,
            ]
        },
    ] {
        let mut invalid = project.design.clone();
        modify(&mut invalid);
        assert!(invalid.validate(&project.assets).is_err());
    }
    let mut legacy = serde_json::to_value(&project.design).unwrap();
    legacy.as_object_mut().unwrap().remove("tests");
    let restored: GameDesign = serde_json::from_value(legacy).unwrap();
    assert!(restored.tests.is_empty());
    restored.validate(&project.assets).unwrap();
}

fn entity_design(project: &mut StudioProject) {
    project.design.entities = serde_json::from_value(serde_json::json!([
        {"id":"coin","name":"Coin","animation":"HeroWalk","behavior":"pickup"},
        {"id":"gate","name":"Gate","animation":"HeroWalk","behavior":"door"},
        {"id":"walker","name":"Walker","animation":"HeroWalk","behavior":"patrol"}
    ]))
    .unwrap();
    project.design.scenes[1].instances = serde_json::from_value(serde_json::json!([
        {"id":"coin-1","entity":"coin","x":48,"y":128},
        {"id":"gate-1","entity":"gate","x":80,"y":128,"target_scene":"ruins"},
        {"id":"walker-1","entity":"walker","x":120,"y":64,"patrol_end_x":180}
    ]))
    .unwrap();
    project.design.scenes[2].instances = serde_json::from_value(serde_json::json!([
        {"id":"gate-2","entity":"gate","x":32,"y":128,"target_scene":"meadow"}
    ]))
    .unwrap();
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
}

#[test]
fn entities_execute_pickup_patrol_doors_and_room_slot_reset_in_real_rom() {
    let mut project = StudioProject::new();
    entity_design(&mut project);
    let (rom, _) = project.compile();
    let mut emulator = Emulator::new();
    emulator.load_rom(&rom).unwrap();
    frames(&mut emulator, 12);
    let base = project.design.pickups_address().unwrap();
    assert_eq!(word(&emulator, base), 0);
    emulator.set_button(0, 3, true);
    frames(&mut emulator, 3);
    emulator.set_button(0, 3, false);
    assert_eq!(
        emulator.ram_snapshot()[base + 3],
        255,
        "Touching edges do not overlap"
    );
    emulator.set_button(0, 7, true);
    frames(&mut emulator, 3);
    emulator.set_button(0, 7, false);
    assert_eq!(word(&emulator, base), 1);
    assert_eq!(emulator.ram_snapshot()[base + 3], 0);
    frames(&mut emulator, 10);
    assert_eq!(word(&emulator, base), 1, "Collected slot stays inactive");
    assert!(
        word(&emulator, base + 3 + 18 + 1) > 120,
        "Patrol actually moves"
    );
    frames(&mut emulator, 85);
    assert!(
        (120..=144).contains(&word(&emulator, base + 3 + 18 + 1)),
        "Patrol turns before the x160 wall even though its authored endpoint is x180"
    );
    emulator.set_button(0, 7, true);
    frames(&mut emulator, 45);
    emulator.set_button(0, 7, false);
    assert_eq!(
        emulator.ram_snapshot()[0x5c0],
        1,
        "Overlap alone cannot activate a door"
    );
    emulator.set_button(0, 0, true);
    frames(&mut emulator, 3);
    assert_eq!(
        emulator.ram_snapshot()[0x5c0],
        2,
        "A activates overlapping door"
    );
    assert_eq!(
        emulator.ram_snapshot()[base + 3 + 9],
        0,
        "Old room slots are cleared"
    );
    frames(&mut emulator, 8);
    assert_eq!(emulator.ram_snapshot()[0x5c0], 2, "Held A cannot cascade");
    emulator.set_button(0, 0, false);
    frames(&mut emulator, 2);
    emulator.set_button(0, 0, true);
    frames(&mut emulator, 3);
    assert_eq!(emulator.ram_snapshot()[0x5c0], 1);
    assert_eq!(
        emulator.ram_snapshot()[base + 3],
        255,
        "Pickups respawn on room entry"
    );
    assert_eq!(word(&emulator, base), 1, "Counter persists between rooms");
    assert_eq!(project.design.generated_ram_bytes(), 44);
}

#[tokio::test]
async fn entity_assertions_and_observations_use_the_saved_runtime_snapshot() {
    let mut project = StudioProject::new();
    entity_design(&mut project);
    project.design.tests = serde_json::from_value(serde_json::json!([
        {"id":"pickup","name":"Collect one","steps":[
            {"frames":2,"buttons":["start"],"expected_active_entities":3,"expected_pickups":0},
            {"frames":3,"buttons":["right"],"expected_active_entities":2,"expected_pickups":1}
        ]}
    ]))
    .unwrap();
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    let (status, report) = scenario_request(&project, "pickup").await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["passed"], true, "{report}");
    assert_eq!(report["entities"][0]["id"], "coin-1");
    assert_eq!(report["entities"][0]["active"], false);
    assert_eq!(report["generated_ram_bytes"], 44);
    project.design.tests[0].steps[1].expected_pickups = Some(2);
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    let (_, report) = scenario_request(&project, "pickup").await;
    assert_eq!(report["passed"], false);
    assert!(report["message"]
        .as_str()
        .unwrap()
        .contains("expected pickups 2"));
}

#[test]
fn entity_reference_and_hardware_limits_are_rejected_before_writing_source() {
    let mut project = StudioProject::new();
    entity_design(&mut project);
    for modify in [
        |d: &mut GameDesign| d.entities[0].animation = "absent".into(),
        |d: &mut GameDesign| d.scenes[1].instances[0].entity = "absent".into(),
        |d: &mut GameDesign| d.scenes[1].instances[1].target_scene = Some("absent".into()),
        |d: &mut GameDesign| d.scenes[1].instances[2].patrol_end_x = Some(120),
        |d: &mut GameDesign| d.scenes[1].instances[0].x = 241,
        |d: &mut GameDesign| d.scenes[2].instances[0].id = "coin-1".into(),
    ] {
        let mut invalid = project.design.clone();
        modify(&mut invalid);
        assert!(project::save_project_with_design(
            &project.name,
            Some("overwrite"),
            None,
            Some(&invalid)
        )
        .is_err());
        assert_eq!(
            project::read_file(&project.name, "main.swiss").unwrap(),
            MAIN_SOURCE
        );
    }
    project.design.scenes[1].instances = (0..16)
        .map(|n| {
            let mut instance = project.design.scenes[1].instances[0].clone();
            instance.id = format!("coin-{n}");
            instance
        })
        .collect();
    assert!(project
        .design
        .validate(&project.assets)
        .unwrap_err()
        .contains("OAM holds 64"));
    project.design.animation = None;
    project.design.validate(&project.assets).unwrap();
    assert_eq!(project.design.generated_ram_bytes(), 156);
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    let (rom, _) = project.compile();
    let mut emulator = Emulator::new();
    emulator.load_rom(&rom).unwrap();
    frames(&mut emulator, 12);
    emulator.set_button(0, 3, true);
    frames(&mut emulator, 3);
    assert_eq!(
        emulator.ram_snapshot()[0x1b],
        64,
        "Entity-only rooms draw without a player animation"
    );
}

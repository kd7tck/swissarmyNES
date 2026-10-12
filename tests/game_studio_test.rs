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

fn combat_design(project: &mut StudioProject) {
    for scene in &mut project.design.scenes {
        scene.instances.clear();
        scene.transitions.clear();
    }
    project.design.start_scene = "meadow".into();
    project.design.combat = Some(serde_json::from_value(serde_json::json!({
        "health":3,"invulnerability_frames":60,"attack_button":"b","attack_damage":1,"attack_reach":8,"death_scene":"ruins"
    })).unwrap());
    project.design.entities = serde_json::from_value(serde_json::json!([
        {"id":"foe","name":"Foe","animation":"HeroWalk","behavior":"enemy","enemy":{"health":2,"damage":1}}
    ])).unwrap();
    project.design.scenes[1].instances = serde_json::from_value(serde_json::json!([
        {"id":"foe-1","entity":"foe","x":48,"y":128}
    ]))
    .unwrap();
}

fn spawner_design(project: &mut StudioProject) {
    combat_design(project);
    project.design.entities[0].enemy.health = 1;
    project.design.entities.push(serde_json::from_value(serde_json::json!({
        "id":"nest-def","name":"Enemy nest","animation":"","behavior":"spawner",
        "spawner":{"entity":"foe","capacity":2,"interval_frames":4,"initial_delay_frames":30,"total":3}
    })).unwrap());
    project.design.scenes[1].instances = serde_json::from_value(serde_json::json!([
        {"id":"nest","entity":"nest-def","x":48,"y":128}
    ]))
    .unwrap();
}

#[test]
fn spawn_pool_obeys_delay_interval_capacity_reuse_health_and_finite_quota() {
    let mut project = StudioProject::new();
    spawner_design(&mut project);
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    let (rom, _) = project.compile();
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();
    frames(&mut emu, 12);
    let base = project
        .design
        .spawner_address(&project.design.scenes[1], "nest")
        .unwrap();
    let entity = project.design.pickups_address().unwrap() + 3;
    let health = project.design.combat_address().unwrap();
    assert_eq!(project.design.entity_slots(), 3);
    assert_eq!(project.design.generated_ram_bytes(), 60);
    assert_eq!(word(&emu, base + 2), 0);
    assert_eq!(
        emu.ram_snapshot()[entity],
        0,
        "Emitter has no runtime sprite"
    );
    let remaining = word(&emu, base);
    assert!(remaining > 1);
    frames(&mut emu, usize::from(remaining) - 1);
    assert_eq!(word(&emu, base + 2), 0);
    frames(&mut emu, 1);
    assert_eq!(word(&emu, base + 2), 1);
    assert_eq!(emu.ram_snapshot()[entity + 9], 255);
    assert_eq!(emu.ram_snapshot()[entity + 18], 0);
    assert_eq!(
        emu.ram_snapshot()[0x1b],
        8,
        "Player plus one spawned enemy renders"
    );
    frames(&mut emu, 3);
    assert_eq!(word(&emu, base + 2), 1);
    frames(&mut emu, 1);
    assert_eq!(word(&emu, base + 2), 2);
    assert_eq!(emu.ram_snapshot()[0x1b], 12);
    frames(&mut emu, 20);
    assert_eq!(word(&emu, base), 0);
    assert_eq!(word(&emu, base + 2), 2, "Full pool cannot consume quota");
    emu.set_button(0, 1, true);
    frames(&mut emu, 1);
    assert_eq!(word(&emu, health + 2), 2);
    assert_eq!(word(&emu, base + 2), 3);
    assert_eq!(emu.ram_snapshot()[entity + 9], 255);
    assert_eq!(
        emu.ram_snapshot()[health + 5],
        1,
        "Recycled slot restores enemy health"
    );
    assert_eq!(emu.ram_snapshot()[entity + 18], 0);
    assert_eq!(word(&emu, entity + 10), 48);
    frames(&mut emu, 6);
    assert_eq!(
        word(&emu, health + 2),
        2,
        "Newborn does not inherit the previous fresh press"
    );
    emu.set_button(0, 1, false);
    frames(&mut emu, 1);
    emu.set_button(0, 1, true);
    frames(&mut emu, 1);
    assert_eq!(word(&emu, health + 2), 3);
    frames(&mut emu, 15);
    assert_eq!(word(&emu, base + 2), 3);
    assert_eq!(emu.ram_snapshot()[0x1b], 4);
}

#[test]
fn spawner_conditions_transition_without_old_pool_state_and_restart_resets_quotas() {
    let mut project = StudioProject::new();
    spawner_design(&mut project);
    project.design.start_scene = "title".into();
    project.design.scenes[0].transitions =
        serde_json::from_value(serde_json::json!([{"trigger":"start","target":"meadow"}])).unwrap();
    project.design.entities[1]
        .spawner
        .as_mut()
        .unwrap()
        .initial_delay_frames = 0;
    project.design.entities.push(serde_json::from_value(serde_json::json!({
        "id":"slow-def","name":"Slow nest","animation":"","behavior":"spawner",
        "spawner":{"entity":"foe","capacity":1,"interval_frames":4,"initial_delay_frames":20,"total":1}
    })).unwrap());
    project.design.scenes[2].instances = serde_json::from_value(
        serde_json::json!([{"id":"slow","entity":"slow-def","x":80,"y":128}]),
    )
    .unwrap();
    project.design.variables =
        serde_json::from_value(serde_json::json!([{"id":"state","name":"State"}])).unwrap();
    project.design.events=serde_json::from_value(serde_json::json!([
        {"id":"wave","name":"Wave complete","scene":"meadow","trigger":{"kind":"update"},"conditions":[{"source":{"kind":"spawned","instance":"nest"},"comparison":"greater_equal","value":2}],"actions":[{"kind":"set","variable":"state","value":1},{"kind":"scene","scene":"ruins"}]},
        {"id":"stale","name":"Stale","scene":"meadow","trigger":{"kind":"update"},"conditions":[{"source":{"kind":"spawned","instance":"nest"},"comparison":"equal","value":2}],"actions":[{"kind":"set","variable":"state","value":666}]},
        {"id":"retry","name":"Retry","scene":"ruins","trigger":{"kind":"pressed","button":"start"},"actions":[{"kind":"restart"}]}
    ])).unwrap();
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    let (rom, _) = project.compile();
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();
    frames(&mut emu, 12);
    let meadow = project
        .design
        .spawner_address(&project.design.scenes[1], "nest")
        .unwrap();
    let ruins = project
        .design
        .spawner_address(&project.design.scenes[2], "slow")
        .unwrap();
    assert_eq!(meadow, ruins, "Per-room spawners share state");
    emu.set_button(0, 3, true);
    frames(&mut emu, 1);
    emu.set_button(0, 3, false);
    assert_eq!(emu.ram_snapshot()[0x5c0], 1);
    assert_eq!(
        word(&emu, meadow + 2),
        0,
        "A room transition defers destination spawners"
    );
    frames(&mut emu, 1);
    assert_eq!(word(&emu, meadow + 2), 1);
    frames(&mut emu, 4);
    assert_eq!(emu.ram_snapshot()[0x5c0], 2);
    assert_eq!(word(&emu, ruins + 2), 0);
    assert_eq!(word(&emu, ruins), 20);
    assert_eq!(
        word(&emu, project.design.variable_address("state").unwrap()),
        1,
        "Old-room rule cannot fire after entry clears shared counters"
    );
    let slots = project.design.pickups_address().unwrap() + 3;
    assert_eq!(emu.ram_snapshot()[slots + 9], 0);
    assert_eq!(emu.ram_snapshot()[slots + 18], 0);
    frames(&mut emu, 19);
    assert_eq!(word(&emu, ruins + 2), 0);
    frames(&mut emu, 1);
    assert_eq!(word(&emu, ruins + 2), 1);
    emu.set_button(0, 3, true);
    frames(&mut emu, 2);
    assert_eq!(emu.ram_snapshot()[0x5c0], 0);
    assert_eq!(word(&emu, ruins + 2), 0);
}

#[tokio::test]
async fn saved_spawn_assertions_report_actual_pool_state_and_preserve_missing_coverage() {
    let mut project = StudioProject::new();
    spawner_design(&mut project);
    project.design.tests=serde_json::from_value(serde_json::json!([
        {"id":"wave","name":"Wave","steps":[
            {"frames":2,"expected_spawns":{"nest":0},"expected_active_entities":0},
            {"frames":30,"expected_spawns":{"nest":2},"expected_active_entities":2},
            {"frames":1,"buttons":["b"],"expected_spawns":{"nest":3},"expected_defeated":2,"expected_active_entities":1},
            {"frames":4,"buttons":["b"],"expected_spawns":{"nest":3},"expected_active_entities":1},
            {"frames":1},{"frames":1,"buttons":["b"],"expected_spawns":{"nest":3},"expected_defeated":3,"expected_active_entities":0}
        ]}
    ])).unwrap();
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    let (status, report) = scenario_request(&project, "wave").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["passed"], true, "{report}");
    assert_eq!(report["spawners"]["nest"]["spawned"], 3);
    assert_eq!(report["spawners"]["nest"]["active"], 0);
    assert_eq!(report["entities"][1]["id"], "@spawn:nest:1");
    assert_eq!(report["entities"][1]["spawned_by"], "nest");
    assert_eq!(report["entities"][1]["health"], 0);
    project.design.scenes[1].instances.clear();
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    let (_, report) = scenario_request(&project, "wave").await;
    assert_eq!(report["passed"], false);
    assert!(report["message"]
        .as_str()
        .unwrap()
        .contains("expected spawner nest count = 0, got null"));
}

#[test]
fn spawner_references_pool_limits_and_reserved_ids_are_validated_before_source_writes() {
    let mut project = StudioProject::new();
    spawner_design(&mut project);
    for modify in [
        |d: &mut GameDesign| d.entities[1].spawner = None,
        |d: &mut GameDesign| d.entities[1].spawner.as_mut().unwrap().entity = "missing".into(),
        |d: &mut GameDesign| d.entities[1].spawner.as_mut().unwrap().entity = "nest-def".into(),
        |d: &mut GameDesign| d.entities[1].spawner.as_mut().unwrap().capacity = 0,
        |d: &mut GameDesign| d.entities[1].spawner.as_mut().unwrap().capacity = 9,
        |d: &mut GameDesign| d.entities[1].spawner.as_mut().unwrap().interval_frames = 0,
        |d: &mut GameDesign| {
            d.entities[0].behavior = swissarmynes::server::game_design::EntityBehavior::Decoration
        },
    ] {
        let mut invalid = project.design.clone();
        modify(&mut invalid);
        assert!(project::save_project_with_design(
            &project.name,
            Some("must not overwrite"),
            None,
            Some(&invalid)
        )
        .is_err());
        assert_eq!(
            project::read_file(&project.name, "main.swiss").unwrap(),
            MAIN_SOURCE
        );
    }
    let mut invalid = project.design.clone();
    invalid.animation = None;
    invalid.entities[1].spawner.as_mut().unwrap().capacity = 8;
    let mut second = invalid.scenes[1].instances[0].clone();
    second.id = "second".into();
    invalid.scenes[1].instances.push(second);
    assert!(invalid
        .validate(&project.assets)
        .unwrap_err()
        .contains("runtime slots"));
    let mut invalid = project.design.clone();
    let mut collision = invalid.scenes[1].instances[0].clone();
    collision.id = "@spawn:nest:1".into();
    collision.entity = "foe".into();
    invalid.scenes[2].instances.push(collision);
    assert!(invalid
        .validate(&project.assets)
        .unwrap_err()
        .contains("IDs must not collide"));
    project.design.events=serde_json::from_value(serde_json::json!([
        {"id":"bad","name":"Bad","scene":"title","trigger":{"kind":"update"},"conditions":[{"source":{"kind":"spawned","instance":"nest"},"comparison":"equal","value":1}],"actions":[{"kind":"stop_music"}]}
    ])).unwrap();
    assert!(project
        .design
        .validate(&project.assets)
        .unwrap_err()
        .contains("in its scene"));
    project.design.events.clear();
    let frame = &project.assets.animations[0].frames[0].metasprite;
    let sprite = project
        .assets
        .metasprites
        .iter_mut()
        .find(|sprite| sprite.name == *frame)
        .unwrap();
    sprite.tiles = vec![sprite.tiles[0].clone(); 22];
    assert!(project
        .design
        .validate(&project.assets)
        .unwrap_err()
        .contains("OAM holds 64"));
}

#[test]
fn unlimited_spawners_keep_recycling_after_the_word_counter_saturates() {
    let mut project = StudioProject::new();
    spawner_design(&mut project);
    let spawn = project.design.entities[1].spawner.as_mut().unwrap();
    spawn.capacity = 1;
    spawn.interval_frames = 1;
    spawn.initial_delay_frames = 0;
    spawn.total = 0;
    let source = MAIN_SOURCE.replace(
        "    GameFlowStart()\n",
        "    GameFlowStart()\n    gf_sp0_spawned = 65535\n",
    );
    project::save_project_with_design(&project.name, Some(&source), None, Some(&project.design))
        .unwrap();
    let (rom, _) = project.compile();
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();
    frames(&mut emu, 12);
    let base = project
        .design
        .spawner_address(&project.design.scenes[1], "nest")
        .unwrap();
    let hp = project.design.combat_address().unwrap();
    assert_eq!(word(&emu, base + 2), 65535);
    assert_eq!(emu.ram_snapshot()[hp + 5], 1);
    emu.set_button(0, 1, true);
    frames(&mut emu, 1);
    assert_eq!(word(&emu, hp + 2), 1);
    assert_eq!(word(&emu, base + 2), 65535);
    assert_eq!(emu.ram_snapshot()[hp + 5], 1);
    emu.set_button(0, 1, false);
    frames(&mut emu, 1);
    emu.set_button(0, 1, true);
    frames(&mut emu, 1);
    assert_eq!(word(&emu, hp + 2), 2);
}

#[test]
fn maximum_eight_enemy_pool_compiles_patrols_and_draws_reserved_capacity() {
    let mut project = StudioProject::new();
    spawner_design(&mut project);
    let spawn = project.design.entities[1].spawner.as_mut().unwrap();
    spawn.capacity = 8;
    spawn.interval_frames = 1;
    spawn.initial_delay_frames = 0;
    spawn.total = 8;
    project.design.scenes[1].instances[0].x = 80;
    project.design.scenes[1].instances[0].y = 64;
    project.design.scenes[1].instances[0].patrol_end_x = Some(112);
    // Frame stepping can sample a live OAM builder halfway through drawing. Inspect its
    // completed output at a source-mapped boundary, retaining the exact sprite-count assertion.
    let source = MAIN_SOURCE.replace(
        "        ' Add custom game systems here. The flow has already read the controller.",
        "        FrameComplete()",
    ) + "\nSUB FrameComplete()\nEND SUB\n";
    let completed_line = source
        .lines()
        .position(|line| line.trim() == "FrameComplete()")
        .unwrap()
        + 1;
    project::save_project_with_design(&project.name, Some(&source), None, Some(&project.design))
        .unwrap();
    let (rom, map) = project.compile();
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();
    frames(&mut emu, 50);
    let completed = map
        .entries
        .iter()
        .filter(|entry| entry.file == "main.swiss" && entry.line == completed_line)
        .min_by_key(|entry| entry.cpu_start)
        .unwrap();
    emu.add_breakpoint(completed.bank, completed.cpu_start);
    let mut stopped = false;
    for _ in 0..4 {
        if emu.step().unwrap() {
            stopped = true;
            break;
        }
        emu.clear_audio_samples();
    }
    assert!(
        stopped,
        "Production core must reach the completed draw boundary"
    );
    let base = project
        .design
        .spawner_address(&project.design.scenes[1], "nest")
        .unwrap();
    assert_eq!(word(&emu, base + 2), 8);
    assert_eq!(project.design.generated_ram_bytes(), 120);
    assert_eq!(emu.ram_snapshot()[0x1b], 36);
    let slot_base = project.design.pickups_address().unwrap() + 3;
    let roles = project
        .design
        .scene_entities(&project.design.scenes[1])
        .unwrap();
    for role in roles.iter().filter(|role| role.spawned_by.is_some()) {
        assert_eq!(emu.ram_snapshot()[slot_base + role.slot * 9], 255);
        assert!(
            (80..=112).contains(&word(&emu, slot_base + role.slot * 9 + 1)),
            "Spawned enemies inherit their patrol path"
        );
    }
}

#[test]
fn spawned_enemy_death_cannot_restore_old_slots_over_destination_state() {
    let mut project = StudioProject::new();
    spawner_design(&mut project);
    project.design.combat.as_mut().unwrap().health = 1;
    project.design.entities[0].enemy.damage = 5;
    project.design.entities[1]
        .spawner
        .as_mut()
        .unwrap()
        .initial_delay_frames = 0;
    project.design.scenes[1].instances[0].x = 32;
    project.design.scenes[2].instances = serde_json::from_value(serde_json::json!([
        {"id":"destination-actor","entity":"foe","x":96,"y":128}
    ]))
    .unwrap();
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    let (rom, _) = project.compile();
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();
    frames(&mut emu, 15);
    assert_eq!(emu.ram_snapshot()[0x5c0], 2);
    assert_eq!(
        emu.ram_snapshot()[project.design.combat_address().unwrap()],
        0
    );
    let base = project.design.pickups_address().unwrap() + 3;
    assert_eq!(emu.ram_snapshot()[base], 255);
    assert_eq!(word(&emu, base + 1), 96);
    assert_eq!(
        emu.ram_snapshot()[base + 9],
        0,
        "Shared enemy worker must not write old pool state after room entry"
    );
    assert_eq!(emu.ram_snapshot()[base + 18], 0);
    frames(&mut emu, 20);
    assert_eq!(emu.ram_snapshot()[base + 9], 0);
}

#[test]
fn studio_visual_variants_share_animation_data_and_player_facing_mirrors_oam() {
    let mut project = StudioProject::new();
    for scene in &mut project.design.scenes {
        scene.instances.clear();
        scene.transitions.clear();
    }
    project.design.start_scene = "meadow".into();
    project.design.player_visual =
        serde_json::from_value(serde_json::json!({"palette":3,"face_movement":true})).unwrap();
    project.design.entities=serde_json::from_value(serde_json::json!([
        {"id":"red","name":"Red shirt","animation":"HeroWalk","behavior":"decoration","visual":{"palette":1}},
        {"id":"green","name":"Green shirt","animation":"HeroWalk","behavior":"decoration","visual":{"palette":2,"flip_x":true,"flip_y":true}}
    ])).unwrap();
    project.design.scenes[1].instances = serde_json::from_value(serde_json::json!([
        {"id":"red-1","entity":"red","x":80,"y":64},{"id":"green-1","entity":"green","x":112,"y":64}
    ]))
    .unwrap();
    let source = MAIN_SOURCE.replace(
        "        ' Add custom game systems here. The flow has already read the controller.",
        "        FrameComplete()",
    ) + "\nSUB FrameComplete()\nEND SUB\n";
    let line = source
        .lines()
        .position(|line| line.trim() == "FrameComplete()")
        .unwrap()
        + 1;
    project::save_project_with_design(&project.name, Some(&source), None, Some(&project.design))
        .unwrap();
    let (rom, map) = project.compile();
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();
    frames(&mut emu, 12);
    emu.set_button(0, 6, true);
    frames(&mut emu, 2);
    let completed = map
        .entries
        .iter()
        .find(|entry| entry.file == "main.swiss" && entry.line == line)
        .unwrap();
    emu.add_breakpoint(completed.bank, completed.cpu_start);
    let mut stopped = false;
    for _ in 0..4 {
        if emu.step().unwrap() {
            stopped = true;
            break;
        }
    }
    assert!(stopped);
    let ram = emu.ram_snapshot();
    assert_eq!(
        ram[0x202] & 0x43,
        0x43,
        "Player retains left facing and palette3"
    );
    assert_eq!(u16::from(ram[0x203]), word(&emu, 0x5c3) + 8);
    assert_eq!(ram[0x212] & 3, 1);
    assert_eq!(ram[0x222] & 0xC3, 0xC2);
    assert_eq!(ram[0x223], 120);
    assert_eq!(ram[0x220], 72);
    let slots = project.design.pickups_address().unwrap() + 3;
    assert_eq!(
        word(&emu, slots + 4),
        word(&emu, slots + 13),
        "Both characters point to the same animation table"
    );
    assert_eq!(project.design.visual_ram_bytes(), 1);
    let mut invalid = project.design.clone();
    invalid.entities[0].visual.palette = Some(4);
    assert!(invalid.validate(&project.assets).is_err());
}

#[test]
fn colored_dual_spawners_with_all_palette_slots_and_audio_fit_fixed_bank() {
    use swissarmynes::server::{
        game_combat::{AttackMode, EnemyMotion},
        game_design::EntityBehavior,
    };
    let mut project = StudioProject::new();
    spawner_design(&mut project);
    project.design.player_visual.face_movement = true;
    project.design.entities[0].visual.palette = Some(0);
    project.design.entities[0].visual.face_movement = true;
    let mut green = project.design.entities[0].clone();
    green.id = "green".into();
    green.name = "Green".into();
    green.visual.palette = Some(1);
    project.design.entities.push(green);
    let mut green_nest = project.design.entities[1].clone();
    green_nest.id = "green-nest".into();
    green_nest.spawner.as_mut().unwrap().entity = "green".into();
    project.design.entities.push(green_nest);
    project.design.scenes[1].instances = serde_json::from_value(serde_json::json!([
        {"id":"nest","entity":"nest-def","x":48,"y":64,"patrol_end_x":80},
        {"id":"green-nest-1","entity":"green-nest","x":88,"y":64,"patrol_end_x":120}
    ]))
    .unwrap();
    let mut legacy = project.design.entities[0].clone();
    legacy.id = "legacy".into();
    legacy.behavior = swissarmynes::server::game_design::EntityBehavior::Decoration;
    legacy.visual = Default::default();
    project.design.entities.push(legacy);
    project.design.scenes[1].instances.push(
        serde_json::from_value(
            serde_json::json!({"id":"legacy-1","entity":"legacy","x":32,"y":192}),
        )
        .unwrap(),
    );
    let base = project.assets.palettes[0].clone();
    project.assets.palettes = ["BG0", "BG1", "BG2", "BG3", "SP0", "SP1", "SP2", "SP3"]
        .iter()
        .map(|name| {
            project
                .assets
                .palettes
                .iter()
                .find(|palette| palette.name == *name)
                .cloned()
                .unwrap_or_else(|| {
                    let mut palette = base.clone();
                    palette.name = (*name).into();
                    palette
                })
        })
        .collect();
    project::save_project_with_design(
        &project.name,
        None,
        Some(&project.assets),
        Some(&project.design),
    )
    .unwrap();
    let (rom, _) = project.compile();
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();
    frames(&mut emu, 60);
    for id in ["nest", "green-nest-1"] {
        assert_eq!(
            word(
                &emu,
                project
                    .design
                    .spawner_address(&project.design.scenes[1], id)
                    .unwrap()
                    + 2
            ),
            2
        );
    }
    // Exercise the richer behavior together with every palette, audio, both rendering
    // paths and larger pools. Isolated combat tests cannot prove combined ROM capacity.
    project.design.combat.as_mut().unwrap().attack_mode = AttackMode::Facing;
    project
        .design
        .combat
        .as_mut()
        .unwrap()
        .attack_cooldown_frames = 12;
    for entity in &mut project.design.entities {
        if entity.behavior == EntityBehavior::Enemy {
            entity.enemy.motion = EnemyMotion::Pursue;
            entity.enemy.detection_range = 255;
        }
        if let Some(spawn) = &mut entity.spawner {
            spawn.capacity = 6;
            spawn.total = 6;
        }
    }
    project::save_project_with_design(
        &project.name,
        None,
        Some(&project.assets),
        Some(&project.design),
    )
    .unwrap();
    let (rom, _) = project.compile();
    emu.load_rom(&rom).unwrap();
    frames(&mut emu, 80);
    for id in ["nest", "green-nest-1"] {
        let base = project
            .design
            .spawner_address(&project.design.scenes[1], id)
            .unwrap();
        assert_eq!(word(&emu, base + 2), 6);
    }
    let base = project.design.pickups_address().unwrap() + 3;
    for role in project
        .design
        .scene_entities(&project.design.scenes[1])
        .unwrap()
        .iter()
        .filter(|role| role.spawned_by.is_some())
    {
        assert_eq!(emu.ram_snapshot()[base + role.slot * 9], 255);
        assert_eq!(
            word(&emu, base + role.slot * 9 + 1),
            u16::from(role.instance.x),
            "Pursuit stops at the left bound"
        );
    }
}

#[test]
fn facing_melee_hits_only_the_front_with_vertical_overlap_and_retains_released_facing() {
    use swissarmynes::server::game_combat::AttackMode;
    let mut project = StudioProject::new();
    combat_design(&mut project);
    project.design.combat.as_mut().unwrap().attack_mode = AttackMode::Facing;
    project.design.entities[0].enemy.health = 1;
    project.design.scenes[1].instances = serde_json::from_value(serde_json::json!([
        {"id":"left","entity":"foe","x":8,"y":128},
        {"id":"right","entity":"foe","x":48,"y":128},
        {"id":"above","entity":"foe","x":48,"y":112}
    ]))
    .unwrap();
    // No player animation: attack facing must still be allocated and updated.
    project.design.animation = None;
    let base = project.design.combat_address().unwrap();
    let old = {
        let mut legacy = project.design.clone();
        legacy.combat.as_mut().unwrap().attack_mode = AttackMode::AllDirections;
        legacy.generated_ram_bytes()
    };
    assert_eq!(project.design.generated_ram_bytes(), old + 1);
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    let (rom, _) = project.compile();
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();
    frames(&mut emu, 12);
    emu.set_button(0, 1, true);
    frames(&mut emu, 2);
    assert_eq!(&emu.ram_snapshot()[base + 4..base + 7], &[1, 0, 1]);
    assert_eq!(word(&emu, base + 2), 1);
    frames(&mut emu, 8);
    assert_eq!(word(&emu, base + 2), 1, "Held melee does not repeat");
    emu.set_button(0, 1, false);
    emu.set_button(0, 6, true);
    frames(&mut emu, 2);
    emu.set_button(0, 6, false);
    frames(&mut emu, 2);
    emu.set_button(0, 1, true);
    frames(&mut emu, 2);
    assert_eq!(&emu.ram_snapshot()[base + 4..base + 7], &[0, 0, 1]);
    assert_eq!(
        word(&emu, base + 2),
        2,
        "Released left-facing attack reaches left only"
    );
}

#[test]
fn facing_melee_screen_edges_do_not_wrap_and_wall_inputs_set_direction() {
    use swissarmynes::server::game_combat::AttackMode;
    for (x, button, width) in [(0, 6, 16), (240, 7, 16), (0, 6, 1), (240, 7, 1)] {
        let mut project = StudioProject::new();
        combat_design(&mut project);
        project.design.combat.as_mut().unwrap().attack_mode = AttackMode::Facing;
        project.design.entities[0].enemy.health = 1;
        project.design.player_hitbox.width = width;
        project.design.entities[0].hitbox.width = width;
        project.design.scenes[1].spawn_x = x;
        project.design.scenes[1].spawn_y = 64;
        project.design.scenes[1].instances = serde_json::from_value(serde_json::json!([
            {"id":"near","entity":"foe","x":x,"y":64},
            {"id":"far","entity":"foe","x":240-x,"y":64}
        ]))
        .unwrap();
        let source = MAIN_SOURCE.replace(
            "    GameFlowStart()\n",
            "    GameFlowStart()\n    gf_invulnerable = 255\n",
        );
        project::save_project_with_design(
            &project.name,
            Some(&source),
            None,
            Some(&project.design),
        )
        .unwrap();
        let (rom, _) = project.compile();
        let mut emu = Emulator::new();
        emu.load_rom(&rom).unwrap();
        frames(&mut emu, 12);
        emu.set_button(0, button, true);
        frames(&mut emu, 2);
        assert_eq!(word(&emu, 0x5c3), u16::from(x));
        emu.set_button(0, button, false);
        emu.set_button(0, 1, true);
        frames(&mut emu, 2);
        let hp = project.design.combat_address().unwrap();
        assert_eq!(&emu.ram_snapshot()[hp + 4..hp + 6], &[0, 1]);
        assert_eq!(word(&emu, hp + 2), 1);
        assert_eq!(emu.ram_snapshot()[hp], 3);
    }
}

#[test]
fn pursuit_tracks_both_directions_stops_at_bounds_and_preserves_existing_addresses() {
    use swissarmynes::server::game_combat::EnemyMotion;
    let mut project = StudioProject::new();
    combat_design(&mut project);
    project.design.entities[0].enemy.motion = EnemyMotion::Pursue;
    project.design.entities[0].enemy.detection_range = 255;
    project.design.scenes[1].spawn_x = 200;
    project.design.scenes[1].spawn_y = 64;
    project.design.scenes[1].instances[0].x = 32;
    project.design.scenes[1].instances[0].y = 64;
    project.design.scenes[1].instances[0].patrol_end_x = Some(80);
    let slot = project.design.pickups_address().unwrap() + 3;
    let bytes = project.design.generated_ram_bytes();
    let source = MAIN_SOURCE.replace("    GameFlowStart()\n", "    GameFlowStart()\n    gf_e0_x = 64\n").replace(
        "        GameFlowUpdate()\n",
        "        IF Controller.IsHeld(Button.Select) THEN\n            gf_x = 0\n        END IF\n        GameFlowUpdate()\n"
    );
    project::save_project_with_design(&project.name, Some(&source), None, Some(&project.design))
        .unwrap();
    let (rom, _) = project.compile();
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();
    frames(&mut emu, 40);
    assert_eq!(word(&emu, slot + 1), 80, "Right limit is inclusive");
    assert_eq!(emu.ram_snapshot()[slot + 3], 0);
    emu.set_button(0, 2, true);
    frames(&mut emu, 60);
    assert_eq!(word(&emu, slot + 1), 32, "Left limit is inclusive");
    assert_eq!(emu.ram_snapshot()[slot + 3], 255);
    assert_eq!(
        project.design.generated_ram_bytes(),
        bytes,
        "Pursuit adds no RAM"
    );
}

#[test]
fn pursuit_detection_waits_on_either_axis_and_checks_the_middle_wall_probe() {
    use swissarmynes::server::game_combat::EnemyMotion;
    // Rightward wall, leftward wall, horizontal range miss, vertical range miss, exact alignment.
    for (player_x, player_y, initial_x, wall, expected) in [
        (100, 127, 64, Some(10), 64),
        (32, 127, 64, Some(7), 64),
        (100, 127, 64, None, 64),
        (64, 64, 64, None, 64),
        (64, 127, 64, None, 64),
    ] {
        let mut project = StudioProject::new();
        combat_design(&mut project);
        project.design.entities[0].enemy.motion = EnemyMotion::Pursue;
        project.design.entities[0].enemy.detection_range = if wall.is_some() { 255 } else { 16 };
        project.design.scenes[1].spawn_x = player_x;
        project.design.scenes[1].spawn_y = player_y;
        project.design.scenes[1].instances[0].x = 32;
        project.design.scenes[1].instances[0].y = 127;
        project.design.scenes[1].instances[0].patrol_end_x = Some(120);
        project.assets.nametables[1].data.fill(0);
        if let Some(tile_x) = wall {
            project.assets.nametables[1].data[16 * 32 + tile_x] = 1;
        }
        let source = MAIN_SOURCE.replace(
            "    GameFlowStart()\n",
            &format!("    GameFlowStart()\n    gf_e0_x = {initial_x}\n    gf_invulnerable = 255\n"),
        );
        project::save_project_with_design(
            &project.name,
            Some(&source),
            Some(&project.assets),
            Some(&project.design),
        )
        .unwrap();
        let (rom, _) = project.compile();
        let mut emu = Emulator::new();
        emu.load_rom(&rom).unwrap();
        frames(&mut emu, 30);
        assert_eq!(
            word(&emu, project.design.pickups_address().unwrap() + 4),
            expected,
            "Pursuit must wait: player=({player_x},{player_y}), wall={wall:?}"
        );
    }
}

#[test]
fn spawned_pursuers_share_ai_and_attack_facing_and_reset_their_pool_on_entry() {
    use swissarmynes::server::game_combat::{AttackMode, EnemyMotion};
    let mut project = StudioProject::new();
    spawner_design(&mut project);
    project.design.combat.as_mut().unwrap().attack_mode = AttackMode::Facing;
    project.design.entities[0].enemy.motion = EnemyMotion::Pursue;
    project.design.entities[0].enemy.detection_range = 255;
    project.design.scenes[1].spawn_x = 200;
    project.design.scenes[1].spawn_y = 64;
    project.design.scenes[1].instances[0].x = 48;
    project.design.scenes[1].instances[0].y = 64;
    project.design.scenes[1].instances[0].patrol_end_x = Some(80);
    let spawn = project.design.entities[1].spawner.as_mut().unwrap();
    spawn.initial_delay_frames = 0;
    spawn.interval_frames = 1;
    spawn.total = 2;
    let source = MAIN_SOURCE.replace("        GameFlowUpdate()\n", "        IF Controller.IsHeld(Button.Select) THEN\n            gf_x = 32\n        END IF\n        IF Controller.IsPressed(Button.Start) THEN\n            GameFlowEnter()\n        END IF\n        GameFlowUpdate()\n");
    project::save_project_with_design(&project.name, Some(&source), None, Some(&project.design))
        .unwrap();
    let (rom, _) = project.compile();
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();
    frames(&mut emu, 45);
    let slot = project.design.pickups_address().unwrap() + 3;
    let base = project.design.combat_address().unwrap();
    assert_eq!(word(&emu, slot + 10), 80);
    assert_eq!(word(&emu, slot + 19), 80);
    emu.set_button(0, 2, true);
    frames(&mut emu, 40);
    assert_eq!(word(&emu, slot + 10), 48);
    assert_eq!(word(&emu, slot + 19), 48);
    emu.set_button(0, 1, true);
    frames(&mut emu, 2);
    assert_eq!(
        word(&emu, base + 2),
        2,
        "Both pooled enemies use the facing strike"
    );
    emu.set_button(0, 1, false);
    emu.set_button(0, 2, false);
    emu.set_button(0, 3, true);
    frames(&mut emu, 2);
    emu.set_button(0, 3, false);
    frames(&mut emu, 4);
    assert_eq!(emu.ram_snapshot()[slot + 9], 255);
    assert_eq!(emu.ram_snapshot()[slot + 18], 255);
    assert!(
        word(&emu, slot + 10) < 80,
        "Entry recreates pool at its origin"
    );
}

#[test]
fn invalid_pursuit_settings_reject_saves_for_authored_and_spawned_enemies() {
    use swissarmynes::server::game_combat::EnemyMotion;
    let mut project = StudioProject::new();
    spawner_design(&mut project);
    project.design.entities[0].enemy.motion = EnemyMotion::Pursue;
    for (end, range) in [(None, 64), (Some(48), 64), (Some(80), 0)] {
        project.design.scenes[1].instances[0].patrol_end_x = end;
        project.design.entities[0].enemy.detection_range = range;
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
    let enemy = serde_json::json!({"health":1,"damage":1,"motion":"invalid"});
    assert!(
        serde_json::from_value::<swissarmynes::server::game_combat::EnemySettings>(enemy).is_err()
    );
}

#[test]
fn attack_cooldown_shares_one_swing_across_authored_and_spawned_enemies_without_queued_presses() {
    let mut project = StudioProject::new();
    spawner_design(&mut project);
    project.design.entities[0].enemy.health = 3;
    project.design.player_visual.face_movement = true;
    let spawn = project.design.entities[1].spawner.as_mut().unwrap();
    spawn.initial_delay_frames = 0;
    spawn.interval_frames = 1;
    spawn.total = 2;
    project.design.scenes[1].instances.push(
        serde_json::from_value(serde_json::json!({
            "id":"authored","entity":"foe","x":48,"y":128
        }))
        .unwrap(),
    );
    let old_bytes = project.design.generated_ram_bytes();
    let old_spawner = project
        .design
        .spawner_address(&project.design.scenes[1], "nest");
    let old_combat = project.design.combat_address();
    assert_eq!(
        project.design.attack_address(),
        None,
        "Legacy designs have no cooldown state"
    );
    project
        .design
        .combat
        .as_mut()
        .unwrap()
        .attack_cooldown_frames = 8;
    assert_eq!(project.design.generated_ram_bytes(), old_bytes + 2);
    assert_eq!(
        project
            .design
            .spawner_address(&project.design.scenes[1], "nest"),
        old_spawner
    );
    assert_eq!(project.design.combat_address(), old_combat);
    let hp = project.design.combat_address().unwrap();
    let attack = project.design.attack_address().unwrap();
    let slots: Vec<_> = project
        .design
        .scene_entities(&project.design.scenes[1])
        .unwrap()
        .iter()
        .filter(|role| {
            role.definition.behavior == swissarmynes::server::game_design::EntityBehavior::Enemy
        })
        .map(|role| role.slot)
        .collect();
    assert_eq!(
        slots.len(),
        3,
        "Fixture needs one placed and two spawned targets"
    );
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    assert_eq!(
        project::get_project(&project.name)
            .unwrap()
            .metadata
            .design
            .unwrap()
            .combat
            .unwrap()
            .attack_cooldown_frames,
        8
    );
    let (rom, _) = project.compile();
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();
    frames(&mut emu, 12);
    emu.set_button(0, 1, true);
    frames(&mut emu, 1);
    assert_eq!(emu.ram_snapshot()[attack], 8);
    for slot in &slots {
        assert_eq!(
            emu.ram_snapshot()[hp + 4 + slot],
            2,
            "One swing reaches every target"
        );
    }
    frames(&mut emu, 2);
    emu.set_button(0, 1, false);
    frames(&mut emu, 1);
    emu.set_button(0, 1, true);
    frames(&mut emu, 1);
    assert_eq!(
        emu.ram_snapshot()[attack],
        4,
        "Early fresh press does not restart timer"
    );
    frames(&mut emu, 4);
    assert_eq!(emu.ram_snapshot()[attack], 0);
    for slot in &slots {
        assert_eq!(
            emu.ram_snapshot()[hp + 4 + slot],
            2,
            "Early press is discarded, not queued"
        );
    }
    emu.set_button(0, 1, false);
    frames(&mut emu, 1);
    emu.set_button(0, 1, true);
    frames(&mut emu, 1);
    for slot in &slots {
        assert_eq!(emu.ram_snapshot()[hp + 4 + slot], 1);
    }
    frames(&mut emu, 6);
    assert_eq!(emu.ram_snapshot()[attack], 2);
    for slot in &slots {
        assert_eq!(
            emu.ram_snapshot()[hp + 4 + slot],
            1,
            "Held button does not repeat when ready"
        );
    }
    emu.set_button(0, 1, false);
    frames(&mut emu, 1);
    assert_eq!(
        emu.ram_snapshot()[attack],
        1,
        "Still cooling down one update before expiry"
    );
    emu.set_button(0, 1, true);
    frames(&mut emu, 1);
    assert_eq!(
        word(&emu, hp + 2),
        3,
        "Fresh press accepted exactly eight updates after swing"
    );
    assert_eq!(emu.ram_snapshot()[attack], 8);
}

#[test]
fn attack_cooldown_survives_rooms_but_restart_clears_it_and_transitions_take_priority() {
    let mut project = StudioProject::new();
    combat_design(&mut project);
    project.design.entities[0].enemy.health = 3;
    project
        .design
        .combat
        .as_mut()
        .unwrap()
        .attack_cooldown_frames = 20;
    project.design.scenes[0].transitions = serde_json::from_value(serde_json::json!([
        {"target":"meadow","trigger":"start"}
    ]))
    .unwrap();
    project.design.scenes[1].transitions = serde_json::from_value(serde_json::json!([
        {"target":"title","trigger":"select"}
    ]))
    .unwrap();
    project.design.events = serde_json::from_value(serde_json::json!([
        {"id":"retry","name":"Retry","scene":"meadow","trigger":{"kind":"pressed","button":"a"},"actions":[{"kind":"restart"}]}
    ])).unwrap();
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    let (rom, _) = project.compile();
    let hp = project.design.combat_address().unwrap();
    let attack = project.design.attack_address().unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();
    frames(&mut emu, 12);
    emu.set_button(0, 1, true);
    emu.set_button(0, 2, true);
    frames(&mut emu, 2);
    assert_eq!(emu.ram_snapshot()[0x5c0], 0);
    assert_eq!(
        emu.ram_snapshot()[attack],
        0,
        "A room transition does not spend the swing"
    );
    emu.set_button(0, 1, false);
    emu.set_button(0, 2, false);
    emu.set_button(0, 3, true);
    frames(&mut emu, 2);
    emu.set_button(0, 3, false);
    emu.set_button(0, 1, true);
    frames(&mut emu, 1);
    assert_eq!(emu.ram_snapshot()[hp + 4], 2);
    assert_eq!(emu.ram_snapshot()[attack], 20);
    emu.set_button(0, 1, false);
    emu.set_button(0, 2, true);
    frames(&mut emu, 2);
    assert!(
        emu.ram_snapshot()[attack] > 0,
        "Room entry cannot clear cooldown"
    );
    emu.set_button(0, 2, false);
    emu.set_button(0, 3, true);
    frames(&mut emu, 2);
    emu.set_button(0, 3, false);
    emu.set_button(0, 1, true);
    frames(&mut emu, 2);
    assert_eq!(
        emu.ram_snapshot()[hp + 4],
        3,
        "Early press after return is discarded"
    );
    frames(&mut emu, 20);
    assert_eq!(
        emu.ram_snapshot()[hp + 4],
        3,
        "Holding through expiry does not attack"
    );
    emu.set_button(0, 1, false);
    frames(&mut emu, 1);
    emu.set_button(0, 1, true);
    frames(&mut emu, 1);
    assert_eq!(emu.ram_snapshot()[hp + 4], 2);
    emu.set_button(0, 0, true);
    frames(&mut emu, 1);
    assert_eq!(
        emu.ram_snapshot()[attack],
        0,
        "GameFlowStart clears cooldown"
    );
    assert_eq!(emu.ram_snapshot()[hp + 4], 3);
}

#[test]
fn combat_fresh_attacks_defeat_enemies_before_contact_and_preserve_variable_offsets() {
    let mut project = StudioProject::new();
    combat_design(&mut project);
    project.design.variables =
        serde_json::from_value(serde_json::json!([{"id":"score","name":"Score","initial":7}]))
            .unwrap();
    project.design.events=serde_json::from_value(serde_json::json!([
        {"id":"award","name":"Award","scene":"meadow","trigger":{"kind":"update"},"once_per_visit":true,
            "conditions":[{"source":{"kind":"defeated"},"comparison":"equal","value":1}],"actions":[{"kind":"add","variable":"score","value":100}]}
    ])).unwrap();
    let score = project.design.variable_address("score").unwrap();
    let mut legacy = project.design.clone();
    legacy.combat = None;
    assert_eq!(legacy.variable_address("score"), Some(score));
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    let (rom, _) = project.compile();
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();
    frames(&mut emu, 12);
    let base = project.design.combat_address().unwrap();
    let slot = project.design.pickups_address().unwrap() + 3;
    assert_eq!(emu.ram_snapshot()[base], 3);
    assert_eq!(emu.ram_snapshot()[base + 4], 2);
    emu.set_button(0, 1, true);
    frames(&mut emu, 2);
    assert_eq!(emu.ram_snapshot()[base + 4], 1);
    frames(&mut emu, 8);
    assert_eq!(
        emu.ram_snapshot()[base + 4],
        1,
        "Held attacks cannot repeat"
    );
    emu.set_button(0, 1, false);
    frames(&mut emu, 2);
    // Overlap the next attack: a defeated enemy cannot deal damage in the same update.
    emu.set_button(0, 7, true);
    emu.set_button(0, 1, true);
    frames(&mut emu, 2);
    assert_eq!(emu.ram_snapshot()[base + 4], 0);
    assert_eq!(emu.ram_snapshot()[slot], 0);
    assert_eq!(emu.ram_snapshot()[base], 3);
    assert_eq!(word(&emu, base + 2), 1);
    assert_eq!(word(&emu, score), 107);
    assert_eq!(
        emu.ram_snapshot()[0x1b],
        4,
        "Defeated enemy is removed from OAM"
    );
    emu.reset();
    frames(&mut emu, 12);
    assert_eq!(word(&emu, base + 2), 0);
    assert_eq!(word(&emu, score), 7);
}

#[test]
fn enemy_contact_immunity_loss_scene_and_restart_execute_without_stale_room_rules() {
    let mut project = StudioProject::new();
    combat_design(&mut project);
    project.design.start_scene = "title".into();
    project.design.scenes[0].transitions =
        serde_json::from_value(serde_json::json!([{"trigger":"start","target":"meadow"}])).unwrap();
    project.design.scenes[1].instances[0].x = 32;
    let mut second = project.design.scenes[1].instances[0].clone();
    second.id = "foe-2".into();
    project.design.scenes[1].instances.push(second);
    project.design.variables =
        serde_json::from_value(serde_json::json!([{"id":"state","name":"State","initial":7}]))
            .unwrap();
    project.design.events=serde_json::from_value(serde_json::json!([
        {"id":"stale","name":"Stale","scene":"meadow","trigger":{"kind":"update"},"conditions":[{"source":{"kind":"health"},"comparison":"equal","value":0}],"actions":[{"kind":"set","variable":"state","value":666}]},
        {"id":"loss","name":"Loss","scene":"ruins","trigger":{"kind":"enter"},"conditions":[{"source":{"kind":"health"},"comparison":"equal","value":0}],"actions":[{"kind":"add","variable":"state","value":1}]},
        {"id":"retry","name":"Retry","scene":"ruins","trigger":{"kind":"pressed","button":"start"},"actions":[{"kind":"restart"}]}
    ])).unwrap();
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    let (rom, _) = project.compile();
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();
    frames(&mut emu, 12);
    let base = project.design.combat_address().unwrap();
    let state = project.design.variable_address("state").unwrap();
    emu.set_button(0, 3, true);
    frames(&mut emu, 3);
    emu.set_button(0, 3, false);
    assert_eq!(
        emu.ram_snapshot()[base],
        2,
        "Only the first overlapping enemy damages the player"
    );
    frames(&mut emu, 30);
    assert_eq!(emu.ram_snapshot()[base], 2);
    frames(&mut emu, 32);
    assert_eq!(emu.ram_snapshot()[base], 1);
    frames(&mut emu, 60);
    assert_eq!(emu.ram_snapshot()[base], 0);
    assert_eq!(emu.ram_snapshot()[0x5c0], 2);
    assert_eq!(emu.ram_snapshot()[0x5c7], 0);
    assert_eq!(
        word(&emu, state),
        8,
        "Only the deferred loss entry rule runs"
    );
    emu.set_button(0, 7, true);
    frames(&mut emu, 3);
    assert_eq!(word(&emu, 0x5c3), 32, "Dead player cannot move");
    emu.set_button(0, 7, false);
    emu.set_button(0, 3, true);
    frames(&mut emu, 3);
    assert_eq!(emu.ram_snapshot()[0x5c0], 0);
    assert_eq!(emu.ram_snapshot()[base], 3);
    assert_eq!(word(&emu, state), 7);
}

#[tokio::test]
async fn combat_scenarios_report_health_defeats_and_fail_missing_combat_assertions() {
    let mut project = StudioProject::new();
    combat_design(&mut project);
    project.design.tests=serde_json::from_value(serde_json::json!([
        {"id":"fight","name":"Fight","steps":[
            {"frames":2,"buttons":["b"],"expected_health":3,"expected_defeated":0,"expected_active_entities":1},
            {"frames":8,"buttons":["b"],"expected_defeated":0},
            {"frames":2},{"frames":2,"buttons":["b"],"expected_health":3,"expected_defeated":1,"expected_active_entities":0}
        ]}
    ])).unwrap();
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    let (status, report) = scenario_request(&project, "fight").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["passed"], true, "{report}");
    assert_eq!(report["entities"][0]["health"], 0);
    assert_eq!(
        report["observations"][0]["state"]["entities"][0]["health"],
        1
    );
    assert_eq!(report["health"], 3);
    project
        .design
        .combat
        .as_mut()
        .unwrap()
        .attack_cooldown_frames = 8;
    project.design.tests[0].steps = serde_json::from_value(serde_json::json!([
        {"frames":1,"buttons":["b"],"expected_defeated":0},
        {"frames":1},
        {"frames":1,"buttons":["b"],"expected_defeated":0},
        {"frames":8,"buttons":["b"],"expected_defeated":0},
        {"frames":1},
        {"frames":1,"buttons":["b"],"expected_defeated":1}
    ]))
    .unwrap();
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    let (_, report) = scenario_request(&project, "fight").await;
    assert_eq!(
        report["passed"], true,
        "Cooldown must also execute in the isolated API harness: {report}"
    );
    project.design.tests[0].steps[2].expected_defeated = Some(1);
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    let (_, wrong_report) = scenario_request(&project, "fight").await;
    assert_eq!(
        wrong_report["passed"], false,
        "Deliberately wrong cooldown expectation must fail"
    );
    assert_eq!(wrong_report["defeated"], 0);
    project.design.combat = None;
    project.design.entities[0].behavior =
        swissarmynes::server::game_design::EntityBehavior::Decoration;
    project.design.tests[0].steps[0].expected_health = Some(0);
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    let (_, report) = scenario_request(&project, "fight").await;
    assert_eq!(report["passed"], false);
    assert!(report["message"]
        .as_str()
        .unwrap()
        .contains("expected health 0, got null"));
}

#[test]
fn inset_hitboxes_control_pickups_and_wall_limits_in_production_rom() {
    let mut project = StudioProject::new();
    entity_design(&mut project);
    project.design.start_scene = "meadow".into();
    project.design.entities[0].hitbox =
        serde_json::from_value(serde_json::json!({"x":8,"y":0,"width":8,"height":16})).unwrap();
    project.design.player_hitbox =
        serde_json::from_value(serde_json::json!({"x":4,"y":0,"width":8,"height":16})).unwrap();
    project::save_project_with_design(&project.name, None, None, Some(&project.design)).unwrap();
    let (rom, _) = project.compile();
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();
    frames(&mut emu, 12);
    let base = project.design.pickups_address().unwrap();
    emu.set_button(0, 7, true);
    frames(&mut emu, 12);
    assert_eq!(word(&emu, 0x5c3), 44);
    assert_eq!(word(&emu, base), 0, "Touching inset edges do not collect");
    frames(&mut emu, 1);
    assert_eq!(word(&emu, base), 1);
    frames(&mut emu, 150);
    assert_eq!(
        word(&emu, 0x5c3),
        148,
        "Right edge of the inset box stops at wall X160"
    );
}

#[test]
fn unaligned_player_and_patrol_boxes_cannot_skip_middle_wall_tiles() {
    let mut project = StudioProject::new();
    for scene in &mut project.design.scenes {
        scene.instances.clear();
        scene.transitions.clear();
    }
    project.design.start_scene = "meadow".into();
    project.design.scenes[1].spawn_x = 64;
    project.design.scenes[1].spawn_y = 127;
    project.assets.nametables[1].data.fill(0);
    project.assets.nametables[1].data[16 * 32 + 10] = 1;
    project.design.entities = serde_json::from_value(serde_json::json!([
        {"id":"walker","name":"Walker","animation":"HeroWalk","behavior":"patrol"}
    ]))
    .unwrap();
    project.design.scenes[1].instances = serde_json::from_value(serde_json::json!([
        {"id":"walker-1","entity":"walker","x":64,"y":127,"patrol_end_x":100}
    ]))
    .unwrap();
    project::save_project_with_design(
        &project.name,
        None,
        Some(&project.assets),
        Some(&project.design),
    )
    .unwrap();
    let (rom, _) = project.compile();
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();
    frames(&mut emu, 12);
    emu.set_button(0, 7, true);
    frames(&mut emu, 3);
    assert_eq!(
        word(&emu, 0x5c3),
        64,
        "Player's middle edge must hit the isolated tile at X80/Y128"
    );
    assert_eq!(
        word(&emu, project.design.pickups_address().unwrap() + 4),
        64,
        "Patrol uses the same complete leading-edge probes"
    );
}

#[test]
fn invalid_combat_hitboxes_and_restart_order_cannot_overwrite_source() {
    let mut project = StudioProject::new();
    combat_design(&mut project);
    for modify in [
        |d: &mut GameDesign| d.combat = None,
        |d: &mut GameDesign| d.combat.as_mut().unwrap().health = 0,
        |d: &mut GameDesign| d.combat.as_mut().unwrap().invulnerability_frames = 0,
        |d: &mut GameDesign| d.combat.as_mut().unwrap().attack_damage = 0,
        |d: &mut GameDesign| d.combat.as_mut().unwrap().attack_reach = 33,
        |d: &mut GameDesign| {
            d.combat.as_mut().unwrap().attack_button =
                swissarmynes::server::game_design::Trigger::Timer
        },
        |d: &mut GameDesign| d.combat.as_mut().unwrap().death_scene = "missing".into(),
        |d: &mut GameDesign| d.entities[0].enemy.health = 0,
        |d: &mut GameDesign| d.entities[0].enemy.damage = 0,
        |d: &mut GameDesign| d.player_hitbox.width = 0,
        |d: &mut GameDesign| d.entities[0].hitbox.x = 1,
        |d: &mut GameDesign| d.scenes[1].instances[0].patrol_end_x = Some(48),
    ] {
        let mut invalid = project.design.clone();
        modify(&mut invalid);
        assert!(project::save_project_with_design(
            &project.name,
            Some("must not overwrite"),
            None,
            Some(&invalid)
        )
        .is_err());
        assert_eq!(
            project::read_file(&project.name, "main.swiss").unwrap(),
            MAIN_SOURCE
        );
    }
    project.design.events=serde_json::from_value(serde_json::json!([
        {"id":"retry","name":"Retry","scene":"title","trigger":{"kind":"enter"},"actions":[{"kind":"restart"},{"kind":"stop_music"}]}
    ])).unwrap();
    assert!(project
        .design
        .validate(&project.assets)
        .unwrap_err()
        .contains("restart must be its final action"));
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

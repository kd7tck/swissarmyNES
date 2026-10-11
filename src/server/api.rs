use crate::compiler::{
    analysis::SemanticAnalyzer,
    assembler::Assembler,
    ast::{AnimationFrame, Expression, MetaspriteTile, TopLevel, TopLevelKind},
    audio,
    codegen::{CodeGenerator, ENVELOPE_TABLE_ADDR, NAMETABLE_ADDR},
    lexer::Lexer,
    parser::Parser,
    preprocessor,
    source_map::LinkedSourceMap as SourceMap,
};
use crate::server::game_design::{GameDesign, FLOW_FILE, MAIN_SOURCE, TEST_BOOT_FRAMES};
use crate::server::project::{self, ProjectAssets};
use axum::{extract::Path, http::StatusCode, response::IntoResponse, Json};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CompileRequest {
    source: Option<String>,
    project_name: Option<String>,
    assets: Option<ProjectAssets>,
}

#[derive(Serialize)]
pub struct CompileResponse {
    rom: String, // Base64 encoded ROM
    map: SourceMap,
}

pub async fn compile(Json(payload): Json<CompileRequest>) -> impl IntoResponse {
    // Spawn a blocking task for the CPU-intensive compilation process
    let result = tokio::task::spawn_blocking(move || {
        compile_source(payload.source, payload.project_name, payload.assets)
    })
    .await;

    match result {
        Ok(compile_result) => match compile_result {
            Ok((rom_data, source_map)) => {
                let encoded_rom = general_purpose::STANDARD.encode(rom_data);
                let response = CompileResponse {
                    rom: encoded_rom,
                    map: source_map,
                };
                Json(response).into_response()
            }
            Err(err_msg) => {
                // Return the compilation error message
                (StatusCode::BAD_REQUEST, err_msg).into_response()
            }
        },
        Err(join_err) => {
            // Return internal server error if the task panicked or failed to join
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal Server Error: {}", join_err),
            )
                .into_response()
        }
    }
}

pub fn compile_source(
    source: Option<String>,
    project_name: Option<String>,
    assets: Option<ProjectAssets>,
) -> Result<(Vec<u8>, SourceMap), String> {
    compile_source_with_design(source, project_name, assets, None)
}

fn compile_source_with_design(
    source: Option<String>,
    project_name: Option<String>,
    assets: Option<ProjectAssets>,
    design: Option<&GameDesign>,
) -> Result<(Vec<u8>, SourceMap), String> {
    // Resolve source
    let source_code = if let Some(s) = source {
        s
    } else if let Some(ref name) = project_name {
        // Read main.swiss from project
        match project::read_file(name, "main.swiss") {
            Ok(s) => s,
            Err(e) => return Err(format!("Failed to read main.swiss: {}", e)),
        }
    } else {
        return Err("No source provided and no project context".to_string());
    };

    // Resolve assets
    let loaded_project = project_name
        .as_ref()
        .and_then(|name| project::get_project(name).ok());
    let generated_flow = design.or_else(|| {
        loaded_project
            .as_ref()
            .and_then(|project| project.metadata.design.as_ref())
    });
    let resolved_assets = if let Some(a) = assets {
        Some(a)
    } else {
        loaded_project
            .as_ref()
            .and_then(|project| project.assets.clone())
    };

    if let Some(assets) = &resolved_assets {
        assets.validate_for_compile()?;
    }
    let flow_source = generated_flow
        .map(|design| design.source(resolved_assets.as_ref().ok_or("Game flow needs assets")?))
        .transpose()?;

    // 1. Lexing
    let mut lexer = Lexer::new(&source_code);
    let tokens = lexer
        .tokenize()
        .map_err(|e| format!("Lexer Error: {:?}", e))?;

    // 2. Parsing
    let mut parser = Parser::new(tokens);
    let program = parser
        .parse()
        .map_err(|e| format!("Parser Error: {:?}", e))?;

    // 2b. Preprocessing (Includes)
    let sources = std::cell::RefCell::new(std::collections::BTreeMap::from([(
        "main.swiss".to_string(),
        source_code.clone(),
    )]));
    let p_name = project_name.clone();
    let provider = |filename: &str| -> Result<String, String> {
        if let Some(name) = &p_name {
            let contents = if filename == FLOW_FILE {
                if let Some(source) = &flow_source {
                    source.clone()
                } else {
                    project::read_file(name, filename)?
                }
            } else {
                project::read_file(name, filename)?
            };
            sources
                .borrow_mut()
                .insert(filename.to_string(), contents.clone());
            Ok(contents)
        } else {
            Err("Includes are only supported within a named project context".to_string())
        }
    };

    let program = preprocessor::process_includes(program, &provider)
        .map_err(|e| format!("Preprocessor Error: {}", e))?;

    // 2c. Preprocessing (Macros)
    let program = preprocessor::expand_macros(program)
        .map_err(|e| format!("Macro Expansion Error: {}", e))?;

    // 2d. Inject Assets (Metasprites/Animations/Metatiles/World)
    let mut program = program;
    if let Some(assets) = &resolved_assets {
        for ms in &assets.metasprites {
            let tiles: Vec<MetaspriteTile> = ms
                .tiles
                .iter()
                .map(|t| MetaspriteTile {
                    x: Expression::Integer(t.x as i32),
                    y: Expression::Integer(t.y as i32),
                    tile: Expression::Integer(t.tile as i32),
                    attr: Expression::Integer(t.attr as i32),
                })
                .collect();
            program.declarations.push(TopLevel {
                kind: TopLevelKind::Metasprite(ms.name.clone(), tiles),
                source_file: "main.swiss".to_string(),
                line: 0,
            });
        }

        for anim in &assets.animations {
            let frames: Vec<AnimationFrame> = anim
                .frames
                .iter()
                .map(|f| AnimationFrame {
                    metasprite: f.metasprite.clone(),
                    duration: f.duration,
                })
                .collect();
            program.declarations.push(TopLevel {
                kind: TopLevelKind::Animation(anim.name.clone(), frames, anim.does_loop),
                source_file: "main.swiss".to_string(),
                line: 0,
            });
        }

        for mt in &assets.metatiles {
            program.declarations.push(TopLevel {
                kind: TopLevelKind::Metatile(mt.name.clone(), mt.tiles, mt.attr),
                source_file: "main.swiss".to_string(),
                line: 0,
            });
        }

        // World cells and complete map images are placed in switchable banks by
        // the world packer. Duplicating their metadata here would consume the
        // limited fixed bank and needlessly cap otherwise valid worlds.
    }

    // 3. Analysis
    let mut analyzer = SemanticAnalyzer::new();
    analyzer
        .analyze(&program)
        .map_err(|e| format!("Analysis Error: {:?}", e))?;

    // 4. Codegen
    let symbol_table = analyzer.symbol_table;

    // Create CodeGenerator
    let mut codegen = CodeGenerator::new(symbol_table);
    let (mut asm_lines, _) = codegen
        .generate_banks(&program)
        .map_err(|e| format!("Codegen Error: {:?}", e))?;
    // KEEP
    // 5. Assembler
    let assembler = Assembler::new();
    crate::compiler::world::append_assets(&mut asm_lines, resolved_assets.as_ref())?;

    let chr_data = resolved_assets.as_ref().map(|a| a.chr_bank.as_slice());

    // Prepare Injections
    let mut injections: Vec<(u16, Vec<u8>)> = Vec::new();

    // 1. Palette Data at $E000
    let palette_data = if let Some(a) = &resolved_assets {
        let mut data = vec![0x0F; 32];
        let names = ["BG0", "BG1", "BG2", "BG3", "SP0", "SP1", "SP2", "SP3"];
        for (i, name) in names.iter().enumerate() {
            if let Some(pal) = a.palettes.iter().find(|p| p.name == *name) {
                let start_idx = i * 4;
                for (j, &color) in pal.colors.iter().enumerate() {
                    if start_idx + j < 32 {
                        data[start_idx + j] = color;
                    }
                }
            }
        }
        data
    } else {
        vec![0x0F; 32] // Default palette
    };
    injections.push((0xE000, palette_data));

    // 2. Period Table at $D000
    let period_table = audio::generate_period_table();
    injections.push((audio::PERIOD_TABLE_ADDR, period_table));

    // 3. Music Data at $D100
    let music_data =
        audio::compile_audio_data(&resolved_assets).map_err(|e| format!("Audio Error: {}", e))?;
    injections.push((audio::MUSIC_DATA_ADDR, music_data));

    // 3b. Sample Data
    let (samples, sample_table) =
        audio::compile_samples(&resolved_assets).map_err(|e| format!("Audio Error: {}", e))?;
    if !samples.is_empty() {
        injections.push((audio::SAMPLE_DATA_ADDR, samples));
    }
    injections.push((audio::SAMPLE_TABLE_ADDR, sample_table));

    // 3c. Envelope Data
    let envelope_data =
        audio::compile_envelopes(&resolved_assets).map_err(|e| format!("Audio Error: {}", e))?;
    injections.push((ENVELOPE_TABLE_ADDR, envelope_data));

    // 3d. SFX Data
    let sfx_data =
        audio::compile_sfx_data(&resolved_assets).map_err(|e| format!("Audio Error: {}", e))?;
    injections.push((audio::SFX_TABLE_ADDR, sfx_data));

    // 4. Nametable Data at $D500 (NAMETABLE_ADDR)
    // We only support one nametable for now (Nametable 0)
    if let Some(a) = &resolved_assets {
        if let Some(nt) = a.nametables.first() {
            // Nametable data is 960 bytes + 64 bytes attr = 1024 bytes
            // Check if data is valid length
            let mut nt_data = nt.data.clone();
            if nt_data.len() < 960 {
                nt_data.resize(960, 0);
            }
            let mut attr_data = nt.attrs.clone();
            if attr_data.len() < 64 {
                attr_data.resize(64, 0);
            }

            let mut full_nt = Vec::with_capacity(1024);
            full_nt.extend_from_slice(&nt_data[..960]);
            full_nt.extend_from_slice(&attr_data[..64]);

            injections.push((NAMETABLE_ADDR, full_nt));
        }
    }

    let (rom, layout) = assembler
        .assemble_with_layout(&asm_lines, chr_data, injections)
        .map_err(|e| format!("Assembler Error: {:?}", e))?;

    let mut map = SourceMap::from_layout(layout);
    map.sources = sources.into_inner();
    Ok((rom, map))
}

// Project API Handlers

pub async fn list_projects() -> impl IntoResponse {
    match project::list_projects() {
        Ok(projects) => Json(projects).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

// File Management APIs

pub async fn list_files(Path(name): Path<String>) -> impl IntoResponse {
    match project::list_files(&name) {
        Ok(files) => Json(files).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e).into_response(),
    }
}

#[derive(Deserialize)]
pub struct FileContent {
    content: String,
}

pub async fn get_file(Path((name, filename)): Path<(String, String)>) -> impl IntoResponse {
    match project::read_file(&name, &filename) {
        Ok(content) => (StatusCode::OK, content).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e).into_response(),
    }
}

pub async fn save_file(
    Path((name, filename)): Path<(String, String)>,
    Json(payload): Json<FileContent>,
) -> impl IntoResponse {
    match project::write_file(&name, &filename, &payload.content) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

#[derive(Deserialize)]
pub struct CreateFileRequest {
    filename: String,
    content: String,
}

pub async fn create_file(
    Path(name): Path<String>,
    Json(payload): Json<CreateFileRequest>,
) -> impl IntoResponse {
    match project::write_file(&name, &payload.filename, &payload.content) {
        Ok(_) => StatusCode::CREATED.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

pub async fn delete_file(Path((name, filename)): Path<(String, String)>) -> impl IntoResponse {
    match project::delete_file(&name, &filename) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

#[derive(Deserialize)]
pub struct CreateProjectRequest {
    name: String,
    source: Option<String>,
    assets: Option<ProjectAssets>,
    design: Option<GameDesign>,
}

pub async fn create_project(Json(payload): Json<CreateProjectRequest>) -> impl IntoResponse {
    match project::create_project_with_content(
        &payload.name,
        payload.source.as_deref(),
        payload.assets.as_ref(),
        payload.design.as_ref(),
    ) {
        Ok(_) => StatusCode::CREATED.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
    }
}

pub async fn get_project(Path(name): Path<String>) -> impl IntoResponse {
    match project::get_project(&name) {
        Ok(proj) => Json(proj).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e).into_response(),
    }
}

#[derive(Deserialize)]
pub struct SaveProjectRequest {
    source: Option<String>,
    assets: Option<project::ProjectAssets>,
    design: Option<GameDesign>,
}

pub async fn save_project(
    Path(name): Path<String>,
    Json(payload): Json<SaveProjectRequest>,
) -> impl IntoResponse {
    match project::save_project_with_design(
        &name,
        payload.source.as_deref(),
        payload.assets.as_ref(),
        payload.design.as_ref(),
    ) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
    }
}

#[derive(Deserialize)]
pub struct FlowTestRequest {
    #[serde(default = "default_test_frames")]
    frames: u16,
    test_id: Option<String>,
}
fn default_test_frames() -> u16 {
    120
}

/// Bounded production-core execution runs a generated harness without replacing custom source.
pub async fn test_game_flow(
    Path(name): Path<String>,
    Json(request): Json<FlowTestRequest>,
) -> impl IntoResponse {
    if request.frames == 0 || request.frames > 600 {
        return (StatusCode::BAD_REQUEST, "Smoke tests need 1..600 frames").into_response();
    }
    let result = tokio::task::spawn_blocking(move || -> Result<serde_json::Value, String> {
        let project = project::get_project(&name)?;
        let design = project.metadata.design.ok_or("Enable Game Studio before testing its flow")?;
        let selected_test = request.test_id.as_ref().map(|id| design.tests.iter().find(|test| test.id == *id).ok_or("Choose an existing saved flow test")).transpose()?;
        // Compile and report the same owned design/assets snapshot, even if another save arrives.
        let (rom, _) = compile_source_with_design(Some(MAIN_SOURCE.into()), Some(name), project.assets, Some(&design))?;
        let mut emulator = swiss_emulator::Emulator::new();
        emulator.load_rom(&rom).map_err(|error| format!("ROM load: {error:?}"))?;
        let boot_frames = if selected_test.is_some() { TEST_BOOT_FRAMES } else { request.frames };
        for _ in 0..boot_frames {
            emulator.step().map_err(|error| format!("ROM execution: {error:?}"))?;
            emulator.clear_audio_samples();
        }
        let mut state = flow_test_state(&emulator, &design)?;
        let mut observations = Vec::new();
        let mut total_frames = boot_frames;
        let mut failure = None;
        if let Some(test) = selected_test {
            for (index, step) in test.steps.iter().enumerate() {
                for button in 0..8 { emulator.set_button(0,button,step.buttons.iter().any(|key| key.button_index() == Some(button))); }
                for _ in 0..step.frames {
                    emulator.step().map_err(|error| format!("ROM execution: {error:?}"))?;
                    emulator.clear_audio_samples();
                }
                total_frames += step.frames;
                state = flow_test_state(&emulator, &design)?;
                let mismatch = if step.expected_scene.as_ref().is_some_and(|expected| state["scene_id"] != *expected) {
                    Some(format!("expected scene {}, got {}",step.expected_scene.as_ref().unwrap(),state["scene_id"]))
                } else if step.expected_x.is_some_and(|expected| state["x"] != expected) {
                    Some(format!("expected X {}, got {}",step.expected_x.unwrap(),state["x"]))
                } else if step.expected_y.is_some_and(|expected| state["y"] != expected) {
                    Some(format!("expected Y {}, got {}",step.expected_y.unwrap(),state["y"]))
                } else if step.expected_pickups.is_some_and(|expected| state["pickups"] != expected) {
                    Some(format!("expected pickups {}, got {}",step.expected_pickups.unwrap(),state["pickups"]))
                } else if step.expected_active_entities.is_some_and(|expected| state["active_entities"] != expected) {
                    Some(format!("expected active entities {}, got {}",step.expected_active_entities.unwrap(),state["active_entities"]))
                } else {
                    step.expected_variables.iter().find_map(|(id, expected)| {
                        (state["variables"][id] != *expected).then(|| format!("expected variable {id} = {expected}, got {}", state["variables"][id]))
                    })
                };
                observations.push(serde_json::json!({"step":index+1,"frames":total_frames,"state":state,"passed":mismatch.is_none()}));
                if let Some(message) = mismatch { failure = Some(format!("Step {}: {message}",index+1)); break; }
            }
        }
        state["passed"] = serde_json::json!(failure.is_none());
        state["message"] = serde_json::json!(failure);
        state["frames"] = serde_json::json!(total_frames);
        state["rom_bytes"] = serde_json::json!(rom.len());
        state["cpu_cycles"] = serde_json::json!(emulator.get_cpu_state().cycles);
        state["observations"] = serde_json::json!(observations);
        state["test_id"] = serde_json::json!(request.test_id);
        state["scope"] = serde_json::json!(if selected_test.is_some() {"saved controller scenario in generated flow; custom main excluded"} else {"generated flow boot; no controller input"});
        Ok(state)
    }).await;
    match result {
        Ok(Ok(report)) => Json(report).into_response(),
        Ok(Err(message)) => (StatusCode::BAD_REQUEST, message).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Smoke test failed: {error}"),
        )
            .into_response(),
    }
}

fn flow_test_state(
    emulator: &swiss_emulator::Emulator,
    design: &GameDesign,
) -> Result<serde_json::Value, String> {
    // Only the isolated harness fixes this declaration layout; custom source may allocate differently.
    let ram = emulator.ram_snapshot();
    let scene = design
        .scenes
        .get(usize::from(ram[0x5c0]))
        .ok_or("Runtime selected an invalid scene")?;
    if ram[0x5c8] != 255 {
        return Err("Runtime did not load the selected world map".into());
    }
    let address = design.pickups_address();
    let pickups = address
        .map(|offset| u16::from_le_bytes([ram[offset], ram[offset + 1]]))
        .unwrap_or(0);
    let entities: Vec<_> = scene
        .instances
        .iter()
        .enumerate()
        .map(|(slot, instance)| {
            let offset = address.unwrap() + 3 + slot * 9;
            serde_json::json!({"id":instance.id,"entity":instance.entity,"active":ram[offset] != 0,
            "x":u16::from_le_bytes([ram[offset+1],ram[offset+2]]),"y":instance.y})
        })
        .collect();
    let active = entities
        .iter()
        .filter(|entity| entity["active"] == true)
        .count();
    let variables: serde_json::Map<String, serde_json::Value> = design
        .variables
        .iter()
        .map(|variable| {
            let offset = design.variable_address(&variable.id).unwrap();
            (
                variable.id.clone(),
                serde_json::json!(u16::from_le_bytes([ram[offset], ram[offset + 1]])),
            )
        })
        .collect();
    Ok(
        serde_json::json!({"scene_id":scene.id,"scene_name":scene.name,
        "x":u16::from_le_bytes([ram[0x5c3],ram[0x5c4]]),"y":u16::from_le_bytes([ram[0x5c5],ram[0x5c6]]),
        "pickups":pickups,"active_entities":active,"entities":entities,"variables":variables,"generated_ram_bytes":design.generated_ram_bytes()}),
    )
}

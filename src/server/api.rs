use crate::compiler::{
    analysis::SemanticAnalyzer,
    assembler::Assembler,
    audio,
    codegen::{CodeGenerator, NAMETABLE_ADDR},
    lexer::Lexer,
    parser::Parser,
};
use crate::server::project::{self, ProjectAssets};
use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CompileRequest {
    source: String,
    assets: Option<ProjectAssets>,
}

pub async fn compile(Json(payload): Json<CompileRequest>) -> impl IntoResponse {
    // Spawn a blocking task for the CPU-intensive compilation process
    let result =
        tokio::task::spawn_blocking(move || compile_source(&payload.source, payload.assets)).await;

    match result {
        Ok(compile_result) => match compile_result {
            Ok(rom_data) => {
                // Return the binary data
                Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "application/octet-stream")
                    .header("Content-Disposition", "attachment; filename=\"game.nes\"")
                    .body(axum::body::Body::from(rom_data))
                    .unwrap()
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

fn compile_source(source: &str, assets: Option<ProjectAssets>) -> Result<Vec<u8>, String> {
    // 1. Lexing
    let mut lexer = Lexer::new(source);
    let tokens = lexer
        .tokenize()
        .map_err(|e| format!("Lexer Error: {:?}", e))?;

    // 2. Parsing
    let mut parser = Parser::new(tokens);
    let program = parser
        .parse()
        .map_err(|e| format!("Parser Error: {:?}", e))?;

    // 3. Analysis
    let mut analyzer = SemanticAnalyzer::new();
    analyzer
        .analyze(&program)
        .map_err(|e| format!("Analysis Error: {:?}", e))?;

    // 4. Codegen
    let symbol_table = analyzer.symbol_table;

    // Create CodeGenerator (reverted signature)
    let mut codegen = CodeGenerator::new(symbol_table);
    let asm_lines = codegen
        .generate(&program)
        .map_err(|e| format!("Codegen Error: {:?}", e))?;
    let asm_source = asm_lines.join("\n");

    // 5. Assembler
    let assembler = Assembler::new();

    let chr_data = assets.as_ref().map(|a| a.chr_bank.as_slice());

    // Prepare Injections
    let mut injections: Vec<(u16, Vec<u8>)> = Vec::new();

    // 1. Palette Data at $E000
    let palette_data = if let Some(a) = &assets {
        let mut data = vec![0x0F; 32];
        for (i, pal) in a.palettes.iter().take(8).enumerate() {
            let start_idx = i * 4;
            for (j, &color) in pal.colors.iter().enumerate() {
                if start_idx + j < 32 {
                    data[start_idx + j] = color;
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
    let music_data = audio::compile_audio_data(&assets);
    injections.push((audio::MUSIC_DATA_ADDR, music_data));

    // 4. Nametable Data at $D500 (NAMETABLE_ADDR)
    // We only support one nametable for now (Nametable 0)
    if let Some(a) = &assets {
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

    let rom = assembler
        .assemble(&asm_source, chr_data, injections)
        .map_err(|e| format!("Assembler Error: {:?}", e))?;

    Ok(rom)
}

// Project API Handlers

pub async fn list_projects() -> impl IntoResponse {
    match project::list_projects() {
        Ok(projects) => Json(projects).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

#[derive(Deserialize)]
pub struct CreateProjectRequest {
    name: String,
}

pub async fn create_project(Json(payload): Json<CreateProjectRequest>) -> impl IntoResponse {
    match project::create_project(&payload.name) {
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
}

pub async fn save_project(
    Path(name): Path<String>,
    Json(payload): Json<SaveProjectRequest>,
) -> impl IntoResponse {
    match project::save_project(&name, payload.source.as_deref(), payload.assets.as_ref()) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

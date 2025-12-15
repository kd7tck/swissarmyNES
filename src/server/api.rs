use crate::compiler::{
    analysis::SemanticAnalyzer, assembler::Assembler, codegen::CodeGenerator, lexer::Lexer,
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

    // Prepare Palette Data (32 bytes flattened)
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
        Some(data)
    } else {
        None
    };

    let rom = assembler
        .assemble(&asm_source, chr_data, palette_data.as_deref())
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

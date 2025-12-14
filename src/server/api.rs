use crate::compiler::{
    analysis::SemanticAnalyzer, assembler::Assembler, codegen::CodeGenerator, lexer::Lexer,
    parser::Parser,
};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub async fn compile(body: String) -> impl IntoResponse {
    // Spawn a blocking task for the CPU-intensive compilation process
    let result = tokio::task::spawn_blocking(move || compile_source(&body)).await;

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

fn compile_source(source: &str) -> Result<Vec<u8>, String> {
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
    let mut codegen = CodeGenerator::new(symbol_table);
    let asm_lines = codegen
        .generate(&program)
        .map_err(|e| format!("Codegen Error: {:?}", e))?;
    let asm_source = asm_lines.join("\n");

    // 5. Assembler
    let assembler = Assembler::new();
    let rom = assembler
        .assemble(&asm_source)
        .map_err(|e| format!("Assembler Error: {:?}", e))?;

    Ok(rom)
}

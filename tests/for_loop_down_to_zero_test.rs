use std::process::Command;
use swissarmynes::compiler::analysis::SemanticAnalyzer;
use swissarmynes::compiler::assembler::Assembler;
use swissarmynes::compiler::codegen::CodeGenerator;
use swissarmynes::compiler::lexer::Lexer;
use swissarmynes::compiler::parser::Parser;

#[test]
fn test_for_loop_byte_down_to_zero() {
    let source = "
        DIM i AS BYTE
        DIM count AS BYTE

        SUB Main()
            LET count = 0
            FOR i = 10 TO 0 STEP -1
                LET count = count + 1
            NEXT i
        END SUB
    ";

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Lexing failed");

    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parsing failed");

    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&program).expect("Analysis failed");

    let mut cg = CodeGenerator::new(analyzer.symbol_table);
    let (asm_lines, _) = cg.generate(&program).expect("Codegen failed");

    let assembler = Assembler::new();
    let rom = assembler
        .assemble(&asm_lines, None, vec![])
        .expect("Assembly failed");

    std::fs::write("/tmp/test_for_down_to_zero.nes", &rom).unwrap();

    // count is second variable in RAM -> $05C1
    let output = Command::new("python3")
        .args(&[
            "/tmp/file_attachments/out/rom_harness.py",
            "/tmp/test_for_down_to_zero.nes",
            "--at",
            "0x05C1",
            "--expect",
            "11",
        ])
        .output()
        .expect("Failed to execute rom_harness.py");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("Harness stdout: {}\nHarness stderr: {}", stdout, stderr);

    assert!(
        output.status.success(),
        "rom_harness execution failed (loop hung or count mismatch). Output: {}",
        stdout
    );
}

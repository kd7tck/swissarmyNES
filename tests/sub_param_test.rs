use std::process::Command;
use swissarmynes::compiler::analysis::SemanticAnalyzer;
use swissarmynes::compiler::assembler::Assembler;
use swissarmynes::compiler::codegen::CodeGenerator;
use swissarmynes::compiler::lexer::Lexer;
use swissarmynes::compiler::parser::Parser;

#[test]
fn test_sub_parameter_codegen_and_body_access() {
    let source = "
        DIM res AS BYTE

        SUB AddTen(v AS BYTE)
            LET res = v + 10
        END SUB

        SUB Main()
            CALL AddTen(5)
        END SUB
    ";

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Lexing failed");

    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parsing failed");

    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&program).expect("Analysis failed");

    let mut cg = CodeGenerator::new(analyzer.symbol_table);
    let (asm_lines, _) = cg
        .generate(&program)
        .expect("Codegen failed for sub parameter access");
    let asm_code = asm_lines.join("\n");

    assert!(asm_code.contains("AddTen:"));

    let assembler = Assembler::new();
    let rom = assembler
        .assemble(&asm_lines, None, vec![])
        .expect("Assembly failed");

    std::fs::write("/tmp/test_sub_param.nes", &rom).unwrap();

    let output = Command::new("python3")
        .args(&[
            "/tmp/file_attachments/out/rom_harness.py",
            "/tmp/test_sub_param.nes",
            "--at",
            "0x05C0",
            "--expect",
            "15",
        ])
        .output()
        .expect("Failed to execute rom_harness.py");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("Harness stdout: {}\nHarness stderr: {}", stdout, stderr);

    assert!(
        output.status.success(),
        "rom_harness execution failed. Output: {}",
        stdout
    );
}

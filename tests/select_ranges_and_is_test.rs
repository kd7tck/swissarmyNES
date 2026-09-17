use std::process::Command;
use swissarmynes::compiler::analysis::SemanticAnalyzer;
use swissarmynes::compiler::assembler::Assembler;
use swissarmynes::compiler::codegen::CodeGenerator;
use swissarmynes::compiler::lexer::Lexer;
use swissarmynes::compiler::parser::Parser;

#[test]
fn test_select_case_ranges_and_is_and_multiple_conditions() {
    let source = "
        DIM val AS BYTE
        DIM res1 AS BYTE
        DIM res2 AS BYTE
        DIM res3 AS BYTE

        SUB Main()
            LET val = 5
            SELECT CASE val
                CASE 1 TO 10
                    LET res1 = 100
                CASE ELSE
                    LET res1 = 0
            END SELECT

            LET val = 25
            SELECT CASE val
                CASE IS > 20
                    LET res2 = 200
                CASE ELSE
                    LET res2 = 0
            END SELECT

            LET val = 3
            SELECT CASE val
                CASE 1, 2, 3
                    LET res3 = 99
                CASE ELSE
                    LET res3 = 0
            END SELECT
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

    std::fs::write("/tmp/test_select_ranges.nes", &rom).unwrap();

    // val = $05C0, res1 = $05C1, res2 = $05C2, res3 = $05C3
    let output = Command::new("python3")
        .args(&[
            "/tmp/file_attachments/out/rom_harness.py",
            "/tmp/test_select_ranges.nes",
            "--at",
            "0x05C1",
            "--expect",
            "100",
        ])
        .output()
        .expect("Failed to execute rom_harness.py");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "res1 failed: {}", stdout);

    let output2 = Command::new("python3")
        .args(&[
            "/tmp/file_attachments/out/rom_harness.py",
            "/tmp/test_select_ranges.nes",
            "--at",
            "0x05C2",
            "--expect",
            "200",
        ])
        .output()
        .expect("Failed to execute rom_harness.py");

    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    assert!(output2.status.success(), "res2 failed: {}", stdout2);

    let output3 = Command::new("python3")
        .args(&[
            "/tmp/file_attachments/out/rom_harness.py",
            "/tmp/test_select_ranges.nes",
            "--at",
            "0x05C3",
            "--expect",
            "99",
        ])
        .output()
        .expect("Failed to execute rom_harness.py");

    let stdout3 = String::from_utf8_lossy(&output3.stdout);
    assert!(output3.status.success(), "res3 failed: {}", stdout3);
}

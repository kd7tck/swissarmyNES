use swissarmynes::compiler::ast::{DataType, Expression, Program, Statement, TopLevel};
use swissarmynes::compiler::codegen::CodeGenerator;
use swissarmynes::compiler::symbol_table::{SymbolKind, SymbolTable};

#[test]
fn test_codegen_for_word_loop() {
    let mut st = SymbolTable::new();
    st.define("w".to_string(), DataType::Word, SymbolKind::Variable)
        .unwrap();

    // FOR w = 1000 TO 2000 STEP 10
    let program = Program {
        declarations: vec![
            TopLevel::Dim("w".to_string(), DataType::Word, None),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![Statement::For(
                    "w".to_string(),
                    Expression::Integer(1000),
                    Expression::Integer(2000),
                    Some(Expression::Integer(10)),
                    vec![
                        // Body doesn't matter for this test
                    ],
                )],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let code = cg.generate(&program).expect("Codegen failed");
    let code_str = code.join("\n");

    // Verification
    // 1. Initialization: Should be 16-bit store
    // 1000 = 0x03E8. Low: E8, High: 03
    assert!(code_str.contains("LDA #$E8"));
    assert!(code_str.contains("LDA #$03"));

    // 2. Condition Check: Should handle 16-bit comparison
    // Logic should likely synthesize a `w <= 2000` check.
    // This typically involves CPX/CMP or helper calls, but importantly, it shouldn't just be `CMP $00` (8-bit)
    // The simplified For loop implementation we plan will use `generate_expression` for `w <= 2000`.
    // That generates a boolean in A ($00 or $FF).

    // 3. Increment: Should be 16-bit addition
    // `w = w + 10`
}

#[test]
fn test_codegen_for_int_signed_loop() {
    let mut st = SymbolTable::new();
    st.define("i".to_string(), DataType::Int, SymbolKind::Variable)
        .unwrap();

    // FOR i = -10 TO 10
    let program = Program {
        declarations: vec![
            TopLevel::Dim("i".to_string(), DataType::Int, None),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![Statement::For(
                    "i".to_string(),
                    Expression::Integer(-10), // -10 is represented as a signed Int literal
                    Expression::Integer(10),
                    None,
                    vec![],
                )],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let code = cg.generate(&program).expect("Codegen failed");
    let code_str = code.join("\n");

    // -10 = 0xF6 (8-bit signed)
    assert!(code_str.contains("LDA #$F6"));
}

use swissarmynes::compiler::ast::{DataType, Expression, Program, Statement, TopLevel};
use swissarmynes::compiler::codegen::CodeGenerator;
use swissarmynes::compiler::symbol_table::{SymbolKind, SymbolTable};

#[test]
fn test_codegen_select_statement() {
    let mut st = SymbolTable::new();
    st.define("x".to_string(), DataType::Byte, SymbolKind::Variable)
        .unwrap();

    // SELECT CASE x
    //   CASE 1
    //     x = 10
    //   CASE 2
    //     x = 20
    //   CASE ELSE
    //     x = 0
    // END SELECT
    let program = Program {
        declarations: vec![
            TopLevel::Dim("x".to_string(), DataType::Byte, None),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![Statement::Select(
                    Expression::Identifier("x".to_string()),
                    vec![
                        (
                            Expression::Integer(1),
                            vec![Statement::Let(
                                Expression::Identifier("x".to_string()),
                                Expression::Integer(10),
                            )],
                        ),
                        (
                            Expression::Integer(2),
                            vec![Statement::Let(
                                Expression::Identifier("x".to_string()),
                                Expression::Integer(20),
                            )],
                        ),
                    ],
                    Some(vec![Statement::Let(
                        Expression::Identifier("x".to_string()),
                        Expression::Integer(0),
                    )]),
                )],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let code = cg.generate(&program).expect("Codegen failed");
    let code_str = code.join("\n");

    // Check stack usage (Push Select Value)
    assert!(code_str.contains("PHA")); // Push Byte

    // Check Stack Peek (Byte)
    assert!(code_str.contains("TSX"));
    assert!(code_str.contains("CMP $0101, X"));

    // Check Branching
    assert!(code_str.contains("BNE GEN_L"));
    assert!(code_str.contains("JMP GEN_L"));

    // Check Case Values
    assert!(code_str.contains("LDA #$01")); // Case 1
    assert!(code_str.contains("LDA #$02")); // Case 2

    // Check Case Bodies
    assert!(code_str.contains("LDA #$0A")); // 10
    assert!(code_str.contains("LDA #$14")); // 20
    assert!(code_str.contains("LDA #$00")); // 0 (Else)

    // Check Cleanup
    assert!(code_str.contains("PLA"));
}

#[test]
fn test_codegen_select_word() {
    let mut st = SymbolTable::new();
    st.define("w".to_string(), DataType::Word, SymbolKind::Variable)
        .unwrap();

    // SELECT CASE w
    //   CASE 1000
    //     w = 1
    // END SELECT
    let program = Program {
        declarations: vec![
            TopLevel::Dim("w".to_string(), DataType::Word, None),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![Statement::Select(
                    Expression::Identifier("w".to_string()),
                    vec![(
                        Expression::Integer(1000),
                        vec![Statement::Let(
                            Expression::Identifier("w".to_string()),
                            Expression::Integer(1),
                        )],
                    )],
                    None,
                )],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let code = cg.generate(&program).expect("Codegen failed");
    let code_str = code.join("\n");

    // Check stack usage (Push Word)
    assert!(code_str.contains("PHA"));
    assert!(code_str.contains("TXA"));
    // Stack structure: Low, High (Top)

    // Check Stack Peek (Word)
    assert!(code_str.contains("TSX"));
    assert!(code_str.contains("CMP $0102, X")); // Compare Low
    assert!(code_str.contains("CMP $0101, X")); // Compare High

    // Check Case Value (1000 = $03E8)
    assert!(code_str.contains("LDA #$E8"));
    assert!(code_str.contains("LDX #$03"));
}

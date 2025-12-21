use swissarmynes::compiler::ast::{
    BinaryOperator, DataType, Expression, Program, Statement, TopLevel,
};
use swissarmynes::compiler::codegen::CodeGenerator;
use swissarmynes::compiler::symbol_table::{SymbolKind, SymbolTable};

#[test]
fn test_codegen_if_statement() {
    let mut st = SymbolTable::new();
    st.define("x".to_string(), DataType::Byte, SymbolKind::Variable)
        .unwrap();

    // IF x = 10 THEN x = 20 ELSE x = 30
    let program = Program {
        declarations: vec![
            TopLevel::Dim("x".to_string(), DataType::Byte, None),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![Statement::If(
                    Expression::BinaryOp(
                        Box::new(Expression::Identifier("x".to_string())),
                        BinaryOperator::Equal,
                        Box::new(Expression::Integer(10)),
                    ),
                    vec![Statement::Let(
                        Expression::Identifier("x".to_string()),
                        Expression::Integer(20),
                    )],
                    Some(vec![Statement::Let(
                        Expression::Identifier("x".to_string()),
                        Expression::Integer(30),
                    )]),
                )],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let code = cg.generate(&program).expect("Codegen failed");
    let code_str = code.join("\n");

    // Check for branching instructions
    assert!(code_str.contains("CMP #0"));
    assert!(code_str.contains("BEQ GEN_L")); // Should jump to Else or End
    assert!(code_str.contains("JMP GEN_L")); // Jump to End
}

#[test]
fn test_codegen_while_statement() {
    let mut st = SymbolTable::new();
    st.define("x".to_string(), DataType::Byte, SymbolKind::Variable)
        .unwrap();

    // WHILE x < 10 ...
    let program = Program {
        declarations: vec![
            TopLevel::Dim("x".to_string(), DataType::Byte, None),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![Statement::While(
                    Expression::BinaryOp(
                        Box::new(Expression::Identifier("x".to_string())),
                        BinaryOperator::LessThan,
                        Box::new(Expression::Integer(10)),
                    ),
                    vec![Statement::Let(
                        Expression::Identifier("x".to_string()),
                        Expression::BinaryOp(
                            Box::new(Expression::Identifier("x".to_string())),
                            BinaryOperator::Add,
                            Box::new(Expression::Integer(1)),
                        ),
                    )],
                )],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let code = cg.generate(&program).expect("Codegen failed");
    let code_str = code.join("\n");

    assert!(code_str.contains("GEN_L1:")); // Start label (assuming 1 is first)
    assert!(code_str.contains("CMP #0"));
    assert!(code_str.contains("BEQ GEN_L")); // Exit loop
    assert!(code_str.contains("JMP GEN_L")); // Loop back
}

#[test]
fn test_codegen_for_statement() {
    let mut st = SymbolTable::new();
    st.define("i".to_string(), DataType::Byte, SymbolKind::Variable)
        .unwrap();

    // FOR i = 0 TO 10 ...
    let program = Program {
        declarations: vec![
            TopLevel::Dim("i".to_string(), DataType::Byte, None),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![Statement::For(
                    "i".to_string(),
                    Expression::Integer(0),
                    Expression::Integer(10),
                    None,
                    vec![Statement::Let(
                        Expression::Identifier("i".to_string()),
                        Expression::Identifier("i".to_string()),
                    )],
                )],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let code = cg.generate(&program).expect("Codegen failed");
    let code_str = code.join("\n");

    // Loop logic verification
    assert!(code_str.contains("STA $0490")); // Init i
    assert!(code_str.contains("CMP $00")); // Compare with Limit (temp)
}

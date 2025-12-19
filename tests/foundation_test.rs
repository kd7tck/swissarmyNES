use swissarmynes::compiler::ast::{
    BinaryOperator, DataType, Expression, Program, Statement, TopLevel, UnaryOperator,
};
use swissarmynes::compiler::codegen::CodeGenerator;
use swissarmynes::compiler::symbol_table::{SymbolKind, SymbolTable};

#[test]
fn test_bitwise_8bit_generation() {
    let mut st = SymbolTable::new();
    st.define("a".to_string(), DataType::Byte, SymbolKind::Variable)
        .unwrap();
    st.define("b".to_string(), DataType::Byte, SymbolKind::Variable)
        .unwrap();

    let program = Program {
        declarations: vec![
            TopLevel::Dim("a".to_string(), DataType::Byte, None),
            TopLevel::Dim("b".to_string(), DataType::Byte, None),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![
                    Statement::Let(
                        Expression::Identifier("a".to_string()),
                        Expression::BinaryOp(
                            Box::new(Expression::Identifier("a".to_string())),
                            BinaryOperator::And,
                            Box::new(Expression::Identifier("b".to_string())),
                        ),
                    ),
                    Statement::Let(
                        Expression::Identifier("a".to_string()),
                        Expression::BinaryOp(
                            Box::new(Expression::Identifier("a".to_string())),
                            BinaryOperator::Or,
                            Box::new(Expression::Identifier("b".to_string())),
                        ),
                    ),
                    Statement::Let(
                        Expression::Identifier("a".to_string()),
                        Expression::BinaryOp(
                            Box::new(Expression::Identifier("a".to_string())),
                            BinaryOperator::Xor,
                            Box::new(Expression::Identifier("b".to_string())),
                        ),
                    ),
                ],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let code = cg.generate(&program).expect("Codegen failed");

    // Check AND
    assert!(code.iter().any(|line| line.contains("AND $00"))); // 8-bit AND
                                                               // Check OR
    assert!(code.iter().any(|line| line.contains("ORA $00"))); // 8-bit OR
                                                               // Check XOR
    assert!(code.iter().any(|line| line.contains("EOR $00"))); // 8-bit XOR
}

#[test]
fn test_unary_ops() {
    let mut st = SymbolTable::new();
    st.define("i".to_string(), DataType::Int, SymbolKind::Variable)
        .unwrap();
    st.define("b".to_string(), DataType::Byte, SymbolKind::Variable)
        .unwrap();

    let program = Program {
        declarations: vec![
            TopLevel::Dim("i".to_string(), DataType::Int, None),
            TopLevel::Dim("b".to_string(), DataType::Byte, None),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![
                    // i = -i
                    Statement::Let(
                        Expression::Identifier("i".to_string()),
                        Expression::UnaryOp(
                            UnaryOperator::Negate,
                            Box::new(Expression::Identifier("i".to_string())),
                        ),
                    ),
                    // b = NOT b
                    Statement::Let(
                        Expression::Identifier("b".to_string()),
                        Expression::UnaryOp(
                            UnaryOperator::Not,
                            Box::new(Expression::Identifier("b".to_string())),
                        ),
                    ),
                ],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let code = cg.generate(&program).expect("Codegen failed");

    // Negate: EOR #$FF, ADC #1 (Two's complement)
    let negate_found = code
        .windows(3)
        .any(|w| w[0].contains("EOR #$FF") && w[1].contains("CLC") && w[2].contains("ADC #1"));
    // Not: EOR #$FF (One's complement)
    let not_found = code.iter().any(|line| line.contains("EOR #$FF"));

    assert!(negate_found, "Negate code not found");
    assert!(not_found, "Not code not found");
}

#[test]
fn test_peek_generation() {
    let mut st = SymbolTable::new();
    st.define("b".to_string(), DataType::Byte, SymbolKind::Variable)
        .unwrap();

    let program = Program {
        declarations: vec![
            TopLevel::Dim("b".to_string(), DataType::Byte, None),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![
                    // b = PEEK($2002)
                    Statement::Let(
                        Expression::Identifier("b".to_string()),
                        Expression::Peek(Box::new(Expression::Integer(0x2002))),
                    ),
                    // b = PEEK(b + 1)
                    Statement::Let(
                        Expression::Identifier("b".to_string()),
                        Expression::Peek(Box::new(Expression::BinaryOp(
                            Box::new(Expression::Identifier("b".to_string())),
                            BinaryOperator::Add,
                            Box::new(Expression::Integer(1)),
                        ))),
                    ),
                ],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let code = cg.generate(&program).expect("Codegen failed");

    // Static PEEK
    assert!(code.iter().any(|line| line.contains("LDA $2002")));

    // Dynamic PEEK (should use indirect $02)
    assert!(code.iter().any(|line| line.contains("STA $02")));
    assert!(code.iter().any(|line| line.contains("LDA ($02),Y")));
}

#[test]
fn test_math_builtins() {
    let mut st = SymbolTable::new();
    st.define("i".to_string(), DataType::Int, SymbolKind::Variable)
        .unwrap();

    let program = Program {
        declarations: vec![
            TopLevel::Dim("i".to_string(), DataType::Int, None),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![
                    Statement::Let(
                        Expression::Identifier("i".to_string()),
                        Expression::Call(
                            Box::new(Expression::Identifier("ABS".to_string())),
                            vec![Expression::Identifier("i".to_string())],
                        ),
                    ),
                    Statement::Let(
                        Expression::Identifier("i".to_string()),
                        Expression::Call(
                            Box::new(Expression::Identifier("SGN".to_string())),
                            vec![Expression::Identifier("i".to_string())],
                        ),
                    ),
                ],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let code = cg.generate(&program).expect("Codegen failed");

    // ABS check: BCC (skip negate if positive/unsigned)
    assert!(code.iter().any(|line| line.contains("BCC")));
    // SGN check: BPL, BCS logic
    // We check if it generates logic to handle negative numbers (-1 -> FF)
    assert!(code.iter().any(|line| line.contains("LDA #$FF")));
    assert!(code.iter().any(|line| line.contains("LDA #1")));
}

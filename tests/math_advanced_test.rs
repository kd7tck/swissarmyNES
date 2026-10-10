use swissarmynes::compiler::ast::{
    BinaryOperator, DataType, Expression, Program, Statement, TopLevel,
};
use swissarmynes::compiler::codegen::CodeGenerator;
use swissarmynes::compiler::symbol_table::SymbolTable;

#[test]
fn test_mul_16_generation() {
    let mut st = SymbolTable::new();
    st.define(
        "w".to_string(),
        DataType::Word,
        swissarmynes::compiler::symbol_table::SymbolKind::Variable,
    )
    .unwrap();

    let program = Program {
        declarations: vec![
            TopLevel::Dim("w".to_string(), DataType::Word, None),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![Statement::Let(
                    Expression::Identifier("w".to_string()),
                    Expression::BinaryOp(
                        Box::new(Expression::Integer(200)),
                        BinaryOperator::Multiply,
                        Box::new(Expression::Integer(50)),
                    ),
                )],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let (code, _) = cg.generate(&program).expect("Codegen failed");

    // Check for Math_Mul16 usage
    assert!(code.iter().any(|line| line.contains("JSR Math_Mul16")));
    // Check that Math_Mul16 is defined
    assert!(code.iter().any(|line| line.contains("Math_Mul16:")));
}

#[test]
fn test_div_16_generation() {
    let mut st = SymbolTable::new();
    st.define(
        "w".to_string(),
        DataType::Word,
        swissarmynes::compiler::symbol_table::SymbolKind::Variable,
    )
    .unwrap();

    let program = Program {
        declarations: vec![
            TopLevel::Dim("w".to_string(), DataType::Word, None),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![Statement::Let(
                    Expression::Identifier("w".to_string()),
                    Expression::BinaryOp(
                        Box::new(Expression::Integer(10000)),
                        BinaryOperator::Divide,
                        Box::new(Expression::Integer(50)),
                    ),
                )],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let (code, _) = cg.generate(&program).expect("Codegen failed");

    assert!(code.iter().any(|line| line.contains("JSR Math_Div16")));
    assert!(code.iter().any(|line| line.contains("Math_Div16:")));
}

#[test]
fn test_div_16_signed_generation() {
    let mut st = SymbolTable::new();
    st.define(
        "i".to_string(),
        DataType::Int,
        swissarmynes::compiler::symbol_table::SymbolKind::Variable,
    )
    .unwrap();

    let program = Program {
        declarations: vec![
            TopLevel::Dim("i".to_string(), DataType::Int, None),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![Statement::Let(
                    Expression::Identifier("i".to_string()),
                    Expression::BinaryOp(
                        Box::new(Expression::Identifier("i".to_string())),
                        BinaryOperator::Divide,
                        Box::new(Expression::Integer(-2)),
                    ),
                )],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let (code, _) = cg.generate(&program).expect("Codegen failed");

    assert!(code
        .iter()
        .any(|line| line.contains("JSR Math_Div16_Signed")));
}

#[test]
fn test_mod_generation() {
    let mut st = SymbolTable::new();
    st.define(
        "w".to_string(),
        DataType::Word,
        swissarmynes::compiler::symbol_table::SymbolKind::Variable,
    )
    .unwrap();

    let program = Program {
        declarations: vec![
            TopLevel::Dim("w".to_string(), DataType::Word, None),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![Statement::Let(
                    Expression::Identifier("w".to_string()),
                    Expression::BinaryOp(
                        Box::new(Expression::Integer(10)),
                        BinaryOperator::Modulo,
                        Box::new(Expression::Integer(3)),
                    ),
                )],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let (code, _) = cg.generate(&program).expect("Codegen failed");

    assert!(code.iter().any(|line| line.contains("JSR Math_Div16")));
    // Check it reads Remainder from $08/$09
    assert!(code.iter().any(|line| line.contains("LDA $08")));
    assert!(code.iter().any(|line| line.contains("LDX $09")));
}

#[test]
fn test_signed_comparison_generation() {
    let mut st = SymbolTable::new();
    st.define(
        "i".to_string(),
        DataType::Int,
        swissarmynes::compiler::symbol_table::SymbolKind::Variable,
    )
    .unwrap();

    let program = Program {
        declarations: vec![
            TopLevel::Dim("i".to_string(), DataType::Int, None),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![Statement::If(
                    Expression::BinaryOp(
                        Box::new(Expression::Identifier("i".to_string())),
                        BinaryOperator::LessThan,
                        Box::new(Expression::Integer(10)),
                    ),
                    vec![Statement::Let(
                        Expression::Identifier("i".to_string()),
                        Expression::Integer(1),
                    )],
                    None,
                )],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let (code, _) = cg.generate(&program).expect("Codegen failed");

    // Check for Signed logic (BVS)
    // LessThan logic: BVS overflow_lbl, BMI true_lbl...
    // We expect BVS to be generated
    assert!(code.iter().any(|line| line.contains("BVS")));
    // We expect comparison instructions
    assert!(code.iter().any(|line| line.contains("SEC")));
    assert!(code.iter().any(|line| line.contains("SBC $00")));
}

#[test]
fn test_mixed_comparison_promotion() {
    // Comparison between Int and Byte should promote to Signed Word
    let mut st = SymbolTable::new();
    st.define(
        "i".to_string(),
        DataType::Int,
        swissarmynes::compiler::symbol_table::SymbolKind::Variable,
    )
    .unwrap();
    st.define(
        "b".to_string(),
        DataType::Byte,
        swissarmynes::compiler::symbol_table::SymbolKind::Variable,
    )
    .unwrap();

    let program = Program {
        declarations: vec![
            TopLevel::Dim("i".to_string(), DataType::Int, None),
            TopLevel::Dim("b".to_string(), DataType::Byte, None),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![Statement::If(
                    Expression::BinaryOp(
                        Box::new(Expression::Identifier("i".to_string())),
                        BinaryOperator::LessThan,
                        Box::new(Expression::Identifier("b".to_string())),
                    ),
                    vec![],
                    None,
                )],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let (code, _) = cg.generate(&program).expect("Codegen failed");

    // Check for Signed logic (BVS) because Int is present
    assert!(code.iter().any(|line| line.contains("BVS")));
}

#[test]
fn test_math_min_max_generation() {
    let mut st = SymbolTable::new();
    st.define(
        "w".to_string(),
        DataType::Word,
        swissarmynes::compiler::symbol_table::SymbolKind::Variable,
    )
    .unwrap();

    let program = Program {
        declarations: vec![
            TopLevel::Dim("w".to_string(), DataType::Word, None),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![
                    Statement::Let(
                        Expression::Identifier("w".to_string()),
                        Expression::Call(
                            Box::new(Expression::MemberAccess(
                                Box::new(Expression::Identifier("Math".to_string())),
                                "Min".to_string(),
                            )),
                            vec![Expression::Integer(100), Expression::Integer(500)],
                        ),
                    ),
                    Statement::Let(
                        Expression::Identifier("w".to_string()),
                        Expression::Call(
                            Box::new(Expression::MemberAccess(
                                Box::new(Expression::Identifier("Math".to_string())),
                                "Max".to_string(),
                            )),
                            vec![Expression::Integer(100), Expression::Integer(500)],
                        ),
                    ),
                ],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let (code, _) = cg.generate(&program).expect("Codegen failed");

    // Check that comparisons are generated for Min and Max
    assert!(code
        .iter()
        .any(|line| line.contains("SBC $00") || line.contains("SBC $01")));
}

#[test]
fn test_math_clamp_generation() {
    let mut st = SymbolTable::new();
    st.define(
        "b".to_string(),
        DataType::Byte,
        swissarmynes::compiler::symbol_table::SymbolKind::Variable,
    )
    .unwrap();
    st.define(
        "w".to_string(),
        DataType::Word,
        swissarmynes::compiler::symbol_table::SymbolKind::Variable,
    )
    .unwrap();

    let program = Program {
        declarations: vec![
            TopLevel::Dim("b".to_string(), DataType::Byte, None),
            TopLevel::Dim("w".to_string(), DataType::Word, None),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![
                    Statement::Let(
                        Expression::Identifier("b".to_string()),
                        Expression::Call(
                            Box::new(Expression::MemberAccess(
                                Box::new(Expression::Identifier("Math".to_string())),
                                "Clamp".to_string(),
                            )),
                            vec![
                                Expression::Integer(150),
                                Expression::Integer(10),
                                Expression::Integer(100),
                            ],
                        ),
                    ),
                    Statement::Let(
                        Expression::Identifier("w".to_string()),
                        Expression::Call(
                            Box::new(Expression::MemberAccess(
                                Box::new(Expression::Identifier("Math".to_string())),
                                "Clamp".to_string(),
                            )),
                            vec![
                                Expression::Integer(500),
                                Expression::Integer(100),
                                Expression::Integer(1000),
                            ],
                        ),
                    ),
                ],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let (code, _) = cg.generate(&program).expect("Codegen failed");

    // Literal bounds promote these calls to WORD; verify both actual bounds
    // rather than incidental CMP instructions in unused runtime helpers.
    assert!(code.iter().any(|line| line.contains("SBC $04")));
    assert!(code.iter().any(|line| line.contains("SBC $06")));
}

#[test]
fn clamp_executes_byte_word_signed_and_nested_boundaries() {
    use swiss_emulator::Emulator;
    use swissarmynes::server::api::compile_source;
    let source = r#"
        DIM b AS BYTE
        DIM low AS BYTE
        DIM high AS BYTE
        DIM values(8) AS WORD
        SUB Main()
            low = 10
            high = 100
            b = 150
            values(0) = Math.Clamp(b, low, high)
            values(1) = Math.Clamp(5, 10, 100)
            values(2) = Math.Clamp(500, 100, 1000)
            values(3) = Math.Clamp(65535, 32768, 60000)
            values(4) = Math.Clamp(-5, -10, 10)
            values(5) = Math.Clamp(-128, -10, 10)
            values(6) = Math.Clamp(127, -10, 10)
            values(7) = Math.Clamp(Math.Clamp(150, 10, 100), 20, 90)
        END SUB
    "#;
    let (rom, _) = compile_source(Some(source.into()), None, None).unwrap();
    let mut emulator = Emulator::new();
    emulator.load_rom(&rom).unwrap();
    for _ in 0..10 {
        emulator.step().unwrap();
    }
    for (index, expected) in [
        100u16,
        10,
        500,
        60000,
        (-5i16) as u16,
        (-10i16) as u16,
        10,
        90,
    ]
    .into_iter()
    .enumerate()
    {
        let address = 0x5c3 + index as u16 * 2;
        assert_eq!(
            u16::from_le_bytes([emulator.peek_cpu(address), emulator.peek_cpu(address + 1)]),
            expected,
            "clamp case {index}"
        );
    }
}

#[test]
fn test_math_sign_generation() {
    let mut st = SymbolTable::new();
    st.define(
        "i".to_string(),
        DataType::Int,
        swissarmynes::compiler::symbol_table::SymbolKind::Variable,
    )
    .unwrap();

    let program = Program {
        declarations: vec![
            TopLevel::Dim("i".to_string(), DataType::Int, None),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![Statement::Let(
                    Expression::Identifier("i".to_string()),
                    Expression::Call(
                        Box::new(Expression::MemberAccess(
                            Box::new(Expression::Identifier("Math".to_string())),
                            "Sign".to_string(),
                        )),
                        vec![Expression::Integer(-50)],
                    ),
                )],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let (code, _) = cg.generate(&program).expect("Codegen failed");

    // Check that sign checking instructions are emitted
    assert!(code
        .iter()
        .any(|line| line.contains("CPX #$80") || line.contains("CMP #$80")));
}

#[test]
fn test_bitwise_builtins_generation() {
    let mut st = SymbolTable::new();
    st.define(
        "w".to_string(),
        DataType::Word,
        swissarmynes::compiler::symbol_table::SymbolKind::Variable,
    )
    .unwrap();

    let program = Program {
        declarations: vec![
            TopLevel::Dim("w".to_string(), DataType::Word, None),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![
                    Statement::Let(
                        Expression::Identifier("w".to_string()),
                        Expression::Call(
                            Box::new(Expression::Identifier("BITAND".to_string())),
                            vec![Expression::Integer(0x00FF), Expression::Integer(0x0F0F)],
                        ),
                    ),
                    Statement::Let(
                        Expression::Identifier("w".to_string()),
                        Expression::Call(
                            Box::new(Expression::Identifier("BITNOT".to_string())),
                            vec![Expression::Integer(0x1234)],
                        ),
                    ),
                ],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let (code, _) = cg.generate(&program).expect("Codegen failed");

    assert!(code.iter().any(|line| line.contains("AND $00")));
    assert!(code.iter().any(|line| line.contains("EOR #$FF")));
}

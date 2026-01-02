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
    let (asm_banks, _) = cg.generate(&program).expect("Codegen failed");
    let bank0 = asm_banks.get(&0).expect("Bank 0 missing").join("\n");
    let bank7 = asm_banks.get(&7).expect("Bank 7 missing").join("\n");
    let code = format!("{}\n{}", bank0, bank7);

    // Check for Math_Mul16 usage
    assert!(code.contains("JSR Math_Mul16"));
    // Check that Math_Mul16 is defined
    assert!(code.contains("Math_Mul16:"));
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
    let (asm_banks, _) = cg.generate(&program).expect("Codegen failed");
    let bank0 = asm_banks.get(&0).expect("Bank 0 missing").join("\n");
    let bank7 = asm_banks.get(&7).expect("Bank 7 missing").join("\n");
    let code = format!("{}\n{}", bank0, bank7);

    assert!(code.contains("JSR Math_Div16"));
    assert!(code.contains("Math_Div16:"));
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
    let (asm_banks, _) = cg.generate(&program).expect("Codegen failed");
    let bank0 = asm_banks.get(&0).expect("Bank 0 missing").join("\n");
    let bank7 = asm_banks.get(&7).expect("Bank 7 missing").join("\n");
    let code = format!("{}\n{}", bank0, bank7);

    assert!(code.contains("JSR Math_Div16_Signed"));
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
    let (asm_banks, _) = cg.generate(&program).expect("Codegen failed");
    let bank0 = asm_banks.get(&0).expect("Bank 0 missing").join("\n");
    let bank7 = asm_banks.get(&7).expect("Bank 7 missing").join("\n");
    let code = format!("{}\n{}", bank0, bank7);

    assert!(code.contains("JSR Math_Div16"));
    // Check it reads Remainder from $08/$09
    assert!(code.contains("LDA $08"));
    assert!(code.contains("LDX $09"));
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
    let (asm_banks, _) = cg.generate(&program).expect("Codegen failed");
    let bank0 = asm_banks.get(&0).expect("Bank 0 missing").join("\n");
    let bank7 = asm_banks.get(&7).expect("Bank 7 missing").join("\n");
    let code = format!("{}\n{}", bank0, bank7);

    // Check for Signed logic (BVS)
    // LessThan logic: BVS overflow_lbl, BMI true_lbl...
    // We expect BVS to be generated
    assert!(code.contains("BVS"));
    // We expect comparison instructions
    assert!(code.contains("SEC"));
    assert!(code.contains("SBC $00"));
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
    let (asm_banks, _) = cg.generate(&program).expect("Codegen failed");
    let bank0 = asm_banks.get(&0).expect("Bank 0 missing").join("\n");
    let bank7 = asm_banks.get(&7).expect("Bank 7 missing").join("\n");
    let code = format!("{}\n{}", bank0, bank7);

    // Check for Signed logic (BVS) because Int is present
    assert!(code.contains("BVS"));
}

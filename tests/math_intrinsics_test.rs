// KEEP
use swissarmynes::compiler::ast::{DataType, Expression, Program, Statement, TopLevel};
use swissarmynes::compiler::codegen::CodeGenerator;
use swissarmynes::compiler::symbol_table::{SymbolKind, SymbolTable};

#[test]
fn test_math_abs_codegen() {
    let mut st = SymbolTable::new();
    st.define("i".to_string(), DataType::Int, SymbolKind::Variable)
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
                            "Abs".to_string(),
                        )),
                        vec![Expression::Identifier("i".to_string())],
                    ),
                )],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let (code, _) = cg.generate(&program).expect("Codegen failed");

    // Check that Math.Abs produces sign check and negate logic
    assert!(code.iter().any(|line| line.contains("CPX #$80")));
    assert!(code.iter().any(|line| line.contains("BCC")));
    assert!(code.iter().any(|line| line.contains("EOR #$FF")));
}

#[test]
fn test_math_wrap_and_lerp_codegen() {
    let mut st = SymbolTable::new();
    st.define("x".to_string(), DataType::Byte, SymbolKind::Variable)
        .unwrap();

    let program = Program {
        declarations: vec![
            TopLevel::Dim("x".to_string(), DataType::Byte, None),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![
                    Statement::Let(
                        Expression::Identifier("x".to_string()),
                        Expression::Call(
                            Box::new(Expression::MemberAccess(
                                Box::new(Expression::Identifier("Math".to_string())),
                                "Wrap".to_string(),
                            )),
                            vec![
                                Expression::Identifier("x".to_string()),
                                Expression::Integer(0),
                                Expression::Integer(100),
                            ],
                        ),
                    ),
                    Statement::Let(
                        Expression::Identifier("x".to_string()),
                        Expression::Call(
                            Box::new(Expression::MemberAccess(
                                Box::new(Expression::Identifier("Math".to_string())),
                                "Lerp".to_string(),
                            )),
                            vec![
                                Expression::Integer(10),
                                Expression::Integer(50),
                                Expression::Integer(128),
                            ],
                        ),
                    ),
                ],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let (code, _) = cg.generate(&program).expect("Codegen failed");

    // Check Wrap and Lerp codegen presence
    assert!(code.iter().any(|line| line.contains("Math_Mul16")));
}

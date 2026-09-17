// KEEP
use swissarmynes::compiler::analysis::SemanticAnalyzer;
use swissarmynes::compiler::ast::{DataType, Expression, Program, Statement, TopLevel};
use swissarmynes::compiler::codegen::CodeGenerator;
use swissarmynes::compiler::symbol_table::{SymbolKind, SymbolTable};

#[test]
fn test_bit_shift_intrinsics_codegen() {
    let mut st = SymbolTable::new();
    st.define("val".to_string(), DataType::Word, SymbolKind::Variable)
        .unwrap();

    let program = Program {
        declarations: vec![
            TopLevel::Dim("val".to_string(), DataType::Word, None),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![
                    Statement::Let(
                        Expression::Identifier("val".to_string()),
                        Expression::Call(
                            Box::new(Expression::Identifier("BITSHL".to_string())),
                            vec![
                                Expression::Identifier("val".to_string()),
                                Expression::Integer(2),
                            ],
                        ),
                    ),
                    Statement::Let(
                        Expression::Identifier("val".to_string()),
                        Expression::Call(
                            Box::new(Expression::Identifier("BITSHR".to_string())),
                            vec![
                                Expression::Identifier("val".to_string()),
                                Expression::Integer(3),
                            ],
                        ),
                    ),
                ],
            ),
        ],
    };

    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&program).expect("Analysis failed");

    let mut cg = CodeGenerator::new(st);
    let (code, _) = cg.generate(&program).expect("Codegen failed");

    assert!(code.iter().any(|line| line.contains("JSR Math_Shl16")));
    assert!(code.iter().any(|line| line.contains("JSR Math_Shr16")));
}

#[test]
fn test_bit_shift_constant_folding() {
    let expr_shl = Expression::Call(
        Box::new(Expression::Identifier("BITSHL".to_string())),
        vec![Expression::Integer(0b00000001), Expression::Integer(4)],
    );
    let folded_shl = SemanticAnalyzer::fold_constants_expr(&expr_shl);
    assert_eq!(folded_shl, Expression::Integer(0b00010000));

    let expr_shr = Expression::Call(
        Box::new(Expression::Identifier("BITSHR".to_string())),
        vec![Expression::Integer(0b00010000), Expression::Integer(2)],
    );
    let folded_shr = SemanticAnalyzer::fold_constants_expr(&expr_shr);
    assert_eq!(folded_shr, Expression::Integer(0b00000100));
}

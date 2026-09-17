// KEEP
use swissarmynes::compiler::analysis::SemanticAnalyzer;
use swissarmynes::compiler::ast::Expression;
use swissarmynes::compiler::codegen::CodeGenerator;
use swissarmynes::compiler::lexer::Lexer;
use swissarmynes::compiler::parser::Parser;

#[test]
fn test_bitwise_shift_constant_folding() {
    let expr_shl = Expression::Call(
        Box::new(Expression::Identifier("BITSHL".to_string())),
        vec![Expression::Integer(1), Expression::Integer(4)],
    );
    let folded_shl = SemanticAnalyzer::fold_constants_expr(&expr_shl);
    assert_eq!(folded_shl, Expression::Integer(16));

    let expr_shr = Expression::Call(
        Box::new(Expression::Identifier("BITSHR".to_string())),
        vec![Expression::Integer(64), Expression::Integer(2)],
    );
    let folded_shr = SemanticAnalyzer::fold_constants_expr(&expr_shr);
    assert_eq!(folded_shr, Expression::Integer(16));
}

#[test]
fn test_bitwise_shift_codegen() {
    let source = r#"
        DIM x AS WORD
        DIM y AS WORD
        SUB Main()
            LET x = BITSHL(y, 2)
            LET y = BITSHR(x, 1)
        END SUB
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenization failed");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parsing failed");

    let mut analyzer = SemanticAnalyzer::new();
    analyzer
        .analyze(&program)
        .expect("Semantic analysis failed");

    let mut cg = CodeGenerator::new(analyzer.symbol_table);
    let (code, _) = cg.generate(&program).expect("Codegen failed");

    assert!(code.iter().any(|line| line.contains("Math_Shl16")));
    assert!(code.iter().any(|line| line.contains("Math_Shr16")));
}

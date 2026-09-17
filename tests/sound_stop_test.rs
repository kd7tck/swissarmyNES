// KEEP
use swissarmynes::compiler::analysis::SemanticAnalyzer;
use swissarmynes::compiler::ast::{Expression, Program, Statement, TopLevel};
use swissarmynes::compiler::codegen::CodeGenerator;
use swissarmynes::compiler::symbol_table::SymbolTable;

#[test]
fn test_sound_stop_codegen() {
    let st = SymbolTable::new();

    let program = Program {
        declarations: vec![TopLevel::Sub(
            "Main".to_string(),
            vec![],
            vec![Statement::Call(
                Expression::MemberAccess(
                    Box::new(Expression::Identifier("Sound".to_string())),
                    "Stop".to_string(),
                ),
                vec![],
            )],
        )],
    };

    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&program).expect("Analysis failed");

    let mut cg = CodeGenerator::new(st);
    let (code, _) = cg.generate(&program).expect("Codegen failed");

    assert!(code.iter().any(|line| line.contains("JSR Sound_Init")));
}

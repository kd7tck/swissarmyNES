// KEEP
use swissarmynes::compiler::analysis::SemanticAnalyzer;
use swissarmynes::compiler::ast::{Expression, Program, Statement, TopLevel};
use swissarmynes::compiler::codegen::CodeGenerator;
use swissarmynes::compiler::symbol_table::SymbolTable;

#[test]
fn test_ppu_set_scroll_codegen() {
    let st = SymbolTable::new();

    let program = Program {
        declarations: vec![TopLevel::Sub(
            "Main".to_string(),
            vec![],
            vec![Statement::Call(
                Expression::MemberAccess(
                    Box::new(Expression::Identifier("PPU".to_string())),
                    "SetScroll".to_string(),
                ),
                vec![Expression::Integer(16), Expression::Integer(32)],
            )],
        )],
    };

    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&program).expect("Analysis failed");

    let mut cg = CodeGenerator::new(st);
    let (code, _) = cg.generate(&program).expect("Codegen failed");

    assert!(code.iter().any(|line| line.contains("STA $E0")));
    assert!(code.iter().any(|line| line.contains("STA $E1")));
}

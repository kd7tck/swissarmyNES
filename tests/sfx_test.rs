use swissarmynes::compiler::ast::{Expression, Program, Statement, TopLevel};
use swissarmynes::compiler::codegen::CodeGenerator;
use swissarmynes::compiler::symbol_table::SymbolTable;

#[test]
fn test_playsfx_generation() {
    let st = SymbolTable::new();
    let program = Program {
        declarations: vec![TopLevel::Sub(
            "Main".to_string(),
            vec![],
            vec![Statement::PlaySfx(Expression::Integer(1))],
        )],
    };

    let mut cg = CodeGenerator::new(st);
    let code = cg.generate(&program).expect("Codegen failed");

    // Verify Sound_Init called
    assert!(code.iter().any(|line| line.contains("JSR Sound_Init")));

    // Verify Sound_Play call
    assert!(code.iter().any(|line| line.contains("LDA #$01")));
    assert!(code.iter().any(|line| line.contains("JSR Sound_Play")));

    // Verify Sound Engine code injection
    assert!(code.iter().any(|line| line.contains("Sound_Init:")));
    assert!(code.iter().any(|line| line.contains("Sound_Play:")));
    assert!(code.iter().any(|line| line.contains("Play_Jump:")));
}

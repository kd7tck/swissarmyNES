// KEEP
use swissarmynes::compiler::ast::{DataType, Expression, Program, Statement, TopLevel};
use swissarmynes::compiler::codegen::CodeGenerator;
use swissarmynes::compiler::symbol_table::{SymbolKind, SymbolTable};

#[test]
fn test_memory_fill_codegen() {
    let mut st = SymbolTable::new();
    st.define(
        "buf".to_string(),
        DataType::Array(Box::new(DataType::Byte), 10),
        SymbolKind::Variable,
    )
    .unwrap();

    let program = Program {
        declarations: vec![
            TopLevel::Dim(
                "buf".to_string(),
                DataType::Array(Box::new(DataType::Byte), 10),
                None,
            ),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![Statement::Call(
                    Expression::MemberAccess(
                        Box::new(Expression::Identifier("Memory".to_string())),
                        "Fill".to_string(),
                    ),
                    vec![
                        Expression::Identifier("buf".to_string()),
                        Expression::Integer(10),
                        Expression::Integer(0xAA),
                    ],
                )],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let (code, _) = cg.generate(&program).expect("Codegen failed");

    // Verify fill loop generation
    assert!(code.iter().any(|line| line.contains("STA ($02),Y")));
    assert!(code.iter().any(|line| line.contains("INY")));
    assert!(code.iter().any(|line| line.contains("CPY $00")));
}

// KEEP
use swissarmynes::compiler::analysis::SemanticAnalyzer;
use swissarmynes::compiler::ast::{DataType, Expression, Program, Statement, TopLevel};
use swissarmynes::compiler::codegen::CodeGenerator;
use swissarmynes::compiler::symbol_table::{SymbolKind, SymbolTable};

#[test]
fn test_memory_copy_codegen() {
    let mut st = SymbolTable::new();
    st.define(
        "buf1".to_string(),
        DataType::Array(Box::new(DataType::Byte), 16),
        SymbolKind::Variable,
    )
    .unwrap();
    st.define(
        "buf2".to_string(),
        DataType::Array(Box::new(DataType::Byte), 16),
        SymbolKind::Variable,
    )
    .unwrap();

    let program = Program {
        declarations: vec![
            TopLevel::Dim(
                "buf1".to_string(),
                DataType::Array(Box::new(DataType::Byte), 16),
                None,
            ),
            TopLevel::Dim(
                "buf2".to_string(),
                DataType::Array(Box::new(DataType::Byte), 16),
                None,
            ),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![Statement::Call(
                    Expression::MemberAccess(
                        Box::new(Expression::Identifier("Memory".to_string())),
                        "Copy".to_string(),
                    ),
                    vec![
                        Expression::Identifier("buf1".to_string()),
                        Expression::Identifier("buf2".to_string()),
                        Expression::Integer(16),
                    ],
                )],
            ),
        ],
    };

    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&program).expect("Analysis failed");

    let mut cg = CodeGenerator::new(st);
    let (code, _) = cg.generate(&program).expect("Codegen failed");

    assert!(code.iter().any(|line| line.contains("LDA ($02),Y")));
    assert!(code.iter().any(|line| line.contains("STA ($04),Y")));
}

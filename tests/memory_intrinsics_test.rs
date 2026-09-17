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
    assert!(code.iter().any(|line| line.contains("CPY $04")));
}

#[test]
fn test_memory_fill_preserves_length_for_indexed_destination() {
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
                        Expression::Call(
                            Box::new(Expression::Identifier("buf".to_string())),
                            vec![Expression::Integer(1)],
                        ),
                        Expression::Integer(3),
                        Expression::Integer(0xAA),
                    ],
                )],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let (code, _) = cg.generate(&program).expect("Codegen failed");
    let address_index = code
        .iter()
        .position(|line| line.contains("STA $02"))
        .expect("indexed address should be generated");
    let length_save = code
        .iter()
        .position(|line| line.contains("STA $04"))
        .expect("fill length should be saved after address generation");
    assert!(length_save > address_index);
    assert!(code.iter().any(|line| line.contains("CPY $04")));
}

#[test]
fn test_memory_copy_codegen() {
    let mut st = SymbolTable::new();
    st.define(
        "src_buf".to_string(),
        DataType::Array(Box::new(DataType::Byte), 10),
        SymbolKind::Variable,
    )
    .unwrap();
    st.define(
        "dst_buf".to_string(),
        DataType::Array(Box::new(DataType::Byte), 10),
        SymbolKind::Variable,
    )
    .unwrap();

    let program = Program {
        declarations: vec![
            TopLevel::Dim(
                "src_buf".to_string(),
                DataType::Array(Box::new(DataType::Byte), 10),
                None,
            ),
            TopLevel::Dim(
                "dst_buf".to_string(),
                DataType::Array(Box::new(DataType::Byte), 10),
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
                        Expression::Identifier("src_buf".to_string()),
                        Expression::Identifier("dst_buf".to_string()),
                        Expression::Integer(10),
                    ],
                )],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let (code, _) = cg.generate(&program).expect("Codegen failed");

    // Verify copy loop generation
    assert!(code.iter().any(|line| line.contains("LDA ($00),Y")));
    assert!(code.iter().any(|line| line.contains("STA ($02),Y")));
    assert!(code.iter().any(|line| line.contains("CPY $04")));
}

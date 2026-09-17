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

    // The merged implementation preserves the destination on the stack while
    // generating the source address, then uses $00/$01 for source and $02/$03
    // for destination so indexed expressions cannot clobber either pointer.
    assert!(code.iter().any(|line| line.contains("LDA ($00),Y")));
    assert!(code.iter().any(|line| line.contains("STA ($02),Y")));
}

#[test]
fn test_memory_copy_preserves_indexed_addresses_and_length() {
    let mut st = SymbolTable::new();
    for name in ["src_buf", "dst_buf"] {
        st.define(
            name.to_string(),
            DataType::Array(Box::new(DataType::Byte), 16),
            SymbolKind::Variable,
        )
        .unwrap();
    }

    let indexed = |name: &str, offset: i32| {
        Expression::Call(
            Box::new(Expression::Identifier(name.to_string())),
            vec![Expression::Integer(offset)],
        )
    };
    let program = Program {
        declarations: vec![
            TopLevel::Dim(
                "src_buf".to_string(),
                DataType::Array(Box::new(DataType::Byte), 16),
                None,
            ),
            TopLevel::Dim(
                "dst_buf".to_string(),
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
                        indexed("src_buf", 2),
                        indexed("dst_buf", 5),
                        Expression::Integer(3),
                    ],
                )],
            ),
        ],
    };

    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&program).expect("Analysis failed");
    let mut cg = CodeGenerator::new(st);
    let (code, _) = cg.generate(&program).expect("Codegen failed");

    // Indexed address generation uses $00/$01. Memory.Copy must preserve the
    // length and destination pointer before evaluating the source address.
    let main_start = code
        .iter()
        .position(|line| line == "Main:")
        .expect("generated Main subroutine should be present");
    let main_end = code
        .iter()
        .enumerate()
        .skip(main_start + 1)
        .find(|(_, line)| line.trim() == "RTS")
        .map(|(index, _)| index)
        .expect("generated Main subroutine should return");
    let main_code = &code[main_start..=main_end];

    let first_length_save = main_code
        .iter()
        .position(|line| line.trim() == "PHA")
        .expect("length should be saved before indexed addresses");
    let destination_address = main_code
        .iter()
        .position(|line| line.trim() == "STA $02")
        .expect("destination address should be generated");
    let source_saved_high = main_code
        .iter()
        .rposition(|line| line.trim() == "STA $01")
        .expect("source high byte should be copied to scratch");
    let restored_destination_high = main_code
        .iter()
        .enumerate()
        .skip(source_saved_high + 1)
        .find(|(_, line)| line.trim() == "STA $03")
        .map(|(index, _)| index)
        .expect("destination high byte should be restored");
    let restored_length = main_code
        .iter()
        .enumerate()
        .skip(restored_destination_high + 1)
        .find(|(_, line)| line.trim() == "STA $04")
        .map(|(index, _)| index)
        .expect("length should be restored after source generation");
    let copy_loop = main_code
        .iter()
        .position(|line| line.contains("LDA ($00),Y"))
        .expect("copy loop should load from the preserved source pointer");

    assert!(first_length_save < destination_address);
    assert!(destination_address < source_saved_high);
    assert!(source_saved_high < restored_destination_high);
    assert!(restored_destination_high < restored_length);
    assert!(restored_length < copy_loop);
}

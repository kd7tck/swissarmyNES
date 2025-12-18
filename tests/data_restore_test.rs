use swissarmynes::compiler::ast::{DataType, Expression, Program, Statement, TopLevel};
use swissarmynes::compiler::codegen::CodeGenerator;
use swissarmynes::compiler::symbol_table::{SymbolKind, SymbolTable};

#[test]
fn test_data_restore_label() {
    let mut st = SymbolTable::new();
    st.define("x".to_string(), DataType::Byte, SymbolKind::Variable)
        .unwrap();

    let program = Program {
        declarations: vec![
            TopLevel::Dim("x".to_string(), DataType::Byte, None),
            // DATA 10, 20
            TopLevel::Data(None, vec![Expression::Integer(10), Expression::Integer(20)]),
            // MyData: DATA 30, 40
            TopLevel::Data(
                Some("MyData".to_string()),
                vec![Expression::Integer(30), Expression::Integer(40)],
            ),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![
                    // RESTORE MyData
                    Statement::Restore(Some("MyData".to_string())),
                    // READ x
                    Statement::Read(vec!["x".to_string()]),
                ],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let code = cg.generate(&program).expect("Codegen failed");
    let code_str = code.join("\n");

    // Verify Label in User Data
    assert!(code_str.contains("MyData:"));
    assert!(code_str.contains("db $1E")); // 30 in Hex

    // Verify Data Table Entry
    assert!(code_str.contains("Ptr_MyData: WORD MyData"));

    // Verify RESTORE implementation
    // RESTORE MyData should load Ptr_MyData
    // We can check if it tries to load an address and store to $04/$05
    // and that address is likely near the other Ptr_ entries.
    // However, exact offset matching in unit test is fragile if we add more default tables.
    // Instead we check structure.

    assert!(code_str.contains("LDA $FF")); // Loading from Table
    assert!(code_str.contains("STA $04")); // Update Data Ptr Low
    assert!(code_str.contains("STA $05")); // Update Data Ptr High
}

use swissarmynes::compiler::ast::{DataType, Expression, Program, TopLevel};
use swissarmynes::compiler::codegen::CodeGenerator;
use swissarmynes::compiler::symbol_table::{SymbolKind, SymbolTable};

#[test]
fn test_string_compilation() {
    let mut st = SymbolTable::new();
    // Simulate Semantic Analyzer registering the symbol
    st.define("s".to_string(), DataType::String, SymbolKind::Variable)
        .unwrap();

    let program = Program {
        declarations: vec![TopLevel::Dim(
            "s".to_string(),
            DataType::String,
            Some(Expression::StringLiteral("Hello".to_string())),
        )],
    };

    let mut cg = CodeGenerator::new(st);
    let code = cg.generate(&program).expect("Codegen failed");

    // Check RAM allocation
    // s @ $05C0 (2 bytes)
    assert!(code.iter().any(|line| line.contains("s @ $05C0")));

    // Check String Data
    // GEN_L1: db ..., ...
    // H=48, e=65, l=6C, l=6C, o=6F
    assert!(code
        .iter()
        .any(|line| line.contains("db $48, $65, $6C, $6C, $6F, $00")));

    // Check Data Table
    // Ptr_GEN_L1: WORD GEN_L1
    assert!(code.iter().any(|line| line.contains("WORD GEN_L1")));

    // Check Initialization
    // LDA $FFxx, STA $03A0
    // LDA $FFxx+1, STA $03A1
    // We don't know exact address but we can check pattern
    assert!(code.iter().any(|line| line.contains("Init String 's'")));
}

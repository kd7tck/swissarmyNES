use swissarmynes::compiler::ast::{DataType, Expression, Program, Statement, TopLevel};
use swissarmynes::compiler::codegen::CodeGenerator;
use swissarmynes::compiler::symbol_table::SymbolTable;

#[test]
fn test_string_literal_in_call() {
    let st = SymbolTable::new();

    // Program:
    // SUB Print(s AS STRING)
    // END SUB
    // SUB Main()
    //   CALL Print("Hello")
    // END SUB

    let program = Program {
        declarations: vec![
            TopLevel::Sub(
                "Print".to_string(),
                vec![("s".to_string(), DataType::String)],
                vec![],
            ),
            TopLevel::Sub(
                "Main".to_string(),
                vec![],
                vec![Statement::Call(
                    Expression::Identifier("Print".to_string()),
                    vec![Expression::StringLiteral("Hello".to_string())],
                )],
            ),
        ],
    };

    let mut cg = CodeGenerator::new(st);
    let result = cg.generate(&program);

    assert!(result.is_ok(), "Code generation failed: {:?}", result.err());
}

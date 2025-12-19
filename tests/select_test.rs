use swissarmynes::compiler::lexer::tokenize;
use swissarmynes::compiler::parser::Parser;
use swissarmynes::compiler::analysis::SemanticAnalyzer;
use swissarmynes::compiler::codegen::CodeGenerator;
use swissarmynes::compiler::ast::{Statement, TopLevel, CaseCondition};

#[test]
fn test_select_case_ranges() {
    let input = r#"
    SUB Main()
        x = 5
        SELECT CASE x
            CASE 1 TO 10
                y = 1
            CASE IS > 20
                y = 2
            CASE 0
                y = 3
        END SELECT
    END SUB
    "#;
    let tokens = tokenize(input);
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().expect("Failed to parse");

    // Check AST
    if let TopLevel::Sub(_, _, body) = &program.declarations[0] {
        if let Statement::Select(_, cases, _) = &body[1] { // 0 is Let x=5
            assert_eq!(cases.len(), 3);
            match &cases[0].0 {
                CaseCondition::Range(_, _) => {}
                _ => panic!("Expected Range"),
            }
            match &cases[1].0 {
                CaseCondition::Comparison(_, _) => {}
                _ => panic!("Expected Comparison"),
            }
            match &cases[2].0 {
                CaseCondition::Equal(_) => {}
                _ => panic!("Expected Equal"),
            }
        } else {
            panic!("Expected Select");
        }
    }

    // Check Codegen
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&program).expect("Analysis failed");
    let mut codegen = CodeGenerator::new(analyzer.symbol_table);
    let code = codegen.generate(&program).expect("Codegen failed");

    // Verify ranges generate comparisons
    // Range 1 TO 10 should generate >= 1 AND <= 10
    // >= is BCS/BCC depending on impl.
    // We expect CMP instructions.
    assert!(code.iter().any(|line| line.contains("CMP $010"))); // Compare with stack
}

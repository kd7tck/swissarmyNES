use swissarmynes::compiler::ast::{DataType, Expression, Program, Statement, TopLevel};
use swissarmynes::compiler::codegen::CodeGenerator;
use swissarmynes::compiler::symbol_table::{SymbolKind, SymbolTable};

#[test]
fn test_string_functions_codegen() {
    let mut st = SymbolTable::new();
    // Define a string variable 's' and integer 'i'
    st.define("s".to_string(), DataType::String, SymbolKind::Variable).unwrap();
    st.assign_address("s", 0x0300).unwrap();
    st.define("i".to_string(), DataType::Word, SymbolKind::Variable).unwrap();
    st.assign_address("i", 0x0302).unwrap();

    // Program:
    // i = ASC(s)
    // i = VAL(s)
    // s = CHR(65)
    // s = STR(123)
    // s = LEFT(s, 2)
    // s = RIGHT(s, 2)
    // s = MID(s, 2, 1)

    let statements = vec![
        Statement::Let(
            Expression::Identifier("i".to_string()),
            Expression::Call(
                Box::new(Expression::Identifier("ASC".to_string())),
                vec![Expression::Identifier("s".to_string())],
            ),
        ),
        Statement::Let(
            Expression::Identifier("i".to_string()),
            Expression::Call(
                Box::new(Expression::Identifier("VAL".to_string())),
                vec![Expression::Identifier("s".to_string())],
            ),
        ),
        Statement::Let(
            Expression::Identifier("s".to_string()),
            Expression::Call(
                Box::new(Expression::Identifier("CHR".to_string())),
                vec![Expression::Integer(65)],
            ),
        ),
        Statement::Let(
            Expression::Identifier("s".to_string()),
            Expression::Call(
                Box::new(Expression::Identifier("STR".to_string())),
                vec![Expression::Integer(123)],
            ),
        ),
        Statement::Let(
            Expression::Identifier("s".to_string()),
            Expression::Call(
                Box::new(Expression::Identifier("LEFT".to_string())),
                vec![Expression::Identifier("s".to_string()), Expression::Integer(2)],
            ),
        ),
         Statement::Let(
            Expression::Identifier("s".to_string()),
            Expression::Call(
                Box::new(Expression::Identifier("RIGHT".to_string())),
                vec![Expression::Identifier("s".to_string()), Expression::Integer(2)],
            ),
        ),
         Statement::Let(
            Expression::Identifier("s".to_string()),
            Expression::Call(
                Box::new(Expression::Identifier("MID".to_string())),
                vec![Expression::Identifier("s".to_string()), Expression::Integer(2), Expression::Integer(1)],
            ),
        ),
    ];

    let program = Program {
        declarations: vec![TopLevel::Sub("Main".to_string(), vec![], statements)],
    };

    let mut cg = CodeGenerator::new(st);
    let code = cg.generate(&program).expect("Codegen failed");

    // Verify calls
    assert!(code.iter().any(|line| line.contains("JSR Runtime_Asc")));
    assert!(code.iter().any(|line| line.contains("JSR Runtime_Val")));
    assert!(code.iter().any(|line| line.contains("JSR Runtime_Chr")));
    assert!(code.iter().any(|line| line.contains("JSR Runtime_Str")));
    assert!(code.iter().any(|line| line.contains("JSR Runtime_Left")));
    assert!(code.iter().any(|line| line.contains("JSR Runtime_Right")));
    assert!(code.iter().any(|line| line.contains("JSR Runtime_Mid")));
}

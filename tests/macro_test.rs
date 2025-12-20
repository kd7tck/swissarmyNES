use swissarmynes::compiler::ast::{BinaryOperator, Expression, Statement, TopLevel};
use swissarmynes::compiler::{lexer, parser, preprocessor};

#[test]
fn test_macro_expansion_basic() {
    let source = r#"
    DEF MACRO SetX(val)
        x = val
    END MACRO

    SUB Main()
        SetX(10)
        SetX(20)
    END SUB
    "#;

    let tokens = lexer::tokenize(source);
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Parse failed");
    let expanded = preprocessor::expand_macros(program).expect("Expansion failed");

    // Verify
    // Main should have 2 statements: Let(x, 10), Let(x, 20)
    if let TopLevel::Sub(name, _, body) = &expanded.declarations[0] {
        assert_eq!(name, "Main");
        assert_eq!(body.len(), 2);

        match &body[0] {
            Statement::Let(target, val) => {
                if let Expression::Identifier(n) = target {
                    assert_eq!(n, "x");
                }
                if let Expression::Integer(v) = val {
                    assert_eq!(*v, 10);
                }
            }
            _ => panic!("Expected Let statement 1"),
        }

        match &body[1] {
            Statement::Let(_target, val) => {
                if let Expression::Integer(v) = val {
                    assert_eq!(*v, 20);
                }
            }
            _ => panic!("Expected Let statement 2"),
        }
    } else {
        panic!("Expected Sub Main");
    }
}

#[test]
fn test_macro_expansion_nested() {
    let source = r#"
    DEF MACRO Inc(v)
        v = v + 1
    END MACRO

    DEF MACRO DoubleInc(v)
        Inc(v)
        Inc(v)
    END MACRO

    SUB Main()
        DoubleInc(x)
    END SUB
    "#;

    let tokens = lexer::tokenize(source);
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Parse failed");
    let expanded = preprocessor::expand_macros(program).expect("Expansion failed");

    if let TopLevel::Sub(_, _, body) = &expanded.declarations[0] {
        // DoubleInc expands to 2 statements
        assert_eq!(body.len(), 2);
    }
}

#[test]
fn test_macro_argument_expression() {
    let source = r#"
    DEF MACRO Add(a, b)
        result = a + b
    END MACRO

    SUB Main()
        Add(10, 20)
        Add(x * 2, y + 5)
    END SUB
    "#;

    let tokens = lexer::tokenize(source);
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Parse failed");
    let expanded = preprocessor::expand_macros(program).expect("Expansion failed");

    if let TopLevel::Sub(_, _, body) = &expanded.declarations[0] {
        // 2nd call: result = (x * 2) + (y + 5)
        match &body[1] {
            Statement::Let(_, expr) => {
                match expr {
                    Expression::BinaryOp(_l, op, _r) => {
                        assert_eq!(*op, BinaryOperator::Add);
                        // Left should be BinaryOp(Mult)
                        // Right should be BinaryOp(Add)
                    }
                    _ => panic!("Expected BinaryOp"),
                }
            }
            _ => panic!("Expected Let"),
        }
    }
}

#[test]
fn test_macro_recursion_limit() {
    let source = r#"
    DEF MACRO Recursive()
        Recursive()
    END MACRO

    SUB Main()
        Recursive()
    END SUB
    "#;

    let tokens = lexer::tokenize(source);
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Parse failed");
    let result = preprocessor::expand_macros(program);

    assert!(result.is_err());
    assert!(result.err().unwrap().contains("recursion limit"));
}

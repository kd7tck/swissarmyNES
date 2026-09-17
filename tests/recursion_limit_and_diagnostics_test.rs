// KEEP
use swissarmynes::server::api::{compile_source, MAX_SOURCE_LENGTH};

#[test]
fn test_deep_if_nesting_returns_error() {
    let depth = 300;
    let mut code = String::new();
    code.push_str("SUB Main()\n");
    for _ in 0..depth {
        code.push_str("IF 1 THEN\n");
    }
    code.push_str("DIM x AS BYTE\n");
    for _ in 0..depth {
        code.push_str("END IF\n");
    }
    code.push_str("END SUB\n");

    let result = compile_source(Some(code), None, None);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("Maximum recursion depth exceeded"),
        "Expected recursion depth error, got: {}",
        err
    );
}

#[test]
fn test_deep_expression_nesting_returns_error() {
    let depth = 300;
    let mut expr = String::new();
    for _ in 0..depth {
        expr.push('(');
    }
    expr.push('1');
    for _ in 0..depth {
        expr.push(')');
    }

    let code = format!("SUB Main()\n  LET x = {}\nEND SUB\n", expr);
    let result = compile_source(Some(code), None, None);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("Maximum recursion depth exceeded"),
        "Expected recursion depth error, got: {}",
        err
    );
}

#[test]
fn test_compiler_remains_usable_after_depth_error() {
    // Fire deep nesting first
    test_deep_if_nesting_returns_error();

    // Now compile a normal valid source
    let valid_code = "DIM x AS BYTE\nSUB Main()\n  LET x = 10\nEND SUB\n".to_string();
    let result = compile_source(Some(valid_code), None, None);
    assert!(
        result.is_ok(),
        "Compiler should remain functional after handling depth limit error, got: {:?}",
        result.err()
    );
}

#[test]
fn test_constant_division_by_zero_diagnostic() {
    let div_zero_code = "SUB Main()\n  LET x = 5 / 0\nEND SUB\n".to_string();
    let result = compile_source(Some(div_zero_code), None, None);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("Division by zero"),
        "Expected 'Division by zero' diagnostic, got: {}",
        err
    );

    let mod_zero_code = "SUB Main()\n  LET x = 10 MOD 0\nEND SUB\n".to_string();
    let result = compile_source(Some(mod_zero_code), None, None);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("Division by zero"),
        "Expected 'Division by zero' diagnostic, got: {}",
        err
    );
}

#[test]
fn test_source_length_limit() {
    let huge_source = " ".repeat(MAX_SOURCE_LENGTH + 10);
    let result = compile_source(Some(huge_source), None, None);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("Source code exceeds maximum allowed length"),
        "Expected max source length error, got: {}",
        err
    );
}

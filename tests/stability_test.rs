use swissarmynes::server::api::{compile_source, MAX_SOURCE_LENGTH};
use swissarmynes::server::project::{create_project, write_file};

#[test]
fn test_parser_recursion_depth_limit() {
    let mut code = String::from("SUB Main()\n");
    for _ in 0..300 {
        code.push_str("IF 1 THEN\n");
    }
    code.push_str("LET x = 1\n");
    for _ in 0..300 {
        code.push_str("END IF\n");
    }
    code.push_str("END SUB\n");

    let result = compile_source(Some(code), None, None);
    assert!(result.is_err(), "Expected recursion depth limit error");
    let err = result.unwrap_err();
    assert!(
        err.contains("Maximum recursion depth exceeded"),
        "Unexpected error message: {}",
        err
    );
}

#[test]
fn test_source_length_limit() {
    let large_code = " ".repeat(MAX_SOURCE_LENGTH + 10);
    let result = compile_source(Some(large_code), None, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("maximum length limit"));
}

#[test]
fn test_empty_name_validations() {
    let res = create_project("");
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("cannot be empty"));

    let res = write_file("proj", "", "content");
    assert!(res.is_err());

    let res = write_file("proj", ".", "content");
    assert!(res.is_err());

    let res = write_file("proj", "..", "content");
    assert!(res.is_err());
}

#[test]
fn test_division_by_zero_error() {
    let code = "SUB Main()\n LET a = 5 / 0\nEND SUB";
    let result = compile_source(Some(code.to_string()), None, None);
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(
        err.contains("Division by zero"),
        "Unexpected error message: {}",
        err
    );
}

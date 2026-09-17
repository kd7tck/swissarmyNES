use swissarmynes::compiler::analysis::SemanticAnalyzer;
use swissarmynes::compiler::assembler::Assembler;
use swissarmynes::compiler::codegen::CodeGenerator;
use swissarmynes::compiler::lexer::Lexer;
use swissarmynes::compiler::parser::Parser;

#[test]
fn test_diagnostics_byte_overflow() {
    let source = "
        DIM a AS BYTE
        SUB Main()
            LET a = 99999
        END SUB
    ";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut analyzer = SemanticAnalyzer::new();
    let errs = analyzer.analyze(&program).unwrap_err();
    assert!(errs.iter().any(|e| e.contains("BYTE")));
}

#[test]
fn test_diagnostics_division_by_zero() {
    let source = "
        DIM x AS BYTE
        SUB Main()
            LET x = 5 / 0
        END SUB
    ";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut analyzer = SemanticAnalyzer::new();
    let errs = analyzer.analyze(&program).unwrap_err();
    assert!(errs.iter().any(|e| e.contains("Division by zero")));
}

#[test]
fn test_diagnostics_empty_string_assembly() {
    let source = "
        DIM s AS STRING
        SUB Main()
            LET s = \"\"
        END SUB
    ";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&program).unwrap();
    let mut cg = CodeGenerator::new(analyzer.symbol_table);
    let (asm_lines, _) = cg.generate(&program).unwrap();
    let assembler = Assembler::new();
    let rom = assembler.assemble(&asm_lines, None, vec![]);
    assert!(rom.is_ok(), "Assembly failed for empty string literal");
}

#[test]
fn test_diagnostics_string_too_long() {
    let long_str = "A".repeat(300);
    let source = format!(
        "
        DIM s AS STRING
        SUB Main()
            LET s = \"{}\"
        END SUB
    ",
        long_str
    );
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut analyzer = SemanticAnalyzer::new();
    let errs = analyzer.analyze(&program).unwrap_err();
    assert!(errs.iter().any(|e| e.contains("maximum length")));
}

#[test]
fn test_diagnostics_mismatched_next() {
    let source = "
        DIM i AS BYTE
        SUB Main()
            FOR i = 1 TO 5
            NEXT j
        END SUB
    ";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let err = parser.parse().unwrap_err();
    assert!(err.contains("Mismatched NEXT variable 'j' for FOR loop variable 'i'"));
}

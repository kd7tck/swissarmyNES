#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_ram_overflow() {
        let source = r#"
        DIM x(600) AS BYTE
        SUB Main()
        END SUB
        "#;

        let tokens = Lexer::new(source).tokenize().expect("Lex failed");
        let program = Parser::new(tokens).parse().expect("Parse failed");

        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(&program).expect("Analysis failed");

        let symbol_table = analyzer.symbol_table;
        let mut codegen = CodeGenerator::new(symbol_table);

        let result = codegen.generate(&program);

        assert!(result.is_err(), "Should have failed due to RAM overflow");
        assert_eq!(
            result.err().unwrap(),
            "RAM overflow: Variable 'x' allocation exceeded safe memory limit ($07FF)"
        );
    }
}

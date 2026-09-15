#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;
    use swissarmynes::server::api::compile_source;

    #[test]
    fn production_allocation_boundaries_and_large_dimensions() {
        // $05C0..$07EF offers 560 bytes; the final sixteen bytes belong to runtime.
        assert!(compile_source(
            Some("DIM x(560) AS BYTE\nSUB Main()\nEND SUB".into()),
            None,
            None
        )
        .is_ok());
        for declaration in [
            "DIM x(561) AS BYTE",
            "DIM x(65536) AS BYTE",
            "DIM x(40000) AS WORD",
            "TYPE Huge\na(40000) AS WORD\nEND TYPE\nDIM x AS Huge",
        ] {
            let source = format!("{declaration}\nSUB Main()\nEND SUB");
            assert!(
                compile_source(Some(source), None, None).is_err(),
                "{declaration}"
            );
        }
    }

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
            "RAM overflow: Variable 'x' allocation exceeded safe memory limit ($07EF)"
        );
    }
}

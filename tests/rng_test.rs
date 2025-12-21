#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_rng() {
        let source = "
            SUB Main()
                DIM x AS WORD
                RANDOMIZE 1234
                x = RND(100)
            END SUB
        ";

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing failed");
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(&program).expect("Analysis failed");

        let symbol_table = analyzer.symbol_table;
        let mut codegen = CodeGenerator::new(symbol_table);
        let asm_lines = codegen.generate(&program).expect("Codegen failed");
        let asm_source = asm_lines.join("\n");

        assert!(asm_source.contains("JSR Runtime_Randomize"));
        assert!(asm_source.contains("JSR Runtime_Random"));
        assert!(asm_source.contains("JSR Math_Div16"));

        // Verify implementation details
        assert!(asm_source.contains("Runtime_Random:"));
        assert!(asm_source.contains("EOR #$39")); // LFSR
    }
}

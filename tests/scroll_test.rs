#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_scroll_set() {
        let source = "
            SUB Main()
                Scroll.Set(50, 10)
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

        // Verify Scroll.Set logic
        assert!(asm_source.contains("LDA #$32")); // 50
        assert!(asm_source.contains("STA $E0"));
        assert!(asm_source.contains("LDA #$0A")); // 10
        assert!(asm_source.contains("STA $E1"));

        // Verify TrampolineNMI
        assert!(asm_source.contains("TrampolineNMI:"));
        assert!(asm_source.contains("LDA $E0"));
        assert!(asm_source.contains("STA $2005"));
        assert!(asm_source.contains("LDA $E1"));
        assert!(asm_source.contains("STA $2005"));
    }
}

#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_array_declaration_and_access() {
        let source = "
            DIM arr(10) AS BYTE
            DIM idx AS BYTE
            DIM x AS BYTE

            SUB Main()
                arr(0) = 5
                idx = 2
                arr(idx) = 10

                ' Read back
                x = arr(0)
                x = arr(idx)
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
        let _asm_source = asm_lines.join("\n");

        // Verify allocation
        // arr @ $0460. Size 10. Next var idx @ $046A.
        assert!(asm_lines.iter().any(|line| line.contains("arr @ $0460")));
        assert!(asm_lines.iter().any(|line| line.contains("idx @ $046A"))); // 420 + 10 = 42A

        // Verify assignment arr(0) = 5
        // Should calculate address $0460 + 0 = $0460.
        // STA ($02),Y or similar.
        assert!(asm_lines.iter().any(|line| line.contains("STA ($02),Y")));
    }

    #[test]
    fn test_word_array() {
        let source = "
            DIM warr(5) AS WORD
            DIM val AS WORD
            SUB Main()
                warr(1) = 1000
                val = warr(1)
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

        // Size 5 * 2 = 10 bytes.
        // Assignment 1000 ($03E8).
        // Word assignment logic involves storing Low and High.
        // Look for INY
        assert!(asm_lines.iter().any(|line| line.contains("INY")));
    }
}

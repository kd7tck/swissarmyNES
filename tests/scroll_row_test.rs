#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_scroll_row() {
        let source = "
            DIM row(32) AS BYTE
            SUB Main()
                PPU.Ctrl($90) ' Enable NMI + BG
                Scroll.LoadRow(240, row)
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

        // Verify Scroll.LoadRow call
        assert!(asm_source.contains("JSR Runtime_Scroll_LoadRow"));

        // Verify Runtime Helper existence
        assert!(asm_source.contains("Runtime_Scroll_LoadRow:"));
        assert!(asm_source.contains("Scroll_RowBaseStore:"));

        // Verify Header Writing (Type 1)
        assert!(asm_source.contains("LDA #1"));
        assert!(asm_source.contains("STA $0381")); // Type

        // Verify NMI Processing (Row Loop)
        assert!(asm_source.contains("VBlankBufferRowLoop:"));
        assert!(asm_source.contains("CPX #32")); // 32 tiles
    }
}

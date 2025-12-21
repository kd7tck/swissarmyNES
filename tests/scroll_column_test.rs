#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_scroll_column() {
        let source = "
            DIM col(30) AS BYTE
            SUB Main()
                PPU.Ctrl($90) ' Enable NMI + BG
                Scroll.LoadColumn(256, col)
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

        // Verify PPU.Ctrl
        assert!(asm_source.contains("LDA #$90"));
        assert!(asm_source.contains("STA $F8"));
        assert!(asm_source.contains("STA $2000"));

        // Verify Scroll.LoadColumn call
        assert!(asm_source.contains("JSR Runtime_Scroll_LoadColumn"));

        // Verify Runtime Helper
        assert!(asm_source.contains("Runtime_Scroll_LoadColumn:"));
        assert!(asm_source.contains("STA $0451")); // Type
        assert!(asm_source.contains("STA $0454, Y")); // Data Copy

        // Verify NMI Processing
        assert!(asm_source.contains("TrampolineNMI:"));
        assert!(asm_source.contains("LDA $0450")); // Check Flag
        assert!(asm_source.contains("STA $2006")); // Set Addr
        assert!(asm_source.contains("ORA #$04")); // Inc 32
        assert!(asm_source.contains("STA $2007")); // Write Data
    }
}

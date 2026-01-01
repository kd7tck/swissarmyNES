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
        let (asm_banks, _) = codegen.generate(&program).expect("Codegen failed");

        // Check Bank 0 for User Code and NMI
        let asm_lines_0 = asm_banks.get(&0).expect("Bank 0 missing");
        let asm_source_0 = asm_lines_0.join("\n");

        // Verify PPU.Ctrl
        assert!(asm_source_0.contains("LDA #$90"));
        assert!(asm_source_0.contains("STA $F8"));
        assert!(asm_source_0.contains("STA $2000"));

        // Verify Scroll.LoadColumn call
        assert!(asm_source_0.contains("JSR Runtime_Scroll_LoadColumn"));

        // Verify NMI Processing (in Bank 0 Startup)
        assert!(asm_source_0.contains("TrampolineNMI:"));
        assert!(asm_source_0.contains("LDA $0380")); // Check Flag
        assert!(asm_source_0.contains("STA $2006")); // Set Addr
        assert!(asm_source_0.contains("ORA #$04")); // Inc 32
        assert!(asm_source_0.contains("STA $2007")); // Write Data

        // Check Bank 7 for Runtime Helper
        let asm_lines_7 = asm_banks.get(&7).expect("Bank 7 missing");
        let asm_source_7 = asm_lines_7.join("\n");

        // Verify Runtime Helper
        assert!(asm_source_7.contains("Runtime_Scroll_LoadColumn:"));
        assert!(asm_source_7.contains("STA $0381")); // Type
        assert!(asm_source_7.contains("STA $0384, Y")); // Data Copy
    }
}

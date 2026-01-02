#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::assembler::Assembler;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;
    use std::collections::HashMap;

    #[test]
    fn test_metasprite_compilation() {
        let source = r#"
            METASPRITE player_idle
                TILE 0, 0, $10, 0
                TILE 8, 0, $11, 0
            END METASPRITE

            SUB Main()
                Sprite.Clear()
                Sprite.Draw(100, 100, player_idle)
            END SUB
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");

        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing failed");

        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(&program).expect("Analysis failed");

        let symbol_table = analyzer.symbol_table;
        let mut codegen = CodeGenerator::new(symbol_table);
        let (asm_banks, _) = codegen.generate(&program).expect("Codegen failed");

        let mut assembler_inputs = HashMap::new();
        for (bank, lines) in &asm_banks {
            assembler_inputs.insert(*bank, lines.join("\n"));
        }

        // Verify content in Bank 7 (System Bank) where User Data is currently emitted
        let bank7_source = assembler_inputs.get(&7).expect("Bank 7 missing");

        assert!(bank7_source.contains("player_idle:"), "Metasprite label missing from Bank 7");
        assert!(bank7_source.contains("Runtime_SpriteDraw:"));
        assert!(bank7_source.contains("Runtime_SpriteClear:"));

        // Check for Data Bytes
        assert!(bank7_source.contains("db $02")); // Count
        assert!(bank7_source.contains("db $00, $00, $10, $00")); // Tile 1
        assert!(bank7_source.contains("db $08, $00, $11, $00")); // Tile 2

        let assembler = Assembler::new();
        let rom = assembler
            .assemble(&assembler_inputs, None, vec![])
            .expect("Assembly failed");

        // 128KB PRG + 8KB CHR + 16b Header = 131072 + 8192 + 16 = 139280
        assert_eq!(rom.len(), 139280);
    }
}

#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::assembler::Assembler;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

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
        let asm_lines = codegen.generate(&program).expect("Codegen failed");
        let asm_source = asm_lines.join("\n");

        assert!(asm_source.contains("player_idle:"));
        assert!(asm_source.contains("Runtime_SpriteDraw:"));
        assert!(asm_source.contains("Runtime_SpriteClear:"));
        // Check for Data Bytes (may be split across lines or spaces)
        // db $02
        assert!(asm_source.contains("db $02"));
        // db $00, $00, $10, $00
        assert!(asm_source.contains("db $00, $00, $10, $00"));
        // db $08, $00, $11, $00
        assert!(asm_source.contains("db $08, $00, $11, $00"));

        let assembler = Assembler::new();
        let rom = assembler
            .assemble(&asm_source, None, vec![])
            .expect("Assembly failed");

        assert_eq!(rom.len(), 40976);
    }
}

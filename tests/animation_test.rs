#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::assembler::Assembler;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_animation_compilation() {
        let source = r#"
            METASPRITE Idle
                TILE 0, 0, $10, 0
            END METASPRITE

            METASPRITE Run
                TILE 0, 0, $11, 0
            END METASPRITE

            ANIMATION PlayerRun
                FRAME Idle, 10
                FRAME Run, 5
                LOOP
            END ANIMATION

            DIM player_anim AS AnimState

            SUB Main()
                Animation.Play(player_anim, PlayerRun)

                DO
                    Animation.Update(player_anim)
                    Animation.Draw(100, 100, player_anim)
                LOOP WHILE 1
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

        let mut bank_sources = std::collections::HashMap::new();
        for (bank, lines) in &asm_banks {
            bank_sources.insert(*bank, lines.join("\n"));
        }
        // Bank 0 should have Main and Animation Data
        let asm_source = bank_sources.get(&0).expect("Bank 0 missing");
        println!("{}", asm_source);

        // Verify Assembly
        assert!(asm_source.contains("PlayerRun:"));
        // Count=2, Loop=1. Might be on different lines or same.
        // The generator output:
        // PlayerRun:
        //   db $02
        //   db $01
        assert!(asm_source.contains("PlayerRun:"));
        assert!(asm_source.contains("db $02"));
        assert!(asm_source.contains("db $01"));

        // Check for Helpers (In Bank 7, or stubbed in Bank 0 if using trampoline)
        // Since we check the joined source, we need to check Bank 7 for implementations
        // Or if we assembled, we check ROM.

        // Let's check Bank 7 source for helpers
        let bank7 = bank_sources.get(&7).expect("Bank 7 missing");
        assert!(bank7.contains("Runtime_Anim_Update:"));
        assert!(bank7.contains("Runtime_Anim_Draw:"));

        let assembler = Assembler::new();
        let _rom = assembler
            .assemble(&bank_sources, None, vec![])
            .expect("Assembly failed");
    }
}

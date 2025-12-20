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
        let asm_lines = codegen.generate(&program).expect("Codegen failed");
        let asm_source = asm_lines.join("\n");
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

        // Check for Helpers
        assert!(asm_source.contains("Runtime_Anim_Update:"));
        assert!(asm_source.contains("Runtime_Anim_Draw:"));

        let assembler = Assembler::new();
        let _rom = assembler
            .assemble(&asm_source, None, vec![])
            .expect("Assembly failed");
    }
}

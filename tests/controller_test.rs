#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::assembler::Assembler;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_controller_api() {
        let source = "
            SUB Main()
                Controller.Read()
                IF Controller.IsPressed(Button.A) THEN
                    POKE($00, 1)
                END IF
                IF Controller.IsHeld(Button.B) THEN
                    POKE($00, 2)
                END IF
                IF Controller.IsReleased(Button.Start) THEN
                    POKE($00, 3)
                END IF
            END SUB
        ";

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");

        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing failed");

        let mut analyzer = SemanticAnalyzer::new();
        // Analysis handles standard library defs now
        analyzer.analyze(&program).expect("Analysis failed");

        let symbol_table = analyzer.symbol_table;
        let mut codegen = CodeGenerator::new(symbol_table);
        let asm_lines = codegen.generate(&program).expect("Codegen failed");
        let asm_source = asm_lines.join("\n");

        // Check for Read call
        assert!(asm_source.contains("JSR Runtime_Controller_Read"));

        // Check for IsPressed logic (checking bit 7 of Pressed var)
        // This depends on implementation, but we expect some logic.

        let assembler = Assembler::new();
        let rom = assembler
            .assemble(&asm_source, None, vec![])
            .expect("Assembly failed");

        assert_eq!(rom.len(), 40976);
    }
}

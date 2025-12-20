#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::assembler::Assembler;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_text_print_literal() {
        let source = "
            SUB Main()
                Text.Print(10, 10, \"HELLO\")
            END SUB
        ";

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing failed");
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(&program).expect("Analysis failed");
        let mut codegen = CodeGenerator::new(analyzer.symbol_table);
        let asm_lines = codegen.generate(&program).expect("Codegen failed");
        let asm_source = asm_lines.join("\n");
        let assembler = Assembler::new();
        let rom = assembler
            .assemble(&asm_source, None, vec![])
            .expect("Assembly failed");

        // Verification:
        // Text.Print(10, 10, "HELLO")
        // Arg 1: 10
        // LDA #10 -> A9 0A
        // LDX #0  -> A2 00 (Integer literal loads Word)
        // STA $14 -> 85 14 (ZP store)

        let pattern_x = vec![0xA9, 0x0A, 0xA2, 0x00, 0x85, 0x14];
        assert!(
            rom.windows(pattern_x.len()).any(|w| w == pattern_x),
            "Did not find Text X setup (A9 0A A2 00 85 14)"
        );

        // Arg 2: 10 -> $15
        let pattern_y = vec![0xA9, 0x0A, 0xA2, 0x00, 0x85, 0x15];
        assert!(
            rom.windows(pattern_y.len()).any(|w| w == pattern_y),
            "Did not find Text Y setup (A9 0A A2 00 85 15)"
        );
    }

    #[test]
    fn test_text_set_offset() {
        let source = "
            SUB Main()
                Text.SetOffset(32)
            END SUB
        ";

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing failed");
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(&program).expect("Analysis failed");
        let mut codegen = CodeGenerator::new(analyzer.symbol_table);
        let asm_lines = codegen.generate(&program).expect("Codegen failed");
        let asm_source = asm_lines.join("\n");
        let assembler = Assembler::new();
        let rom = assembler
            .assemble(&asm_source, None, vec![])
            .expect("Assembly failed");

        // Arg: 32 -> $20
        // LDA #32 -> A9 20
        // LDX #0  -> A2 00
        // STA $18 -> 85 18

        let pattern = vec![0xA9, 0x20, 0xA2, 0x00, 0x85, 0x18];
        assert!(
            rom.windows(pattern.len()).any(|w| w == pattern),
            "Did not find Text Offset setup (A9 20 A2 00 85 18)"
        );
    }
}

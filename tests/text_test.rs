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
        // Target Addr: $2000 + (10*32) + 10 = $214A
        // Should see code that sets PPU addr to $214A and writes chars.
        // Since we are using a Runtime helper, we expect:
        // - Push X, Y, String Ptr
        // - Call Runtime_Text_Print
        // The runtime itself will do the PPU writes.
        // We can verify the arguments are being passed correctly.
        // Arg 1: 10 -> $14 (Text X)
        // Arg 2: 10 -> $15 (Text Y)
        // Arg 3: "HELLO" -> $16/$17 (Text Ptr)

        // Look for:
        // LDA #10 -> A9 0A
        // STA $14 -> 85 14 (or 8D 14 00 if absolute)
        // LDA #10 -> A9 0A
        // STA $15 -> 85 15
        // LDA #<Str
        // STA $16
        // LDA #>Str
        // STA $17
        // JSR Runtime_Text_Print

        // Note: CodeGen usually generates absolute STA ($8D) for ZP if not optimized?
        // Let's check codegen.rs: "STA ${:04X}" -> Absolute.
        // So STA $0014 is 8D 14 00.

        // Pattern: A9 0A ... 8D 14 00 ... A9 0A ... 8D 15 00
        // And call to Runtime_Text_Print.

        // Since label addresses change, we can't search for exact JSR addr.
        // But we can check for the setup code.

        let pattern_x = vec![0xA9, 0x0A, 0x8D, 0x14, 0x00];
        assert!(
            rom.windows(pattern_x.len()).any(|w| w == pattern_x),
            "Did not find Text X setup"
        );

        let pattern_y = vec![0xA9, 0x0A, 0x8D, 0x15, 0x00];
        assert!(
            rom.windows(pattern_y.len()).any(|w| w == pattern_y),
            "Did not find Text Y setup"
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
        // Dest: $18 (Text Offset)
        // LDA #32 -> A9 20
        // STA $18 -> 8D 18 00

        let pattern = vec![0xA9, 0x20, 0x8D, 0x18, 0x00];
        assert!(
            rom.windows(pattern.len()).any(|w| w == pattern),
            "Did not find Text Offset setup"
        );
    }
}

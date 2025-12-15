#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::assembler::Assembler;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_hello_world_background_color() {
        // "Hello World" for NES often means just changing the background color.
        // This program writes to PPU registers to set the background color.
        let source = "
            CONST PPU_ADDR = $2006
            CONST PPU_DATA = $2007
            CONST PPU_MASK = $2001

            SUB Main()
                ' Set Palette Address $3F00
                POKE(PPU_ADDR, $3F)
                POKE(PPU_ADDR, $00)
                ' Write Color ($11 = Blue)
                POKE(PPU_DATA, $11)
                ' Enable Rendering (Show background, no sprites, no clipping)
                POKE(PPU_MASK, %00001010)
            END SUB
        ";

        // 1. Lexing
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");

        // 2. Parsing
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing failed");

        // 3. Analysis
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(&program).expect("Analysis failed");

        // 4. Codegen
        let symbol_table = analyzer.symbol_table;
        let mut codegen = CodeGenerator::new(symbol_table);
        let asm_lines = codegen.generate(&program).expect("Codegen failed");
        let asm_source = asm_lines.join("\n");

        // 5. Assembler
        let assembler = Assembler::new();
        let rom = assembler
            .assemble(&asm_source, None, None)
            .expect("Assembly failed");

        // Verify ROM size (Header + PRG + CHR)
        assert_eq!(rom.len(), 40976);

        // Verification Logic:
        // We look for the sequence of instructions corresponding to the POKEs.
        // Note: The current CodeGenerator emits PHA/PLA even for constants.

        // Pattern 1: POKE(PPU_ADDR, $3F)
        // LDA #$3F -> A9 3F
        // PHA      -> 48
        // PLA      -> 68
        // STA $2006 -> 8D 06 20
        let pattern1 = vec![0xA9, 0x3F, 0x48, 0x68, 0x8D, 0x06, 0x20];
        assert!(
            rom.windows(pattern1.len()).any(|w| w == pattern1),
            "Did not find POKE(PPU_ADDR, $3F)"
        );

        // Pattern 2: POKE(PPU_ADDR, $00)
        // LDA #$00 -> A9 00
        // PHA      -> 48
        // PLA      -> 68
        // STA $2006 -> 8D 06 20
        let pattern2 = vec![0xA9, 0x00, 0x48, 0x68, 0x8D, 0x06, 0x20];
        assert!(
            rom.windows(pattern2.len()).any(|w| w == pattern2),
            "Did not find POKE(PPU_ADDR, $00)"
        );

        // Pattern 3: POKE(PPU_DATA, $11)
        // LDA #$11 -> A9 11
        // PHA      -> 48
        // PLA      -> 68
        // STA $2007 -> 8D 07 20
        let pattern3 = vec![0xA9, 0x11, 0x48, 0x68, 0x8D, 0x07, 0x20];
        assert!(
            rom.windows(pattern3.len()).any(|w| w == pattern3),
            "Did not find POKE(PPU_DATA, $11)"
        );

        // Pattern 4: POKE(PPU_MASK, %00001010) -> $0A
        // LDA #$0A -> A9 0A
        // PHA      -> 48
        // PLA      -> 68
        // STA $2001 -> 8D 01 20
        let pattern4 = vec![0xA9, 0x0A, 0x48, 0x68, 0x8D, 0x01, 0x20];
        assert!(
            rom.windows(pattern4.len()).any(|w| w == pattern4),
            "Did not find POKE(PPU_MASK, %00001010)"
        );
    }
}

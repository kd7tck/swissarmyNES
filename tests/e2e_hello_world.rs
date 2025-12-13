#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::assembler::Assembler;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_hello_world_rom() {
        // A minimal "Hello World" that sets the background color.
        // PPU_ADDR = $2006 (8198), PPU_DATA = $2007 (8199)
        // Palette address = $3F00
        // Color = $16 (Red-ish)
        // PPU_MASK = $2001 (8193)

        let source = "
            CONST PPU_ADDR = 8198  ' $2006
            CONST PPU_DATA = 8199  ' $2007
            CONST BG_COLOR = 22    ' $16 (Red)
            CONST PPU_MASK = 8193  ' $2001

            SUB Main()
                ' Write $3F00 to PPU_ADDR
                POKE(PPU_ADDR, 63)  ' $3F
                POKE(PPU_ADDR, 0)   ' $00

                ' Write Color to PPU_DATA
                POKE(PPU_DATA, BG_COLOR)

                ' Enable Rendering
                POKE(PPU_MASK, 30) ' %00011110
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
        println!("Generated Assembly:\n{}", asm_source);

        // 5. Assembler
        let assembler = Assembler::new();
        let rom = assembler.assemble(&asm_source).expect("Assembly failed");

        // Verify ROM size (Header + 32KB + 8KB) = 16 + 32768 + 8192 = 40976
        assert_eq!(rom.len(), 40976);

        // Header Check
        assert_eq!(rom[0], b'N');
        assert_eq!(rom[1], b'E');
        assert_eq!(rom[2], b'S');
        assert_eq!(rom[3], 0x1A);
    }
}

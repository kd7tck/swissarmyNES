#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::assembler::Assembler;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_e2e_compile_to_rom() {
        let source = "
            CONST MY_VAL = 42
            DIM x AS BYTE

            SUB Main()
                LET x = MY_VAL + 1
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
        // Pass the symbol table from analyzer to codegen
        let symbol_table = analyzer.symbol_table;
        let mut codegen = CodeGenerator::new(symbol_table);
        let asm_lines = codegen.generate(&program).expect("Codegen failed");

        // Join lines with newlines
        let asm_source = asm_lines.join("\n");

        // 5. Assembler
        let assembler = Assembler::new();
        let rom = assembler.assemble(&asm_source, None, None).expect("Assembly failed");

        // Verify we got 40KB (16 Header + 32KB PRG + 8KB CHR)
        assert_eq!(rom.len(), 40976);

        // Debug: Print generated assembly
        println!("Generated Assembly:\n{}", asm_source);

        // Debug: Print ROM dump around beginning (Skip header)
        println!("PRG ROM Dump:");
        for byte in rom.iter().skip(16).take(32) {
            print!("{:02X} ", byte);
        }
        println!();

        // Check for some bytes we expect.
        // x is assigned MY_VAL + 1 = 43 ($2B)
        // CodeGen:
        //   LDA #$2A  (MY_VAL) -> A9 2A
        //   PHA       -> 48
        //   LDA #$01  (1)      -> A9 01
        //   STA $00   -> 85 00 (or 8D 00 00 depending on absolute vs zero page addressing in codegen)
        //   PLA       -> 68
        //   CLC       -> 18
        //   ADC $00   -> 65 00 (or 6D 00 00)
        //   STA x     -> 8D 00 03 (x @ $0300)

        // It won't find LDA #43 because constant folding isn't implemented in AST/Codegen yet!
        // It generates code to calculate 42 + 1 at runtime.

        // So we should look for the sequence:
        // LDA #$2A

        let mut found = false;
        for i in 0..rom.len() - 1 {
            if rom[i] == 0xA9 && rom[i + 1] == 0x2A {
                found = true;
                break;
            }
        }
        assert!(found, "Did not find LDA #42 (0xA9 0x2A) in generated ROM");
    }
}

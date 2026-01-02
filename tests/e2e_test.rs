#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::assembler::Assembler;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;
    use std::collections::HashMap;

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
        let symbol_table = analyzer.symbol_table;
        let mut codegen = CodeGenerator::new(symbol_table);
        let (asm_banks, _) = codegen.generate(&program).expect("Codegen failed");

        let mut assembler_inputs = HashMap::new();
        for (bank, lines) in &asm_banks {
            assembler_inputs.insert(*bank, lines.join("\n"));
        }

        let asm_source = assembler_inputs.values().cloned().collect::<Vec<_>>().join("\n");
        println!("Generated Assembly:\n{}", asm_source);

        // 5. Assembler
        let assembler = Assembler::new();
        let rom = assembler
            .assemble(&assembler_inputs, None, vec![])
            .expect("Assembly failed");

        // Verify we got correct size (16 Header + 128KB PRG + 8KB CHR)
        assert_eq!(rom.len(), 139280);

        // Debug: Print ROM dump around beginning (Skip header)
        println!("PRG ROM Dump:");
        for byte in rom.iter().skip(16).take(32) {
            print!("{:02X} ", byte);
        }
        println!();

        // Check for some bytes we expect.
        // x is assigned MY_VAL + 1 = 43 ($2B)
        // CodeGen: LDA #$2A (MY_VAL), PHA, LDA #$01, ...
        // It won't find LDA #43 because constant folding isn't implemented in AST/Codegen yet!

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

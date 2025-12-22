#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_poke_word_variable() {
        let source = "
            DIM Ptr AS WORD
            SUB Main()
                LET Ptr = $2006
                POKE(Ptr, $3F)
            END SUB
        ";

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

        // Verify Assembly
        // Look for assignment of $2006 to Ptr (at $05C0)
        let found_assignment = asm_lines.iter().any(|line| line.contains("LDA #$06"))
            && asm_lines.iter().any(|line| line.contains("STA $05C0"))
            && asm_lines.iter().any(|line| line.contains("LDA #$20"))
            && asm_lines.iter().any(|line| line.contains("STA $05C1"));

        assert!(
            found_assignment,
            "Assignment of WORD not found in assembly:\n{}",
            asm_source
        );

        // Verify POKE uses indirect addressing
        let found_poke = asm_lines.iter().any(|line| line.contains("STA ($02),y"));
        assert!(found_poke, "Did not find indirect store");

        // We want to ensure we are loading from the variable
        let found_ptr_load = asm_lines.iter().any(|line| line.contains("LDA $05C0"))
            && asm_lines.iter().any(|line| line.contains("STA $02"))
            && asm_lines.iter().any(|line| line.contains("LDA $05C1"))
            && asm_lines.iter().any(|line| line.contains("STA $03"));

        assert!(found_ptr_load, "Did not find 16-bit pointer setup");
    }

    #[test]
    fn test_assign_const_to_word() {
        let source = "
            CONST MyAddr = $2007
            DIM Ptr AS WORD
            SUB Main()
                LET Ptr = MyAddr
            END SUB
        ";

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

        // Verify assignment of constant to WORD
        // Should load #$07 and store to Low, load #$20 and store to High
        let found_assignment = asm_lines.iter().any(|line| line.contains("LDA #$07"))
            && asm_lines.iter().any(|line| line.contains("STA $05C0"))
            && ((asm_lines.iter().any(|line| line.contains("LDA #$20"))
                && asm_lines.iter().any(|line| line.contains("STA $05C1")))
                || (asm_lines.iter().any(|line| line.contains("LDX #$20"))
                    && asm_lines.iter().any(|line| line.contains("STX $05C1"))));

        assert!(
            found_assignment,
            "Assignment of CONST to WORD not found:\n{}",
            asm_source
        );
    }
}

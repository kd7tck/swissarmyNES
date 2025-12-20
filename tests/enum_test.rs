#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_enum_compilation() {
        let source = "
            ENUM State
                Idle
                Run
                Jump = 5
                Fall
            END ENUM

            DIM x AS INT

            SUB Main()
                x = State.Idle
                x = State.Run
                x = State.Jump
                x = State.Fall
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

        println!("{}", asm_source);

        // Verify Assembly contains correct loads
        // State.Idle -> 0
        // LDA #$00 (Low)
        // LDX #$00 (High)
        // STA ...
        assert!(asm_source.contains("LDA #$00"));

        // State.Run -> 1
        assert!(asm_source.contains("LDA #$01"));

        // State.Jump -> 5
        assert!(asm_source.contains("LDA #$05"));

        // State.Fall -> 6 (5 + 1)
        assert!(asm_source.contains("LDA #$06"));
    }

    #[test]
    fn test_enum_negatives() {
        let source = "
            ENUM Signed
                Neg = -5
                NextOne
            END ENUM

            DIM x AS INT

            SUB Main()
                x = Signed.Neg
                x = Signed.NextOne
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

        // Neg = -5
        // LDA #$FB (-5 in 8-bit two's complement)
        // LDX #$FF (Sign extension)
        assert!(asm_source.contains("LDA #$FB"));
        assert!(asm_source.contains("LDX #$FF"));

        // NextOne = -4
        // LDA #$FC
        assert!(asm_source.contains("LDA #$FC"));
    }
}

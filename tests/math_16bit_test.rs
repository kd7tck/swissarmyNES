#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_add_literals_16bit() {
        let source = "
            DIM w AS WORD
            SUB Main()
                LET w = 1000 + 500
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

        // This should now succeed
        let result = codegen.generate(&program);

        match result {
            Ok(lines) => {
                let asm = lines.join("\n");
                // Check for 16-bit logic
                // 1000 = $03E8. 500 = $01F4.
                // 1000 + 500 = 1500 ($05DC).

                // Generated code structure:
                // Push Left (1000)
                // Eval Right (500)
                // Add

                // Look for loading 1000 ($03E8)
                assert!(asm.contains("LDA #$E8"), "Missing Low byte of 1000");
                assert!(asm.contains("LDX #$03"), "Missing High byte of 1000");

                // Look for loading 500 ($01F4)
                assert!(asm.contains("LDA #$F4"), "Missing Low byte of 500");
                assert!(asm.contains("LDX #$01"), "Missing High byte of 500");

                // Look for 16-bit add sequence
                assert!(asm.contains("ADC $00"), "Missing Low Add");
                assert!(asm.contains("ADC $01"), "Missing High Add");
            }
            Err(e) => panic!("Codegen failed: {}", e),
        }
    }
}

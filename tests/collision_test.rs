#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_collision_point_generation() {
        let source = "
            DIM result AS BYTE
            SUB Main()
                IF Collision.Point(10, 10, 0, 0, 20, 20) THEN
                    result = 1
                ELSE
                    result = 0
                END IF
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

        // Verify helper is present
        assert!(asm_source.contains("Runtime_Collision_Point:"));

        // Verify call
        // 6 args * 2 bytes = 12 bytes pushed.
        // We look for PHA sequence or just the JSR.
        assert!(asm_source.contains("JSR Runtime_Collision_Point"));

        // Verify stack cleanup (TSX, TXA, ADC #12, TAX, TXS)
        assert!(asm_source.contains("ADC #12"));
    }

    #[test]
    fn test_collision_tile_generation() {
        let source = "
            DIM t AS BYTE
            SUB Main()
                t = Collision.Tile(100, 50)
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

        // Verify helper is present
        assert!(asm_source.contains("Runtime_Collision_Tile:"));

        // Verify call
        assert!(asm_source.contains("JSR Runtime_Collision_Tile"));

        // Verify stack cleanup (2 args * 2 bytes = 4 bytes)
        assert!(asm_source.contains("ADC #4"));

        // Verify nametable math in helper
        // Look for shifting logic
        assert!(asm_source.contains("AND #$1F")); // Clamp Col
    }
}

#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_collision_rect() {
        let source = "
            DIM c1 AS BOOL
            DIM c2 AS BOOL

            SUB Main()
                ' Box 1: 10,10 10x10 (10-20, 10-20)
                ' Box 2: 15,15 10x10 (15-25, 15-25) -> Overlap
                c1 = Collision.Rect(10, 10, 10, 10, 15, 15, 10, 10)

                ' Box 3: 30,30 10x10 -> No Overlap
                c2 = Collision.Rect(10, 10, 10, 10, 30, 30, 10, 10)
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

        assert!(asm_source.contains("JSR Runtime_Collision_Rect"));
        assert!(asm_source.contains("Collision_False:"));
    }

    #[test]
    fn test_collision_point() {
        let source = "
            DIM c1 AS BOOL
            DIM c2 AS BOOL

            SUB Main()
                ' Rect: 10,10 20x20 (10-30, 10-30)
                ' Point 1: 15,15 -> Inside
                c1 = Collision.Point(15, 15, 10, 10, 20, 20)

                ' Point 2: 5,5 -> Outside
                c2 = Collision.Point(5, 5, 10, 10, 20, 20)
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

        assert!(asm_source.contains("JSR Runtime_Collision_Point"));
        // Stack cleanup (12 bytes)
        assert!(asm_source.contains("ADC #12"));
    }

    #[test]
    fn test_collision_tile() {
        let source = "
            DIM t1 AS BYTE

            SUB Main()
                ' Pixel 16,16 -> Tile 2,2 -> Offset 2*32 + 2 = 66
                t1 = Collision.Tile(16, 16)
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

        assert!(asm_source.contains("JSR Runtime_Collision_Tile"));
        assert!(asm_source.contains("Runtime_Collision_Tile:"));
        // Check for shift instructions (LSR)
        assert!(asm_source.contains("LSR"));
        // Check for offset calculation (ASL/ROL)
        assert!(asm_source.contains("ROL $03"));
        // Stack cleanup (4 bytes)
        assert!(asm_source.contains("ADC #4"));
    }
}

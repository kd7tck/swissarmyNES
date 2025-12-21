#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_struct_declaration_and_usage() {
        let source = "
            TYPE Point
                x AS BYTE
                y AS BYTE
            END TYPE

            DIM p AS Point

            SUB Main()
                p.x = 10
                p.y = 20
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

        // p is at $0490.
        // p.x is at $0490. p.y is at $0491.
        // p.x = 10 -> LDA #$0A; STA $0490
        // p.y = 20 -> LDA #$14; STA $0491

        let asm_str = asm_lines.join("\n");
        assert!(asm_str.contains("LDA #$0A"));
        assert!(asm_str.contains("STA $0490"));
        assert!(asm_str.contains("LDA #$14"));
        assert!(asm_str.contains("STA $0491"));
    }

    #[test]
    fn test_nested_structs() {
        let source = "
            TYPE Point
                x AS BYTE
                y AS BYTE
            END TYPE

            TYPE Rect
                tl AS Point
                br AS Point
            END TYPE

            DIM r AS Rect

            SUB Main()
                r.tl.x = 1
                r.br.y = 2
            END SUB
        ";

        // r @ $0460
        // r.tl @ $0460 (Point)
        // r.tl.x @ $0460
        // r.tl.y @ $0461
        // r.br @ $0462 (Point)
        // r.br.x @ $0462
        // r.br.y @ $0463

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing failed");
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(&program).expect("Analysis failed");

        let symbol_table = analyzer.symbol_table;
        let mut codegen = CodeGenerator::new(symbol_table);
        let asm_lines = codegen.generate(&program).expect("Codegen failed");

        let asm_str = asm_lines.join("\n");

        // r.tl.x = 1 => STA $0490
        assert!(asm_str.contains("LDA #$01"));
        assert!(asm_str.contains("STA $0490"));

        // r.br.y = 2 => STA $0493
        assert!(asm_str.contains("LDA #$02"));
        assert!(asm_str.contains("STA $0493"));
    }
}

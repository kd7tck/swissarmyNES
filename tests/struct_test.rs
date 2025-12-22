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

        // p is at $05C0.
        // p.x is at $05C0. p.y is at $05C1.
        // p.x = 10 -> LDA #$0A; STA $05C0
        // p.y = 20 -> LDA #$14; STA $05C1

        let asm_str = asm_lines.join("\n");
        assert!(asm_str.contains("LDA #$0A"));
        assert!(asm_str.contains("STA $05C0"));
        assert!(asm_str.contains("LDA #$14"));
        assert!(asm_str.contains("STA $05C1"));
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

        // r @ $05C0
        // r.tl @ $05C0 (Point)
        // r.tl.x @ $05C0
        // r.tl.y @ $05C1
        // r.br @ $05C2 (Point)
        // r.br.x @ $05C2
        // r.br.y @ $05C3

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

        // r.tl.x = 1 => STA $05C0
        assert!(asm_str.contains("LDA #$01"));
        assert!(asm_str.contains("STA $05C0"));

        // r.br.y = 2 => STA $05C3
        assert!(asm_str.contains("LDA #$02"));
        assert!(asm_str.contains("STA $05C3"));
    }
}

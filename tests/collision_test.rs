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
}

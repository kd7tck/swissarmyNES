#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_len_literal() {
        let source = "
            DIM L AS WORD
            SUB Main()
                LET L = LEN(\"Hello\")
            END SUB
        ";

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");

        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing failed");

        let mut analyzer = SemanticAnalyzer::new();
        // This should pass once we implement LEN support in Analysis
        analyzer.analyze(&program).expect("Analysis failed");

        let symbol_table = analyzer.symbol_table;
        let mut codegen = CodeGenerator::new(symbol_table);
        let asm_lines = codegen.generate(&program).expect("Codegen failed");

        // Check for call to Runtime_StringLen
        let found_call = asm_lines
            .iter()
            .any(|line| line.contains("JSR Runtime_StringLen"));
        assert!(found_call, "LEN call not found");

        // Verify Runtime_StringLen exists
        let found_routine = asm_lines
            .iter()
            .any(|line| line.contains("Runtime_StringLen:"));
        assert!(found_routine, "Runtime_StringLen helper not found");
    }

    #[test]
    fn test_len_variable() {
        let source = "
            DIM S AS STRING = \"World\"
            DIM L AS WORD
            SUB Main()
                LET L = LEN(S)
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

        // Check for call to Runtime_StringLen
        let found_call = asm_lines
            .iter()
            .any(|line| line.contains("JSR Runtime_StringLen"));
        assert!(found_call, "LEN call not found");
    }

    #[test]
    #[should_panic(expected = "LEN expects a string argument")]
    fn test_len_type_mismatch() {
        let source = "
            DIM L AS WORD
            SUB Main()
                LET L = LEN(123)
            END SUB
        ";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing failed");
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(&program).expect("Analysis failed");
    }
}

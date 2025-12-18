#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_read_string() {
        let source = r#"
            DATA "Hello", "World"
            DIM s1 AS STRING
            DIM s2 AS STRING

            SUB Main()
                READ s1, s2
            END SUB
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");

        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing failed");

        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(&program).expect("Analysis failed");

        let symbol_table = analyzer.symbol_table;
        let mut codegen = CodeGenerator::new(symbol_table);
        let asm_lines = codegen.generate(&program).expect("Codegen failed");

        // Verify DATA generation with null terminator
        // "Hello" -> $48, $65, $6C, $6C, $6F, $00
        let found_hello = asm_lines
            .iter()
            .any(|line| line.contains("db $48, $65, $6C, $6C, $6F, $00"));
        assert!(found_hello, "DATA 'Hello' with null terminator not found");

        // Verify READ call
        let found_call = asm_lines
            .iter()
            .any(|line| line.contains("JSR Runtime_ReadString"));
        assert!(found_call, "JSR Runtime_ReadString not found");

        // Verify Runtime_ReadString implementation
        let found_impl = asm_lines
            .iter()
            .any(|line| line.contains("Runtime_ReadString:"));
        assert!(found_impl, "Runtime_ReadString implementation not found");

        let found_loop = asm_lines
            .iter()
            .any(|line| line.contains("Runtime_ReadString_Loop:"));
        assert!(found_loop, "Runtime_ReadString loop not found");
    }
}

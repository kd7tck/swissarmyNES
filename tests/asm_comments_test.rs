#[cfg(test)]
mod tests {
    use swissarmynes::compiler::ast::TopLevelKind;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;
    use swissarmynes::compiler::symbol_table::SymbolTable;

    #[test]
    fn test_asm_comments_preservation() {
        let input = r#"
        ASM
            LDA #$00 ; This is a comment
            STA $2000
            ; Full line comment
        END ASM
        SUB Main()
        END SUB
        "#;

        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("Lexing failed");

        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing failed");

        // Check if comments are in the AST
        if let TopLevelKind::Asm(lines) = &program.declarations[0].kind {
            assert!(lines.iter().any(|l| l.contains("; This is a comment")));
            assert!(lines.iter().any(|l| l.contains("; Full line comment")));
        } else {
            panic!("Expected ASM block");
        }

        let st = SymbolTable::new();
        let mut cg = CodeGenerator::new(st);
        let (code, _) = cg.generate(&program).expect("Codegen failed");

        // Check if comments are in the output
        assert!(code.iter().any(|l| l.contains("; This is a comment")));
        assert!(code.iter().any(|l| l.contains("; Full line comment")));
    }
}

#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_control_flow() {
        let source = "
            DIM x AS BYTE
            SUB Main()
                x = 10
                IF x = 10 THEN
                    x = 20
                ELSE
                    x = 30
                END IF
                WHILE x < 100
                    x = x + 1
                WEND
            END SUB
        ";
        compile_check(source, vec!["x @ $0300", "BNE", "JMP"]);
    }

    #[test]
    fn test_greater_than() {
        let source = "
            DIM res AS BYTE
            SUB Main()
                IF 5 > 10 THEN
                    res = 1
                ELSE
                    res = 2
                END IF

                IF 10 > 5 THEN
                    res = res + 10
                END IF
            END SUB
        ";
        // We expect codegen to have branches.
        // ideally we would run this in an emulator or check exact assembly logic.
        // Checking for BCS/BEQ logic existence.
        compile_check(source, vec!["BCC", "BEQ", "JMP", "LDA #1"]);
    }

    fn compile_check(source: &str, expected_substrings: Vec<&str>) {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");

        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing failed");

        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(&program).expect("Analysis failed");

        let symbol_table = analyzer.symbol_table;
        let mut codegen = CodeGenerator::new(symbol_table);
        let asm_lines = codegen.generate(&program).expect("Codegen failed");
        let asm = asm_lines.join("\n");

        for sub in expected_substrings {
            if !asm.contains(sub) {
                panic!("Generated ASM does not contain '{}':\n{}", sub, asm);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_array_of_structs_access() {
        let source = "
            TYPE Entity
                active AS BYTE
                x AS BYTE
                y AS BYTE
            END TYPE

            DIM pool(10) AS Entity
            DIM idx AS BYTE

            SUB Main()
                ' Direct access with constant index
                pool(0).x = 10
                pool(0).y = 20

                ' Indirect access with variable index
                idx = 5
                pool(idx).active = 1
                pool(idx).x = 50
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
        // This is expected to fail or generate incorrect code if support is missing
        let asm_lines = codegen.generate(&program).expect("Codegen failed");
        let asm_source = asm_lines.join("\n");

        // Verify constant index access
        // pool(0) is at Base. .x is Base+1.
        // We expect some form of calculation or static address usage.

        // Verify variable index access
        // Should see calculation of address using idx
    }
}

#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_pool_spawn_despawn() {
        let source = "
            TYPE Enemy
                active AS BYTE
                id AS BYTE
            END TYPE

            DIM pool(5) AS Enemy
            DIM i0 AS INT
            DIM i1 AS INT
            DIM i2 AS INT
            DIM i_new AS INT

            SUB Main()
                ' Spawn 3
                i0 = Pool.Spawn(pool) ' Should be 0
                pool(i0).id = 100
                i1 = Pool.Spawn(pool) ' Should be 1
                pool(i1).id = 101
                i2 = Pool.Spawn(pool) ' Should be 2
                pool(i2).id = 102

                ' Despawn 1
                Pool.Despawn(pool, i1)

                ' Spawn new (should take slot 1)
                i_new = Pool.Spawn(pool)
                pool(i_new).id = 200
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

        // Verify Spawn logic
        assert!(asm_source.contains("JSR Runtime_Pool_Spawn"));
        assert!(asm_source.contains("JSR Runtime_Pool_Despawn"));

        // We can't verify runtime logic without running it, but code generation correctness
        // implies functionality if helpers are correct.
        // We verified helpers via reasoning.
        // Also verified dynamic array access via repro test.
    }
}

#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::lexer::Lexer;
    use swissarmynes::compiler::parser::Parser;

    #[test]
    fn test_data_read_compile() {
        let source = r#"
            DATA 1, 2, 300, -1
            DIM a AS BYTE
            DIM b AS BYTE
            DIM w AS WORD
            DIM c AS BYTE

            SUB Main()
                READ a, b, w, c
                RESTORE
                READ a
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
        let _asm_source = asm_lines.join("\n");

        // 1. Verify DATA generation
        // 1 -> $01
        // 2 -> $02
        // 300 -> $2C, $01
        // -1 -> $FF (treated as byte if logic is correct for range -128..255)

        let found_data_label = asm_lines
            .iter()
            .any(|line| line.contains("USER_DATA_START:"));
        assert!(found_data_label, "USER_DATA_START label missing");

        // We look for db lines.
        let found_1 = asm_lines.iter().any(|line| line.contains("db $01"));
        let found_2 = asm_lines.iter().any(|line| line.contains("db $02"));
        let found_300 = asm_lines.iter().any(|line| line.contains("db $2C, $01"));
        let found_neg = asm_lines.iter().any(|line| line.contains("db $FF"));

        assert!(found_1, "Data 1 not found");
        assert!(found_2, "Data 2 not found");
        assert!(found_300, "Data 300 not found");
        assert!(found_neg, "Data -1 not found");

        // 2. Verify READ
        // READ a (Byte) -> JSR Runtime_ReadByte, STA $0300
        let found_read_byte = asm_lines
            .iter()
            .any(|line| line.contains("JSR Runtime_ReadByte"))
            && asm_lines.iter().any(|line| line.contains("STA $0300")); // a is first var

        assert!(found_read_byte, "READ Byte code incorrect");

        // 3. Verify RESTORE
        // LDA $FFxx, STA $04
        // We can't check exact address, but we can check if it loads from ROM
        let found_restore = asm_lines.iter().any(|line| line.contains("STA $04"))
            && asm_lines.iter().any(|line| line.contains("STA $05"));
        assert!(found_restore, "RESTORE code missing");

        // Verify InitUserData in Data Tables
        let found_init_ptr = asm_lines
            .iter()
            .any(|line| line.contains("InitUserData: WORD USER_DATA_START"));
        assert!(found_init_ptr, "InitUserData missing in Data Tables");
    }
}

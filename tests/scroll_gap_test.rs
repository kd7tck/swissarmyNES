#[cfg(test)]
mod tests {
    use swissarmynes::compiler::{
        analysis::SemanticAnalyzer, ast::Program, codegen::CodeGenerator, lexer::Lexer,
        parser::Parser, symbol_table::SymbolTable,
    };

    fn parse_fixture(source: &str) -> (Program, SymbolTable) {
        let mut lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer.tokenize().unwrap());
        let program = parser.parse().unwrap();
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(&program).unwrap();
        (program, analyzer.symbol_table)
    }

    #[test]
    fn test_scroll_load_row_asm() {
        // Reachability pruning retains only helpers requested by real source.
        let (program, symbol_table) =
            parse_fixture("DIM arr(32) AS BYTE\nSUB Main()\nScroll.LoadRow(480, arr)\nEND SUB\n");
        let mut codegen = CodeGenerator::new(symbol_table);

        let (asm, _) = codegen.generate(&program).unwrap();
        let asm_str = asm.join("\n");

        // Extract Runtime_Scroll_LoadRow
        if let Some(start) = asm_str.find("Runtime_Scroll_LoadRow:") {
            if let Some(end_offset) = asm_str[start..].find("Scroll_LoadRow_Done:") {
                let routine = &asm_str[start..start + end_offset];
                println!("{}", routine);

                // Check for gap skipping logic
                assert!(
                    routine.contains("JSR Runtime_Scroll_NormalizeY"),
                    "Runtime_Scroll_LoadRow must normalize repeated 240-pixel screens"
                );
                assert!(
                    asm_str.contains("Scroll_Row_Gap_Skip"),
                    "Runtime scroll normalization missing terminal gap-skip label"
                );
            } else {
                panic!("Scroll_LoadRow_Done: label not found");
            }
        } else {
            panic!("Runtime_Scroll_LoadRow not found");
        }
    }

    #[test]
    fn test_scroll_load_column_asm() {
        let (program, symbol_table) = parse_fixture(
            "DIM arr(30) AS BYTE\nSUB Main()\nScroll.LoadColumn(256, arr)\nEND SUB\n",
        );
        let mut codegen = CodeGenerator::new(symbol_table);
        let (asm, _) = codegen.generate(&program).unwrap();
        let asm_str = asm.join("\n");

        if let Some(start) = asm_str.find("Runtime_Scroll_LoadColumn:") {
            if let Some(end_offset) = asm_str[start..].find("Scroll_LoadColumn_Done:") {
                let routine = &asm_str[start..start + end_offset];
                println!("{}", routine);

                // It should NOT compare X ($02) against 240 ($F0)
                assert!(
                    !routine.contains("CMP #$F0"),
                    "Runtime_Scroll_LoadColumn SHOULD NOT check CMP #$F0"
                );
            } else {
                panic!("Scroll_LoadColumn_Done: label not found");
            }
        } else {
            panic!("Runtime_Scroll_LoadColumn not found");
        }
    }
}

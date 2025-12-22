#[cfg(test)]
mod tests {
    use swissarmynes::compiler::{
        ast::{DataType, Program},
        codegen::CodeGenerator,
        symbol_table::SymbolTable,
    };

    #[test]
    fn test_scroll_load_row_asm() {
        let mut symbol_table = SymbolTable::new();
        // Define 'arr'
        symbol_table
            .define(
                "arr".to_string(),
                DataType::Array(Box::new(DataType::Byte), 32),
                swissarmynes::compiler::symbol_table::SymbolKind::Variable,
            )
            .unwrap();
        symbol_table.assign_address("arr", 0x0500).unwrap();

        let mut codegen = CodeGenerator::new(symbol_table);

        let program = Program {
            declarations: vec![],
        };

        let asm = codegen.generate(&program).unwrap();
        let asm_str = asm.join("\n");

        // Extract Runtime_Scroll_LoadRow
        if let Some(start) = asm_str.find("Runtime_Scroll_LoadRow:") {
            if let Some(end_offset) = asm_str[start..].find("Scroll_LoadRow_Done:") {
                let routine = &asm_str[start..start + end_offset];
                println!("{}", routine);

                // Check for gap skipping logic
                assert!(
                    routine.contains("CMP #$F0"),
                    "Runtime_Scroll_LoadRow missing CMP #$F0 check"
                );
                assert!(
                    routine.contains("Scroll_Row_Gap_Skip"),
                    "Runtime_Scroll_LoadRow missing Gap Skip Label"
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
        let mut symbol_table = SymbolTable::new();
        let mut codegen = CodeGenerator::new(symbol_table);
        let program = Program {
            declarations: vec![],
        };
        let asm = codegen.generate(&program).unwrap();
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

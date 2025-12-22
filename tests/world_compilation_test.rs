#[cfg(test)]
mod tests {
    use swissarmynes::compiler::analysis::SemanticAnalyzer;
    use swissarmynes::compiler::ast::{Program, TopLevel};
    use swissarmynes::compiler::codegen::CodeGenerator;

    #[test]
    fn test_world_compilation() {
        let world_decl = TopLevel::World(
            2,
            2,                // 2x2 grid
            vec![0, 1, 2, 3], // Nametable indices
        );

        let program = Program {
            declarations: vec![world_decl],
        };

        // Analysis
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(&program).expect("Analysis failed");

        // Codegen
        let symbol_table = analyzer.symbol_table;
        let mut codegen = CodeGenerator::new(symbol_table);
        let asm_lines = codegen.generate(&program).expect("Codegen failed");
        let asm_source = asm_lines.join("\n");

        assert!(asm_source.contains("World_Map:"));
        // Width = 2 ($02 $00)
        assert!(asm_source.contains("db $02, $00"));
        // Height = 2 ($02 $00)
        // Data = 0, 1, 2, 3
        assert!(asm_source.contains("db $00, $01, $02, $03"));

        // Check data table pointer
        assert!(asm_source.contains("Ptr_World_Map: WORD World_Map"));
    }

    #[test]
    fn test_metatile_compilation() {
        let mt_decl = TopLevel::Metatile(
            "Grass".to_string(),
            [10, 11, 12, 13], // Tiles
            1,                // Attr
        );

        let program = Program {
            declarations: vec![mt_decl],
        };

        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(&program).expect("Analysis failed");

        let symbol_table = analyzer.symbol_table;
        let mut codegen = CodeGenerator::new(symbol_table);
        let asm_lines = codegen.generate(&program).expect("Codegen failed");
        let asm_source = asm_lines.join("\n");

        assert!(asm_source.contains("Grass:"));
        assert!(asm_source.contains("db $0A, $0B, $0C, $0D, $01"));
        assert!(asm_source.contains("Ptr_Grass: WORD Grass"));
    }
}

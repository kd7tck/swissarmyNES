#[cfg(test)]
mod tests {
    use swissarmynes::compiler::ast::Program;
    use swissarmynes::compiler::ast::{DataType, Expression, TopLevel};
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::symbol_table::{SymbolKind, SymbolTable};

    #[test]
    fn test_dim_initialization() {
        let program = Program {
            declarations: vec![
                TopLevel::Dim(
                    "x".to_string(),
                    DataType::Byte,
                    Some(Expression::Integer(42)),
                ),
                TopLevel::Dim(
                    "w".to_string(),
                    DataType::Word,
                    Some(Expression::Integer(300)),
                ),
                TopLevel::Dim(
                    "i".to_string(),
                    DataType::Int,
                    Some(Expression::Integer(100)),
                ),
            ],
        };

        let mut symbol_table = SymbolTable::new();
        symbol_table
            .define("x".to_string(), DataType::Byte, SymbolKind::Variable)
            .unwrap();
        symbol_table
            .define("w".to_string(), DataType::Word, SymbolKind::Variable)
            .unwrap();
        symbol_table
            .define("i".to_string(), DataType::Int, SymbolKind::Variable)
            .unwrap();

        let mut codegen = CodeGenerator::new(symbol_table);
        let (output, _) = codegen.generate(&program).expect("CodeGen failed");

        // Startup/Init code is now in Bank 7
        let asm = output.get(&7).expect("Bank 7 missing").join("\n");
        println!("{}", asm);

        // 1. Check Byte Init (x = 42)
        assert!(asm.contains("Init x @"), "Missing x init comment");
        assert!(asm.contains("LDA #$2A"), "Missing LDA #$2A for x");

        // 2. Check Word Init (w = 300 = $012C)
        assert!(asm.contains("Init w @"), "Missing w init comment");
        assert!(asm.contains("LDA #$2C"), "Missing LDA #$2C for w");
        assert!(asm.contains("STX"), "Missing STX for w (High Byte)");

        // 3. Check Int Init (i = 100 = $64)
        assert!(asm.contains("Init i @"), "Missing i init comment");
        assert!(asm.contains("LDA #$64"), "Missing LDA #$64 for i");
    }
}

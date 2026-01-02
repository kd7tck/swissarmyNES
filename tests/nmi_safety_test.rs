#[cfg(test)]
mod tests {
    use swissarmynes::compiler::{
        ast::{DataType, Expression, Program, Statement, TopLevel},
        codegen::CodeGenerator,
        symbol_table::SymbolTable,
    };

    #[test]
    fn test_nmi_safety_generation() {
        // Create a simple program with an NMI interrupt
        // INTERRUPT NMI
        //   LET x = 1
        // END INTERRUPT

        let program = Program {
            declarations: vec![
                TopLevel::Dim("x".to_string(), DataType::Byte, None),
                TopLevel::Interrupt(
                    "NMI".to_string(),
                    vec![Statement::Let(
                        Expression::Identifier("x".to_string()),
                        Expression::Integer(1),
                    )],
                ),
            ],
        };

        let mut symbol_table = SymbolTable::new();
        // Define symbols manually to skip Analysis phase
        symbol_table
            .define(
                "x".to_string(),
                DataType::Byte,
                swissarmynes::compiler::symbol_table::SymbolKind::Variable,
            )
            .unwrap();
        symbol_table
            .define(
                "NMI".to_string(),
                DataType::Byte,
                swissarmynes::compiler::symbol_table::SymbolKind::Sub,
            )
            .unwrap();

        let mut codegen = CodeGenerator::new(symbol_table);
        let (asm_banks, _) = codegen.generate(&program).expect("Codegen failed");

        // TrampolineNMI is in Fixed Bank 7
        let bank7 = asm_banks.get(&7).expect("Bank 7 missing");

        // Verify TrampolineNMI exists and has safe context saving
        let trampoline_idx = bank7
            .iter()
            .position(|line| line == "TrampolineNMI:")
            .unwrap();
        let trampoline_code = &bank7[trampoline_idx..];

        // Check for saving $00-$0F
        assert!(trampoline_code.iter().any(|line| line.contains("LDA $00")));
        assert!(trampoline_code.iter().any(|line| line.contains("LDA $0F")));

        // Check for JSR CallUserNMI
        assert!(trampoline_code
            .iter()
            .any(|line| line.contains("JSR CallUserNMI")));

        // Verify NMI handler (in Bank 0) ends in RTS (not RTI)
        let bank0 = asm_banks.get(&0).expect("Bank 0 missing");
        let nmi_idx = bank0.iter().position(|line| line == "NMI:").unwrap();
        // Look for next RTS/RTI
        let return_idx = bank0[nmi_idx..]
            .iter()
            .position(|line| line.contains("RTS") || line.contains("RTI"))
            .unwrap();
        let return_instr = &bank0[nmi_idx + return_idx];

        assert_eq!(
            return_instr.trim(),
            "RTS",
            "INTERRUPT block should end with RTS to support Trampoline"
        );
    }
}

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
        let (asm, _) = codegen.generate(&program).expect("Codegen failed");

        // Verify TrampolineNMI exists and has safe context saving
        let trampoline_idx = asm
            .iter()
            .position(|line| line == "TrampolineNMI:")
            .unwrap();
        let trampoline_code = &asm[trampoline_idx..];

        // Sparse indexed loops cover temporary ranges without rolling back
        // persistent controller, text, sprite, scroll or RNG state.
        for instruction in [
            "LDX #0",
            "LDY InterruptScratchAddresses,X",
            "LDA $0000,Y",
            "CPX #28",
            "BNE Save_NMI_Scratch",
            "LDX #27",
            "STA $0000,Y",
            "BPL Restore_NMI_Scratch",
            "STA $07F1",
        ] {
            assert!(trampoline_code
                .iter()
                .any(|line| line.trim() == instruction));
        }

        // Check for JSR CallUserNMI
        assert!(trampoline_code
            .iter()
            .any(|line| line.contains("JSR CallUserNMI")));

        // Verify NMI handler ends in RTS (not RTI)
        let nmi_idx = asm.iter().position(|line| line == "NMI:").unwrap();
        // Look for next RTS/RTI
        let return_idx = asm[nmi_idx..]
            .iter()
            .position(|line| line.contains("RTS") || line.contains("RTI"))
            .unwrap();
        let return_instr = &asm[nmi_idx + return_idx];

        assert_eq!(
            return_instr.trim(),
            "RTS",
            "INTERRUPT block should end with RTS to support Trampoline"
        );
    }
}

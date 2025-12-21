#[cfg(test)]
mod tests {
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::symbol_table::SymbolTable;
    use swissarmynes::compiler::ast::{Program, TopLevel, Statement, Expression, DataType};
    use swissarmynes::compiler::symbol_table::SymbolKind;

    #[test]
    fn test_heap_slot_carry_logic() {
        // This test ensures Runtime_GetHeapSlot handles page crossing correctly
        let mut st = SymbolTable::new();
        st.define("s".to_string(), DataType::String, SymbolKind::Variable).unwrap();

        // Compile a dummy program that uses string concatenation to trigger Heap helper generation
        let program = Program {
            declarations: vec![
                TopLevel::Dim("s".to_string(), DataType::String, None),
                TopLevel::Sub("Main".to_string(), vec![], vec![
                    Statement::Let(
                        Expression::Identifier("s".to_string()),
                        Expression::BinaryOp(
                            Box::new(Expression::StringLiteral("A".to_string())),
                            swissarmynes::compiler::ast::BinaryOperator::Add,
                            Box::new(Expression::StringLiteral("B".to_string()))
                        )
                    )
                ])
            ],
        };

        let mut cg = CodeGenerator::new(st);
        let code = cg.generate(&program).expect("Codegen failed");

        // Find Runtime_GetHeapSlot
        let start_idx = code.iter().position(|line| line == "Runtime_GetHeapSlot:").expect("Helper not found");
        // Find RTS after start
        let end_idx = code.iter().skip(start_idx).position(|line| line == "  RTS").expect("End of helper not found") + start_idx;
        let body = &code[start_idx..=end_idx];

        // Verify logic:
        // ADC #$C0
        // LDX #$03
        // BCC ...
        // INX

        let has_adc = body.iter().any(|line| line.contains("ADC #$C0"));
        let has_bcc = body.iter().any(|line| line.contains("BCC Heap_NoCarry"));
        let has_inx = body.iter().any(|line| line == "  INX");

        assert!(has_adc, "ADC instruction missing (checking for $C0 constant logic)");
        assert!(has_bcc, "BCC instruction missing (Carry check)");
        assert!(has_inx, "INX instruction missing (High byte increment)");
    }
}

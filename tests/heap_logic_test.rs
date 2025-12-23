#[cfg(test)]
mod tests {
    use swissarmynes::compiler::ast::{DataType, Expression, Program, Statement, TopLevel};
    use swissarmynes::compiler::codegen::CodeGenerator;
    use swissarmynes::compiler::symbol_table::SymbolKind;
    use swissarmynes::compiler::symbol_table::SymbolTable;

    #[test]
    fn test_heap_slot_carry_logic() {
        // This test ensures Runtime_GetHeapSlot handles page crossing correctly
        let mut st = SymbolTable::new();
        st.define("s".to_string(), DataType::String, SymbolKind::Variable)
            .unwrap();

        // Compile a dummy program that uses string concatenation to trigger Heap helper generation
        let program = Program {
            declarations: vec![
                TopLevel::Dim("s".to_string(), DataType::String, None),
                TopLevel::Sub(
                    "Main".to_string(),
                    vec![],
                    vec![Statement::Let(
                        Expression::Identifier("s".to_string()),
                        Expression::BinaryOp(
                            Box::new(Expression::StringLiteral("A".to_string())),
                            swissarmynes::compiler::ast::BinaryOperator::Add,
                            Box::new(Expression::StringLiteral("B".to_string())),
                        ),
                    )],
                ),
            ],
        };

        let mut cg = CodeGenerator::new(st);
        let code = cg.generate(&program).expect("Codegen failed");

        // Find Runtime_GetHeapSlot
        let start_idx = code
            .iter()
            .position(|line| line == "Runtime_GetHeapSlot:")
            .expect("Helper not found");
        // Find RTS after start
        let end_idx = code
            .iter()
            .skip(start_idx)
            .position(|line| line == "  RTS")
            .expect("End of helper not found")
            + start_idx;
        let body = &code[start_idx..=end_idx];

        // Verify logic:
        // AND #$1F (Mask Index)
        // AND #$10 (High Bit Check)
        // AND #$0F (Low Bits Check)
        // ADC #$C0 (Base Low)
        // ADC #$03 (Base High)

        let has_mask = body.iter().any(|line| line.contains("AND #$1F"));
        let has_high_calc = body.iter().any(|line| line.contains("AND #$10"));
        let has_low_calc = body.iter().any(|line| line.contains("AND #$0F"));
        let has_adc_low = body.iter().any(|line| line.contains("ADC #$C0"));
        let has_adc_high = body.iter().any(|line| line.contains("ADC #$03"));

        assert!(has_mask, "Missing Index Masking");
        assert!(has_high_calc, "Missing High Offset Calculation");
        assert!(has_low_calc, "Missing Low Offset Calculation");
        assert!(has_adc_low, "Missing Base Low Addition");
        assert!(has_adc_high, "Missing Base High Addition");
    }
}

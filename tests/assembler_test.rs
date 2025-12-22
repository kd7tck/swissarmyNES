#[cfg(test)]
mod tests {
    use swissarmynes::compiler::assembler::Assembler;

    #[test]
    fn test_overlap_detection() {
        let assembler = Assembler::new();

        // Code at $8000
        let source = "
            .ORG $8000
            LDA #$FF
            STA $00
        ";
        // Injection at $8000, overwriting code
        let injections = vec![(0x8000, vec![0x00, 0x01])];

        let result = assembler.assemble(source, None, injections);
        assert!(result.is_err(), "Assembler should detect overlap between code and injection");
    }

    #[test]
    fn test_injection_overlap() {
        let assembler = Assembler::new();
        let source = ".ORG $8000\nLDA #0";

        // Injection 1: $D000, len 10
        // Injection 2: $D005, len 10 (Overlap $D005-$D009)
        let injections = vec![
            (0xD000, vec![0; 10]),
            (0xD005, vec![0; 10]),
        ];

        let result = assembler.assemble(source, None, injections);
        assert!(result.is_err(), "Assembler should detect overlap between injections");
    }
}

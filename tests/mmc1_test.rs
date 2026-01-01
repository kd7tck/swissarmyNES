
use swissarmynes::compiler::assembler::Assembler;
use std::collections::HashMap;

#[test]
fn test_mmc1_banking_compilation() {
    // Valid SwissBASIC with BANK directives
    // DIM must be top-level
    let source = r#"
    BANK 0
    DIM x AS BYTE

    SUB Bank0Sub()
        LET x = 10
    END SUB

    BANK 1
    DIM y AS BYTE

    SUB Bank1Sub()
        LET y = 20
    END SUB

    BANK 0
    SUB Main()
        ' Dummy Main
    END SUB
    "#;

    // Use the public API compile_source
    let result = swissarmynes::server::api::compile_source(Some(source.to_string()), None, None);

    match result {
        Ok((rom, _)) => {
            // Check ROM Size.
            // 128KB PRG + 8KB CHR + 16B Header = 131072 + 8192 + 16 = 139280
            assert_eq!(rom.len(), 139280, "ROM size should be 128KB PRG + 8KB CHR + Header");

            // Check Header
            // Mapper 1 (MMC1) -> Byte 6 low nibble = 1
            // Byte 6: Mapper Low (4-7), Flags 6 (0-3).
            // Our assembler sets it to 0x11 (Vertical Mirroring | Mapper 1)
            assert_eq!(rom[6] & 0xF0, 0x10, "Mapper Lower Nibble should be 1");
            // PRG Size -> Byte 4 = 8 (128KB / 16KB)
            assert_eq!(rom[4], 0x08, "PRG Size should be 8 banks");
        },
        Err(e) => panic!("Compilation failed: {}", e),
    }
}

#[test]
fn test_assembler_mmc1_layout() {
    // This tests the Assembler's ability to stitch banks.
    let assembler = Assembler::new();
    let mut sources: HashMap<u8, String> = HashMap::new();

    // Bank 0 Source (Switchable)
    // This should land at offset 0 (after header)
    sources.insert(0, "
        .ORG $8000
        LDA #$AA
        STA $00
    ".to_string());

    // Bank 7 Source (Fixed/System)
    // This should land at offset 7 * 16KB (after header)
    sources.insert(7, "
        .ORG $C000
        LDA #$BB
        STA $01
    ".to_string());

    let rom = assembler.assemble(&sources, None, vec![]).expect("Failed to assemble");

    let header_size = 16;
    let bank_size = 16384;

    // Check Bank 0 content
    // LDA #$AA is A9 AA.
    // Offset 0 in Bank 0 is header + 0.
    assert_eq!(rom[header_size], 0xA9);
    assert_eq!(rom[header_size + 1], 0xAA);

    // Check Bank 7 content
    // Offset 0 in Bank 7 is header + 7 * 16KB.
    let bank7_offset = header_size + 7 * bank_size;
    assert_eq!(rom[bank7_offset], 0xA9);
    assert_eq!(rom[bank7_offset + 1], 0xBB);
}

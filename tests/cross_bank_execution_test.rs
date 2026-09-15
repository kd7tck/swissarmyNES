use swiss_emulator::Emulator;
use swissarmynes::server::api::compile_source;

#[test]
fn main_in_switchable_bank_and_interrupt_in_fixed_bank_execute() {
    let source = "DIM ready AS BYTE\nDIM ticks AS BYTE\nBANK 3\nSUB Main()\nLET ready = 42\nPOKE($2000, $80)\nEND SUB\nINTERRUPT NMI()\nLET ticks = ticks + 1\nEND INTERRUPT";
    let (rom, _) = compile_source(Some(source.into()), None, None).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();
    for _ in 0..10 {
        emu.step().expect("Execution failed");
    }
    let ram = emu.ram_snapshot();
    assert_eq!(ram[0x5c0], 42);
    assert!(ram[0x5c1] > 0, "NMI handler should execute after startup");
    assert!(u16::from_le_bytes([ram[0x7f4], ram[0x7f5]]) >= 0xc000);
}

#[test]
fn test_vector_layout_in_bank7() {
    let source = r#"
        SUB Main()
        END SUB
    "#;

    let (rom_bytes, _) = compile_source(Some(source.to_string()), None, None).unwrap();
    let prg_end = 16 + 131072;
    let vectors = &rom_bytes[prg_end - 6..prg_end];

    let vec_nmi = (vectors[0] as u16) | ((vectors[1] as u16) << 8);
    let vec_reset = (vectors[2] as u16) | ((vectors[3] as u16) << 8);
    let vec_irq = (vectors[4] as u16) | ((vectors[5] as u16) << 8);

    assert!(
        vec_reset >= 0xC000,
        "VecReset (${:04X}) must point to Bank 7 ($C000+)",
        vec_reset
    );
    assert!(
        vec_nmi >= 0xC000,
        "VecNMI (${:04X}) must point to Bank 7 ($C000+)",
        vec_nmi
    );
    assert!(
        vec_irq >= 0xC000,
        "VecIRQ (${:04X}) must point to Bank 7 ($C000+)",
        vec_irq
    );
}

#[test]
fn test_cross_bank_nested_calls_execution() {
    let source = r#"
        DIM flag0 AS BYTE
        DIM flag3 AS BYTE
        DIM flag5 AS BYTE
        DIM p AS WORD

        BANK 3
        SUB Padding()
            LET p = 1234
            LET p = p + 1
        END SUB

        SUB Foo()
            LET flag3 = 33
            Bar()
        END SUB

        BANK 5
        SUB Bar()
            LET flag5 = 55
        END SUB

        BANK 0
        SUB Main()
            Foo()
            LET flag0 = 11
        END SUB
    "#;

    let (rom_bytes, _) = compile_source(Some(source.to_string()), None, None).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom_bytes).unwrap();

    for _ in 0..30 {
        emu.step().expect("Emulator execution failed");
    }

    let wram = emu.ram_snapshot();

    assert_eq!(wram[0x05C0], 11, "flag0 in Bank 0 should be 11");
    assert_eq!(wram[0x05C1], 33, "flag3 in Bank 3 should be 33");
    assert_eq!(wram[0x05C2], 55, "flag5 in Bank 5 should be 55");

    // Reserved PRG bank location $07F0 must equal 0 (returned to caller Bank 0)
    assert_eq!(
        wram[0x07F0], 0,
        "current_prg_bank at $07F0 should return to 0"
    );
}

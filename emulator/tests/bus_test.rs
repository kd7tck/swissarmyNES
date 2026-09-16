use swiss_emulator::Emulator;

fn make_nrom_rom(code: &[u8]) -> Vec<u8> {
    let mut rom = vec![0; 16 + 16384 + 8192];
    rom[..8].copy_from_slice(&[b'N', b'E', b'S', 0x1a, 1, 1, 0, 0]);
    rom[16..16 + code.len()].copy_from_slice(code);
    rom[16 + 0x3ffc..16 + 0x3ffe].copy_from_slice(&0x8000u16.to_le_bytes());
    rom
}

#[test]
fn test_internal_ram_and_mirroring() {
    // 6502 instructions:
    // 1: LDA #$A5
    // 2: STA $0000
    // 3: LDA #$5A
    // 4: STA $03FF
    // 5: LDA #$FF
    // 6: STA $07FF
    // 7: NOP
    let code = [
        0xa9, 0xa5, 0x8d, 0x00, 0x00, // LDA #$A5, STA $0000
        0xa9, 0x5a, 0x8d, 0xff, 0x03, // LDA #$5A, STA $03FF
        0xa9, 0xff, 0x8d, 0xff, 0x07, // LDA #$FF, STA $07FF
        0xea, // NOP
    ];
    let rom = make_nrom_rom(&code);
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();

    // Execute 6 instructions to run all 3 STAs
    for _ in 0..6 {
        emu.trace_instruction().unwrap();
    }

    // Verify values at original RAM addresses
    assert_eq!(emu.peek_cpu(0x0000), 0xa5);
    assert_eq!(emu.peek_cpu(0x03ff), 0x5a);
    assert_eq!(emu.peek_cpu(0x07ff), 0xff);

    // Verify 3 mirrors of $0000 ($0800, $1000, $1800)
    assert_eq!(emu.peek_cpu(0x0800), 0xa5);
    assert_eq!(emu.peek_cpu(0x1000), 0xa5);
    assert_eq!(emu.peek_cpu(0x1800), 0xa5);

    // Verify 3 mirrors of $03FF ($0bff, $13ff, $1bff)
    assert_eq!(emu.peek_cpu(0x0bff), 0x5a);
    assert_eq!(emu.peek_cpu(0x13ff), 0x5a);
    assert_eq!(emu.peek_cpu(0x1bff), 0x5a);

    // Verify 3 mirrors of $07FF ($0fff, $17ff, $1fff)
    assert_eq!(emu.peek_cpu(0x0fff), 0xff);
    assert_eq!(emu.peek_cpu(0x17ff), 0xff);
    assert_eq!(emu.peek_cpu(0x1fff), 0xff);
}

#[test]
fn test_ppu_register_mirroring() {
    // Write to PPUCTRL via mirror $2008 and PPUMASK via mirror $2009
    let code = [
        0xa9, 0x90, 0x8d, 0x08, 0x20, // LDA #$90, STA $2008 -> PPUCTRL ($2000)
        0xa9, 0x1e, 0x8d, 0x09, 0x20, // LDA #$1E, STA $2009 -> PPUMASK ($2001)
        0xea, // NOP
    ];
    let rom = make_nrom_rom(&code);
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();

    // Execute 4 instructions
    for _ in 0..4 {
        emu.trace_instruction().unwrap();
    }

    // Verify peek does not panic or mutate unexpected state across PPU mirrors
    assert_eq!(emu.peek_cpu(0x2000), emu.peek_cpu(0x2008));
    assert_eq!(emu.peek_cpu(0x2001), emu.peek_cpu(0x2009));
}

#[test]
fn test_rom_write_protection() {
    // Attempt STA $8000 (PRG-ROM) - should not modify PRG-ROM byte
    let code = [
        0xa9, 0xff, 0x8d, 0x00, 0x80, // LDA #$FF, STA $8000
        0xea, // NOP
    ];
    let rom = make_nrom_rom(&code);
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();

    let orig_byte = emu.peek_cpu(0x8000); // 0xA9
    assert_eq!(orig_byte, 0xa9);

    for _ in 0..2 {
        emu.trace_instruction().unwrap();
    }

    assert_eq!(emu.peek_cpu(0x8000), 0xa9); // ROM byte remains untouched
}

#[test]
fn test_side_effect_free_peeks() {
    let code = [
        0xea, // NOP
    ];
    let rom = make_nrom_rom(&code);
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();

    // Peeking registers, controller ports, or memory repeatedly should not alter state
    let state_before = emu.get_cpu_state();
    for _ in 0..10 {
        let _ = emu.peek_cpu(0x2002); // PPUSTATUS
        let _ = emu.peek_cpu(0x4016); // Controller 1
        let _ = emu.peek_cpu(0x4017); // Controller 2
        let _ = emu.peek_cpu(0x0000);
        let _ = emu.peek_cpu(0x8000);
    }
    let state_after = emu.get_cpu_state();

    assert_eq!(state_before.pc, state_after.pc);
    assert_eq!(state_before.cycles, state_after.cycles);
    assert_eq!(state_before.acc, state_after.acc);
    assert_eq!(state_before.status, state_after.status);
}

use swiss_emulator::cartridge::Mapper;
use swiss_emulator::Emulator;

fn make_nrom_chr_ram_rom(code: &[u8]) -> Vec<u8> {
    let mut rom = vec![0; 16 + 16384];
    rom[..8].copy_from_slice(&[b'N', b'E', b'S', 0x1a, 1, 0, 0, 0]);
    rom[16..16 + code.len()].copy_from_slice(code);
    rom[16 + 0x3ffc..16 + 0x3ffe].copy_from_slice(&0x8000u16.to_le_bytes());
    rom
}

fn make_uxrom_rom(bank0_code: &[u8], bank1_code: &[u8]) -> Vec<u8> {
    let mut rom = vec![0; 16 + 32768];
    // Mapper 2 = 0x20 in header byte 6 (low nibble 2 << 4 = 0x20)
    rom[..8].copy_from_slice(&[b'N', b'E', b'S', 0x1a, 2, 0, 0x20, 0]);
    rom[16..16 + bank0_code.len()].copy_from_slice(bank0_code);
    let bank1_offset = 16 + 16384;
    rom[bank1_offset..bank1_offset + bank1_code.len()].copy_from_slice(bank1_code);
    // Fixed upper bank vector at $FFFC -> $C000
    rom[16 + 0x7ffc..16 + 0x7ffe].copy_from_slice(&0xc000u16.to_le_bytes());
    rom
}

#[test]
fn test_mapper_trait_prg_and_chr_access() {
    let code = [0xea]; // NOP
    let rom = make_nrom_chr_ram_rom(&code);
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();

    // PRG ROM read via Mapper trait
    assert_eq!(emu.read_prg(0x8000), 0xea);

    // PRG RAM write & read via Mapper trait
    emu.write_prg(0x6000, 0x42);
    assert_eq!(emu.read_prg(0x6000), 0x42);

    // CHR RAM read/write via Mapper trait
    emu.write_chr(0x0000, 0x99);
    assert_eq!(emu.read_chr(0x0000), 0x99);

    // Step IRQ returns false when no IRQ pending
    assert!(!emu.step_irq());
}

#[test]
fn test_uxrom_mapper_2_bank_switching() {
    let bank0 = [0xa9, 0x01, 0x8d, 0x00, 0x80]; // LDA #1, STA $8000 (switch to bank 1)
    let bank1 = [0xea]; // NOP in fixed bank / bank 1
    let rom = make_uxrom_rom(&bank0, &bank1);

    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();

    // Before bank switch, $8000 reads bank 0
    assert_eq!(emu.read_prg(0x8000), 0xa9);
    // $C000 reads fixed last bank (bank 1)
    assert_eq!(emu.read_prg(0xc000), 0xea);

    // Write bank number 1 to PRG ROM area ($8000)
    emu.write_prg(0x8000, 1);

    // Now switchable bank $8000 reads bank 1
    assert_eq!(emu.read_prg(0x8000), 0xea);
}

#[test]
fn mapper_irq_query_reports_actual_ppu_mmc3_delivery_without_double_clocking() {
    let mut rom = vec![0; 16 + 32768 + 8192];
    rom[..8].copy_from_slice(&[b'N', b'E', b'S', 0x1a, 2, 1, 0x40, 0]);
    // Execute in MMC3's fixed last 8 KiB. CPU writes configure a zero-latch IRQ;
    // actual PPU rendering later supplies the A12 transitions that assert it.
    let code = [
        0xa9, 0, 0x8d, 0, 0xc0, 0x8d, 1, 0xc0, 0x8d, 1, 0xe0, 0x78, 0x4c, 0x0c, 0xe0,
    ];
    rom[16 + 0x6000..16 + 0x6000 + code.len()].copy_from_slice(&code);
    rom[16 + 0x7ffc..16 + 0x7ffe].copy_from_slice(&0xe000u16.to_le_bytes());
    let mut emulator = Emulator::new();
    emulator.load_rom(&rom).unwrap();
    for _ in 0..2 {
        emulator.step().unwrap();
    }
    assert!(
        !emulator.step_irq(),
        "inactive rendering must not invent a mapper IRQ"
    );
    // Set background pattern table high and enable rendering after warmup.
    emulator.write_prg(0x2000, 0x10);
    emulator.write_prg(0x2001, 0x18);
    for _ in 0..4 {
        emulator.step().unwrap();
        if emulator.step_irq() {
            break;
        }
    }
    assert!(
        emulator.step_irq(),
        "PPU pattern fetches must route into MMC3's IRQ counter"
    );
    let cycles = emulator.get_cpu_state().cycles;
    assert!(emulator.step_irq());
    assert!(emulator.step_irq());
    assert_eq!(
        emulator.get_cpu_state().cycles,
        cycles,
        "compatibility query must not clock twice"
    );
    emulator.write_prg(0xe000, 0);
    assert!(
        !emulator.step_irq(),
        "mapper acknowledgement must clear the queried level"
    );
}

#[test]
fn mmc3_four_screen_ppudata_preserves_tables_and_3000_mirrors() {
    let mut rom = vec![0; 16 + 32768 + 8192];
    rom[..8].copy_from_slice(&[b'N', b'E', b'S', 0x1a, 2, 1, 0x48, 0]);
    // RAM flag gates each transaction. CPU reads PPUDATA twice to discard its
    // pipeline buffer, records the requested byte, then writes $9B to address+2.
    let code = [
        0xa9, 0, 0x85, 0, 0xa5, 0, 0xf0, 0xfc, 0xad, 7, 0x20, 0xad, 7, 0x20, 0x8d, 0, 2, 0xa9,
        0x9b, 0x8d, 7, 0x20, 0xa9, 0, 0x85, 0, 0x4c, 4, 0xe0,
    ];
    rom[16 + 0x6000..16 + 0x6000 + code.len()].copy_from_slice(&code);
    rom[16 + 0x7ffc..16 + 0x7ffe].copy_from_slice(&0xe000u16.to_le_bytes());
    let mut emulator = Emulator::new();
    emulator.load_rom(&rom).unwrap();
    for _ in 0..2 {
        emulator.step().unwrap();
    }
    for (address, value) in [
        (0x2000, 0x11),
        (0x2400, 0x22),
        (0x2800, 0x33),
        (0x2c00, 0x44),
    ] {
        emulator.write_chr(address, value);
    }
    fn transfer(emulator: &mut Emulator, address: u16) -> u8 {
        emulator.write_prg(0x2006, (address >> 8) as u8);
        emulator.write_prg(0x2006, address as u8);
        emulator.write_prg(0, 1);
        for _ in 0..32 {
            emulator.trace_instruction().unwrap();
            if emulator.peek_cpu(0) == 0 {
                return emulator.peek_cpu(0x200);
            }
        }
        panic!("PPUDATA transfer did not complete");
    }
    for (table, value) in [0x11, 0x22, 0x33, 0x44].into_iter().enumerate() {
        let base = 0x2000 + table as u16 * 0x400;
        assert_eq!(transfer(&mut emulator, base + 0x1000), value);
        assert_eq!(
            transfer(&mut emulator, base + 2),
            0x9b,
            "CPU mirrored write for table {table}"
        );
        assert_eq!(
            transfer(&mut emulator, base),
            value,
            "four independent nametables"
        );
    }
}

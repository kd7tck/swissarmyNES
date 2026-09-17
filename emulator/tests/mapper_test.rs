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

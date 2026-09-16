use swiss_emulator::cartridge::Mapper;
use swiss_emulator::Emulator;

fn make_nrom_chr_ram_rom(code: &[u8]) -> Vec<u8> {
    let mut rom = vec![0; 16 + 16384];
    rom[..8].copy_from_slice(&[b'N', b'E', b'S', 0x1a, 1, 0, 0, 0]);
    rom[16..16 + code.len()].copy_from_slice(code);
    rom[16 + 0x3ffc..16 + 0x3ffe].copy_from_slice(&0x8000u16.to_le_bytes());
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

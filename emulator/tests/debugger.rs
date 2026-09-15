use swiss_emulator::Emulator;

fn nrom() -> Vec<u8> {
    let mut rom = vec![0; 16 + 16384 + 8192];
    rom[..8].copy_from_slice(&[b'N', b'E', b'S', 0x1a, 1, 1, 0, 0]);
    rom[16..23].copy_from_slice(&[0xa9, 42, 0x85, 0, 0x4c, 0, 0x80]);
    rom[16 + 0x3ffc..16 + 0x3ffe].copy_from_slice(&0x8000u16.to_le_bytes());
    rom
}

#[test]
fn initial_breakpoint_stops_before_execution_and_continue_rehits() {
    let mut emu = Emulator::new();
    emu.load_rom(&nrom()).unwrap();
    emu.add_breakpoint(0, 0x8000);
    let cycles = emu.get_cpu_state().cycles;
    assert!(emu.step().unwrap());
    assert_eq!(emu.get_cpu_state().cycles, cycles);
    assert!(emu.step().unwrap());
    assert_eq!(emu.get_cpu_state().pc, 0x8000);
    assert!(emu.get_cpu_state().cycles > cycles);
    assert_eq!(emu.peek_cpu(0), 42);
    assert_eq!(emu.peek_cpu(0x800), 42);
    assert_eq!(emu.prg_offset(0x8000), 0);
    assert_eq!(emu.prg_offset(0xc000), 0);
    assert_eq!(emu.prg_offset(0), -1);
    let cycles = emu.get_cpu_state().cycles;
    assert!(emu.load_rom(b"invalid ROM").is_err());
    assert_eq!(emu.get_cpu_state().cycles, cycles);
    assert_eq!(emu.peek_cpu(0), 42);
    emu.reset();
    let cycles = emu.get_cpu_state().cycles;
    assert!(emu.step().unwrap());
    assert_eq!(emu.get_cpu_state().cycles, cycles);
}

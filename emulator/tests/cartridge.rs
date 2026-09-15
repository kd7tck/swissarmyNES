use swiss_emulator::cartridge::CartridgeInfo;

fn image(header: [u8; 16], payload: usize) -> Vec<u8> {
    let mut rom = header.to_vec();
    rom.resize(16 + payload, 0);
    rom
}
fn header() -> [u8; 16] {
    [b'N', b'E', b'S', 0x1a, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
}

#[test]
fn legacy_defaults_and_trainer_length() {
    let mut h = header();
    h[5] = 0;
    h[6] = 7;
    let parsed = CartridgeInfo::parse(&image(h, 16384 + 512)).unwrap();
    assert_eq!(parsed.prg_bytes, 16384);
    assert_eq!(parsed.chr_ram_bytes, 8192);
    assert_eq!(parsed.prg_nvram_bytes, 8192);
    assert_eq!(parsed.prg_ram_bytes, 0);
    assert!(parsed.trainer && parsed.vertical_mirroring);
    assert!(CartridgeInfo::parse(&image(h, 16384 + 511)).is_err());
}

#[test]
fn nes2_metadata_and_exponent_sizes() {
    let mut h = header();
    h[4] = 14 << 2;
    h[5] = 13 << 2;
    h[6] = 0x10;
    h[7] = 0x28;
    h[8] = 0x53;
    h[9] = 0xff;
    h[10] = 0x76;
    h[11] = 0x45;
    let parsed = CartridgeInfo::parse(&image(h, 16384 + 8192)).unwrap();
    assert_eq!(parsed.mapper, 0x321);
    assert_eq!(parsed.submapper, 5);
    assert_eq!((parsed.prg_bytes, parsed.chr_bytes), (16384, 8192));
    assert_eq!((parsed.prg_ram_bytes, parsed.prg_nvram_bytes), (4096, 8192));
    assert_eq!((parsed.chr_ram_bytes, parsed.chr_nvram_bytes), (2048, 1024));
    h[10] = 0;
    h[11] = 0;
    let parsed = CartridgeInfo::parse(&image(h, 16384 + 8192)).unwrap();
    assert_eq!(parsed.chr_ram_bytes, 0);
    assert_eq!(parsed.prg_ram_bytes, 0);
}

#[test]
fn malformed_and_oversized_headers_fail_before_allocation() {
    for len in 0..16 {
        assert!(CartridgeInfo::parse(&vec![0; len]).is_err());
    }
    let mut h = header();
    h[7] = 8;
    h[9] = 15;
    h[4] = 255;
    assert!(
        CartridgeInfo::parse(&h).unwrap_err().contains("size")
            || CartridgeInfo::parse(&h).unwrap_err().contains("limit")
    );
    h = header();
    h[4] = 0;
    assert!(CartridgeInfo::parse(&h).unwrap_err().contains("no PRG"));
}

#[test]
fn exponent_image_and_trainer_execute_via_production_loader() {
    let mut h = header();
    h[4] = 14 << 2;
    h[5] = 13 << 2;
    h[7] = 8;
    h[9] = 0xff;
    h[10] = 7;
    h[6] = 4;
    let mut rom = image(h, 512 + 16384 + 8192);
    rom[16..528].fill(0x5a);
    let base = 528;
    rom[base..base + 8].copy_from_slice(&[0xad, 0, 0x70, 0x85, 0, 0x4c, 5, 0x80]);
    rom[base + 0x3ffc..base + 0x3ffe].copy_from_slice(&0x8000u16.to_le_bytes());
    let mut emulator = swiss_emulator::Emulator::new();
    emulator.load_rom(&rom).unwrap();
    emulator.step().unwrap();
    assert_eq!(emulator.peek_cpu(0), 0x5a);
    assert_eq!(emulator.peek_cpu(0x71ff), 0x5a);
}

#[test]
fn loaded_region_controls_exported_frame_cadence() {
    for (timing, expected) in [(0, 60.0988), (1, 50.007), (3, 50.007)] {
        let mut rom = vec![0; 16 + 16384 + 8192];
        rom[..8].copy_from_slice(&[b'N', b'E', b'S', 0x1a, 1, 1, 0, 8]);
        rom[12] = timing;
        rom[16 + 0x3ffc..16 + 0x3ffe].copy_from_slice(&0x8000u16.to_le_bytes());
        let mut emulator = swiss_emulator::Emulator::new();
        emulator.load_rom(&rom).unwrap();
        assert!(
            (emulator.frame_rate() - expected).abs() < 0.002,
            "timing {timing}: {}",
            emulator.frame_rate()
        );
    }
}

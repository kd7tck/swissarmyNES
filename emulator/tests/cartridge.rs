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
fn nes2_ram_and_nvram_shift_decoding() {
    let mut h = header();
    h[7] = 8; // NES 2.0
    h[10] = 0x70; // NVRAM shift 7 = 64 << 7 = 8192 bytes, RAM shift 0 = 0
    h[11] = 0x05; // CHR NVRAM shift 0 = 0, CHR RAM shift 5 = 64 << 5 = 2048
    let parsed = CartridgeInfo::parse(&image(h, 16384 + 8192)).unwrap();
    assert_eq!(parsed.prg_ram_bytes, 0);
    assert_eq!(parsed.prg_nvram_bytes, 8192);
    assert_eq!(parsed.chr_ram_bytes, 2048);
    assert_eq!(parsed.chr_nvram_bytes, 0);
}

#[test]
fn four_screen_mirroring_and_battery_flags() {
    let mut h = header();
    h[6] = 0x0a; // Bit 1 = battery (2), Bit 3 = four_screen (8) -> 10 (0x0A)
    let parsed = CartridgeInfo::parse(&image(h, 16384 + 8192)).unwrap();
    assert!(parsed.battery);
    assert!(parsed.four_screen);

    h[6] = 0x00;
    let parsed = CartridgeInfo::parse(&image(h, 16384 + 8192)).unwrap();
    assert!(!parsed.battery);
    assert!(!parsed.four_screen);
}

#[test]
fn nes2_submapper_and_12bit_mapper_decoding() {
    let mut h = header();
    h[6] = 0x30; // mapper low nibble = 3
    h[7] = 0x28; // nes2 flag (8) | mapper mid nibble = 2 -> 0x23
    h[8] = 0x51; // submapper = 5, mapper high nibble = 1 -> 0x123
    let parsed = CartridgeInfo::parse(&image(h, 16384 + 8192)).unwrap();
    assert_eq!(parsed.mapper, 0x123);
    assert_eq!(parsed.submapper, 5);
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
    // Invalid magic signature
    let mut bad_magic = header();
    bad_magic[0] = b'B';
    assert_eq!(
        CartridgeInfo::parse(&image(bad_magic, 16384)).unwrap_err(),
        "Invalid iNES signature"
    );

    // Corrupted / unsupported iNES header variants (header[7] & 12 in {4, 12})
    let mut bad_variant = header();
    bad_variant[7] = 4;
    assert_eq!(
        CartridgeInfo::parse(&image(bad_variant, 16384)).unwrap_err(),
        "Unsupported or corrupted iNES header variant"
    );

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
    // Reset is a fresh boot of the loaded cartridge, including its trainer.
    // A failed replacement must retain both the game and its boot data.
    assert!(emulator.load_rom(b"invalid").is_err());
    emulator.reset();
    for address in 0x7000..=0x71ff {
        assert_eq!(
            emulator.peek_cpu(address),
            0x5a,
            "trainer byte {address:04X}"
        );
    }
    emulator.step().unwrap();
    assert_eq!(emulator.peek_cpu(0), 0x5a);
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

#[test]
fn nes2_nrom_ram_absence_sizes_and_nvram_execute_as_declared() {
    for (declaration, size, read_address) in [
        (0, 0, 0x6000u16),
        (1, 128, 0x6080),
        (5, 2048, 0x6800),
        (0x70, 8192, 0x6000),
        (0x66, 8192, 0x6000),
    ] {
        let mut h = header();
        h[7] = 8;
        h[10] = declaration;
        if declaration & 0xf0 != 0 {
            h[6] |= 2;
        }
        let mut rom = image(h, 16384 + 8192);
        // Store to RAM base, read through its declared mirror, and record in CPU RAM.
        let [low, high] = read_address.to_le_bytes();
        rom[16..28].copy_from_slice(&[
            0xa9, 0x5a, 0x8d, 0, 0x60, 0xad, low, high, 0x85, 0, 0xea, 0xea,
        ]);
        rom[16 + 0x3ffc..16 + 0x3ffe].copy_from_slice(&0x8000u16.to_le_bytes());
        let mut emulator = swiss_emulator::Emulator::new();
        emulator.load_rom(&rom).unwrap();
        assert_eq!(
            emulator.prg_ram_len(),
            size,
            "declaration {declaration:02X}"
        );
        for _ in 0..4 {
            emulator.trace_instruction().unwrap();
        }
        // On an absent RAM read, the last bus value is the address operand's high byte.
        assert_eq!(emulator.peek_cpu(0), if size == 0 { high } else { 0x5a });
    }
}

#[test]
fn nes2_nrom_rejects_unmappable_ram_and_missing_chr_storage() {
    let mut h = header();
    h[7] = 8;
    h[10] = 8; // 16 KiB cannot be addressed by NROM's unbanked 8 KiB window.
    let mut emulator = swiss_emulator::Emulator::new();
    h[6] = 4;
    h[10] = 0;
    let mut absent_trainer = image(h, 512 + 16384 + 8192);
    absent_trainer[16..528].fill(0x5a);
    assert!(emulator
        .load_rom(&absent_trainer)
        .unwrap_err()
        .contains("trainer requires"));
    h[6] = 0;
    h[10] = 8;
    assert!(emulator
        .load_rom(&image(h, 16384 + 8192))
        .unwrap_err()
        .contains("at most 8 KiB"));
    h[10] = 0;
    h[5] = 0;
    assert!(emulator
        .load_rom(&image(h, 16384))
        .unwrap_err()
        .contains("explicit 8 KiB CHR RAM"));
}

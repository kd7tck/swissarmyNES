use swiss_emulator::{cartridge::Mapper, Emulator};

fn image(prg_ram: u8, chr_ram: u8) -> Vec<u8> {
    let mut rom = vec![0; 16 + 16384];
    rom[..8].copy_from_slice(&[b'N', b'E', b'S', 0x1a, 1, 0, 2, 8]);
    rom[10] = prg_ram;
    rom[11] = chr_ram;
    rom[16..19].copy_from_slice(&[0x4c, 0, 0x80]);
    rom[16 + 0x3ffc..16 + 0x3ffe].copy_from_slice(&0x8000u16.to_le_bytes());
    rom
}

#[test]
fn declared_battery_regions_survive_reset_and_round_trip_without_volatile_bytes() {
    let rom = image(0x66, 0x66); // Each region has 4 KiB volatile + 4 KiB nonvolatile.
    let mut emulator = Emulator::new();
    emulator.load_rom(&rom).unwrap();
    emulator.write_prg(0x6000, 0x11);
    emulator.write_prg(0x7000, 0x5a);
    emulator.write_chr(0, 0x22);
    emulator.write_chr(0x1000, 0xa5);
    let saved = emulator.save_ram_snapshot();
    assert_eq!(saved.len(), 8192);
    assert_eq!((saved[0], saved[4096]), (0x5a, 0xa5));

    emulator.reset();
    assert_eq!(emulator.read_prg(0x7000), 0x5a);
    assert_eq!(emulator.read_chr(0x1000), 0xa5);
    assert_eq!(emulator.save_ram_snapshot(), saved);

    let mut restored = Emulator::new();
    restored.load_rom(&rom).unwrap();
    restored.write_prg(0x6000, 0x77);
    restored.write_chr(0, 0x88);
    restored.restore_save_ram(&saved).unwrap();
    assert_eq!(
        restored.read_prg(0x6000),
        0x77,
        "restore must leave volatile RAM alone"
    );
    assert_eq!(
        restored.read_chr(0),
        0x88,
        "restore must leave volatile CHR alone"
    );
    assert_eq!(
        (restored.read_prg(0x7000), restored.read_chr(0x1000)),
        (0x5a, 0xa5)
    );
    assert!(restored.restore_save_ram(&saved[..8191]).is_err());
    assert_eq!(
        restored.save_ram_snapshot(),
        saved,
        "rejected restore must be transactional"
    );

    assert!(restored.load_rom(b"invalid").is_err());
    restored.reset();
    assert_eq!(
        restored.save_ram_snapshot(),
        saved,
        "failed load must retain battery ownership"
    );
    restored.load_rom(&image(7, 7)).unwrap();
    assert!(
        restored.save_ram_snapshot().is_empty(),
        "replacement must not inherit prior save data"
    );
}

#[test]
fn battery_only_and_legacy_battery_have_the_documented_payload_size() {
    let mut emulator = Emulator::new();
    assert!(emulator.restore_save_ram(&[]).is_err());
    assert!(emulator.save_ram_snapshot().is_empty());
    emulator.reset();
    emulator.load_rom(&image(0x70, 7)).unwrap();
    emulator.write_prg(0x6000, 0x42);
    emulator.reset();
    assert_eq!(emulator.read_prg(0x6000), 0x42);
    assert_eq!(emulator.save_ram_snapshot().len(), 8192);

    let mut legacy = image(0, 0);
    legacy[7] = 0;
    emulator.load_rom(&legacy).unwrap();
    emulator.write_prg(0x7fff, 0x99);
    emulator.reset();
    assert_eq!(emulator.read_prg(0x7fff), 0x99);
    assert_eq!(emulator.save_ram_snapshot().len(), 8192);
}

#[test]
fn mapper_defaults_that_disagree_with_nes2_are_rejected_transactionally() {
    let mut emulator = Emulator::new();
    emulator.load_rom(&image(0x70, 7)).unwrap();
    emulator.write_prg(0x6000, 0x42);
    let mut unsupported = image(0, 7);
    unsupported[6] = 0x10; // MMC1 backend would invent 32 KiB RAM despite explicit absence.
    assert!(emulator
        .load_rom(&unsupported)
        .unwrap_err()
        .contains("PRG RAM layout"));
    assert_eq!(emulator.read_prg(0x6000), 0x42);
}

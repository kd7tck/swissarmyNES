use std::collections::HashMap;
use swiss_emulator::Emulator;
use swissarmynes::compiler::world;
use swissarmynes::server::{api::compile_source, project::ProjectAssets};

fn assets(map_count: usize) -> ProjectAssets {
    let maps: Vec<_> = (0..map_count).map(|map| serde_json::json!({
        "name": format!("map{map}"),
        "data": (0..960).map(|index| ((index * 13 + map * 37 + 17) & 255) as u8).collect::<Vec<_>>(),
        "attrs": (0..64).map(|index| ((index * 7 + map * 29 + 3) & 255) as u8).collect::<Vec<_>>()
    })).collect();
    serde_json::from_value(serde_json::json!({
        "chr_bank": [], "palettes": [], "nametables": maps,
        "world": {"width": 3, "height": 2, "data": [0, 1, -1, 2, 3, -1]}
    }))
    .unwrap()
}

fn image(assets: &ProjectAssets, index: usize) -> Vec<u8> {
    assets.nametables[index]
        .data
        .iter()
        .chain(&assets.nametables[index].attrs)
        .copied()
        .collect()
}

#[test]
fn four_maps_attributes_backtracking_empty_cells_and_banked_return_execute() {
    let source = r#"
        DIM sample AS BYTE
        DIM visit AS BYTE
        DIM restored_bank AS BYTE
        DIM restored_ctrl AS BYTE
        DIM restored_mask AS BYTE
        DIM loaded AS BYTE
        DIM done AS BYTE
        DIM marker AS BYTE
        DIM nmi_ticks AS BYTE
        DIM selected_tile AS BYTE
        DIM empty_result AS BYTE
        DIM invalid_result AS BYTE
        DIM index AS WORD
        DIM xs(5) AS BYTE
        DIM ys(5) AS BYTE
        DIM dummy AS BYTE
        DIM collision_tile AS BYTE
        BANK 5
        SUB Main()
            xs(0) = 0
            xs(1) = 1
            xs(2) = 0
            xs(3) = 1
            xs(4) = 0
            ys(0) = 0
            ys(1) = 0
            ys(2) = 1
            ys(3) = 1
            ys(4) = 0
            POKE($E0, 19)
            POKE($E1, 27)
            PPU.Ctrl($83)
            PPU.Mask($1E)
            dummy = PEEK($2002)
            POKE($2005, 19)
            POKE($2005, 27)
            ASM
                CLI
            END ASM
            FOR visit = 0 TO 4
                loaded = World.Load(xs(visit), ys(visit))
                restored_bank = PEEK($07F0)
                restored_ctrl = PEEK($F8)
                restored_mask = PEEK($F9)
                collision_tile = Collision.Tile(255, 239)
                PPU.Ctrl(3)
                PPU.Mask(0)
                POKE($2006, $2C)
                POKE($2006, 0)
                dummy = PEEK($2007)
                FOR index = 0 TO 1023
                    sample = PEEK($2007)
                    marker = 1
                NEXT index
                PPU.Mask($1E)
                PPU.Ctrl($83)
                WAIT_VBLANK
            NEXT visit
            empty_result = World.Load(2, 0)
            invalid_result = World.Load(256, 0)
            selected_tile = World.Tile(255, 239)
            done = 1
        END SUB
        INTERRUPT NMI()
            nmi_ticks = nmi_ticks + 1
        END INTERRUPT
    "#;
    let assets = assets(4);
    let (rom, map) = compile_source(Some(source.into()), None, Some(assets.clone()))
        .expect("world production compile");
    // Directory/data are actual PRG bytes, each reference has a full 1024-byte image.
    let directory = &rom[16 + 16384..16 + 16384 + 34];
    assert_eq!(&directory[..4], &[world::DIRECTORY_VERSION, 3, 2, 5]);
    for (cell, map_index) in [Some(0), Some(1), None, Some(2), Some(3), None]
        .iter()
        .enumerate()
    {
        let record = &directory[4 + cell * 5..4 + cell * 5 + 5];
        if let Some(index) = map_index {
            assert_eq!(&record[3..], &[0, 4]);
            let address = u16::from_le_bytes([record[1], record[2]]);
            let offset = 16 + usize::from(record[0]) * 16384 + usize::from(address) - 0x8000;
            assert_eq!(&rom[offset..offset + 1024], image(&assets, *index));
        } else {
            assert_eq!(record, &[0xFF, 0, 0, 0, 0]);
        }
    }
    let line = source
        .lines()
        .position(|line| line.trim() == "marker = 1")
        .unwrap()
        + 1;
    let stop = map
        .entries
        .iter()
        .find(|entry| entry.line == line && entry.bank == 5)
        .expect("read marker source map");
    let probe_line = source
        .lines()
        .position(|line| line.trim() == "restored_bank = PEEK($07F0)")
        .unwrap()
        + 1;
    let probe = map
        .entries
        .iter()
        .find(|entry| entry.line == probe_line && entry.bank == 5)
        .expect("post-load source map");
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();
    emu.add_breakpoint(stop.bank, stop.cpu_start);
    emu.add_breakpoint(probe.bank, probe.cpu_start);
    let mut transitions = 0;
    for expected_map in [0, 1, 2, 3, 0] {
        for expected in image(&assets, expected_map) {
            let mut hit = false;
            for _ in 0..20 {
                if emu.step().unwrap() {
                    let state = emu.get_cpu_state();
                    if state.pc == probe.cpu_start {
                        transitions += 1;
                        assert_eq!(
                            emu.viewport_position(),
                            [275, 267],
                            "programmed nametable origin and fine scroll restored"
                        );
                        assert_eq!(
                            state.status & 4,
                            0,
                            "World.Load restores the caller's clear IRQ mask"
                        );
                    } else {
                        hit = true;
                        break;
                    }
                }
            }
            assert!(hit, "each actual PPU byte must reach the marker");
            let ram = emu.ram_snapshot();
            assert_eq!(
                ram[0x05C0],
                expected,
                "map{expected_map}, byte stream index{}",
                u16::from_le_bytes([ram[0x05CC], ram[0x05CD]])
            );
            assert_eq!(
                &ram[0x05C2..0x05C6],
                &[5, 0x83, 0x1E, 0xFF],
                "World.Load returns to bank5 with CTRL/MASK and success intact"
            );
            assert_eq!(&ram[0xE0..0xE2], &[19, 27], "fine scroll preserved");
            assert_eq!(
                ram[0x05D9], assets.nametables[expected_map].data[959],
                "collision and graphics select the same map"
            );
        }
    }
    assert_eq!(transitions, 5);
    emu.clear_breakpoints();
    for _ in 0..20 {
        emu.step().unwrap();
        if emu.ram_snapshot()[0x05C6] == 1 {
            break;
        }
    }
    let ram = emu.ram_snapshot();
    assert_eq!(ram[0x05C6], 1, "switchable Main reaches completion");
    assert_ne!(
        ram[0x05C8], 0,
        "NMI continues after successful/empty transitions"
    );
    assert_eq!(
        ram[0x05C9], assets.nametables[0].data[959],
        "selected-map tile lookup survives backtracking"
    );
    assert_eq!(
        &ram[0x05CA..0x05CC],
        &[0, 0],
        "empty and high-byte coordinates return false"
    );
    assert_eq!(
        ram[0x07F0], 0,
        "Main trampoline restores original caller bank"
    );
    assert_eq!(&ram[0xF8..0xFA], &[0x83, 0x1E]);
}

#[test]
fn legacy_single_map_and_world_tile_without_world_keep_first_map_behavior() {
    let mut assets = assets(4);
    assets.world = None;
    assets.nametables.truncate(1);
    let source = "DIM selected_tile AS BYTE\nSUB Main()\nselected_tile = World.Tile(0, 0)\nEND SUB";
    let (rom, _) = compile_source(Some(source.into()), None, Some(assets.clone())).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();
    for _ in 0..10 {
        emu.step().unwrap();
    }
    assert_eq!(emu.ram_snapshot()[0x05C0], assets.nametables[0].data[0]);
}

#[test]
fn maps_cross_asset_banks_and_duplicate_cells_share_storage() {
    let mut assets = assets(20);
    assets.world = Some(swissarmynes::server::project::WorldLayout {
        width: 21,
        height: 1,
        data: (0..20).chain([19]).collect(),
    });
    let source = "DIM loaded AS BYTE\nDIM selected_tile AS BYTE\nDIM collision_tile AS BYTE\nDIM returned_bank AS BYTE\nBANK 5\nSUB Main()\nloaded = World.Load(19, 0)\nselected_tile = World.Tile(255, 239)\ncollision_tile = Collision.Tile(255, 239)\nreturned_bank = PEEK($07F0)\nEND SUB";
    let (rom, _) = compile_source(Some(source.into()), None, Some(assets.clone())).unwrap();
    let directory = 16 + 16384;
    let record = &rom[directory + 4 + 19 * 5..directory + 4 + 20 * 5];
    assert_eq!(
        record[0], 2,
        "map19 uses a different bank from its directory"
    );
    assert_eq!(record, &rom[directory + 4 + 20 * 5..directory + 4 + 21 * 5]);
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();
    for _ in 0..10 {
        emu.step().unwrap();
    }
    let ram = emu.ram_snapshot();
    assert_eq!(
        &ram[0x05C0..0x05C4],
        &[
            0xFF,
            assets.nametables[19].data[959],
            assets.nametables[19].data[959],
            5
        ]
    );
    assert_eq!(ram[0x07FA], 2);
    assert_eq!(ram[0x07F0], 0);
}

#[test]
fn signed_negative_coordinates_are_rejected_without_wrapping() {
    let source = "DIM coordinate AS INT = -1\nDIM loaded AS BYTE\nDIM selected_tile AS BYTE\nSUB Main()\nloaded = World.Load(coordinate, 0)\nselected_tile = World.Tile(coordinate, 0)\nEND SUB";
    let (rom, _) = compile_source(Some(source.into()), None, Some(assets(4))).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();
    for _ in 0..10 {
        emu.step().unwrap();
    }
    assert_eq!(&emu.ram_snapshot()[0x05C1..0x05C3], &[0, 0]);
    assert_eq!(emu.ram_snapshot()[0x07FA], 0);
}

#[test]
fn production_world_capacity_and_argument_errors_are_explicit() {
    let mut large = assets(100);
    large.world = Some(swissarmynes::server::project::WorldLayout {
        width: 100,
        height: 1,
        data: (0..100).collect(),
    });
    let error = compile_source(Some("SUB Main()\nEND SUB".into()), None, Some(large)).unwrap_err();
    assert!(error.contains("World map data exceeds"), "{error}");
    let mut huge = assets(4);
    let layout = huge.world.as_mut().unwrap();
    layout.width = 255;
    layout.height = 255;
    layout.data = vec![-1; 255 * 255];
    let error = compile_source(Some("SUB Main()\nEND SUB".into()), None, Some(huge)).unwrap_err();
    assert!(error.contains("World directory exceeds"), "{error}");
    for call in [
        "World.Load(0)",
        "World.Tile(\"bad\", 0)",
        "World.Missing(0, 0)",
    ] {
        assert!(
            compile_source(Some(format!("SUB Main()\n{call}\nEND SUB")), None, None).is_err(),
            "{call}"
        );
    }
}

#[test]
fn public_world_packer_rejects_invalid_inputs_transactionally() {
    for (width, height, data) in [
        (256, 1, vec![0; 256]),
        (0, 1, vec![]),
        (2, 2, vec![0]),
        (1, 1, vec![-2]),
        (1, 1, vec![4]),
    ] {
        let mut assets = assets(4);
        let layout = assets.world.as_mut().unwrap();
        layout.width = width;
        layout.height = height;
        layout.data = data;
        let mut banks = HashMap::from([(0, vec![".ORG $8000".into()])]);
        let before = banks.clone();
        assert!(world::append_assets(&mut banks, Some(&assets)).is_err());
        assert_eq!(banks, before);
    }
}

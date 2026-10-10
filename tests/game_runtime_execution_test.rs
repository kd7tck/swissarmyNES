use swiss_emulator::Emulator;
use swissarmynes::server::api::compile_source;
use swissarmynes::server::project::{Nametable, ProjectAssets};

fn execute(source: &str) -> Emulator {
    execute_with_assets(source, None)
}

fn execute_with_assets(source: &str, assets: Option<ProjectAssets>) -> Emulator {
    let (rom, _) = compile_source(Some(source.to_string()), None, assets)
        .expect("Production compiler must build game fixture");
    let mut emu = Emulator::new();
    emu.load_rom(&rom).expect("Fixture must load");
    for _ in 0..10 {
        emu.step().expect("Game fixture must run and return safely");
    }
    emu
}

#[test]
fn scroll_column_preserves_pointer_coordinate_return_and_pending_upload() {
    let emu = execute(
        r#"
        DIM col(30) AS BYTE
        DIM i AS BYTE
        DIM returned AS BYTE
        SUB Main()
            PPU.Ctrl(0)
            FOR i = 0 TO 29
                col(i) = i + 40
            NEXT i
            POKE($03C0, $A5)
            Scroll.LoadColumn(264, col)
            Scroll.LoadColumn(0, col)
            returned = 1
        END SUB
        "#,
    );
    let ram = emu.ram_snapshot();
    assert_eq!(ram[0x05df], 1, "Both calls must return to Main");
    assert_eq!(&ram[0x0380..0x0384], &[1, 0, 0x24, 1]);
    assert_eq!(&ram[0x0384..0x03a2], &(40..70).collect::<Vec<_>>());
    assert_eq!(ram[0x03c0], 0xa5, "Upload must stay inside its buffer");
}

#[test]
fn scroll_row_preserves_pointer_and_maps_the_vertical_gap() {
    let emu = execute(
        r#"
        DIM row(32) AS BYTE
        DIM i AS BYTE
        DIM returned AS BYTE
        SUB Main()
            PPU.Ctrl(0)
            FOR i = 0 TO 31
                row(i) = i + 80
            NEXT i
            POKE($03C0, $A5)
            Scroll.LoadRow(240, row)
            returned = 1
        END SUB
        "#,
    );
    let ram = emu.ram_snapshot();
    assert_eq!(ram[0x05e1], 1, "Upload call must return to Main");
    assert_eq!(&ram[0x0380..0x0384], &[1, 1, 0x28, 0]);
    assert_eq!(&ram[0x0384..0x03a4], &(80..112).collect::<Vec<_>>());
    assert_eq!(ram[0x03c0], 0xa5, "Upload must stay inside its buffer");
}

#[test]
fn scroll_upload_reaches_mapped_ppu_memory_on_nmi() {
    let emu = execute(
        r#"
        DIM row(32) AS BYTE
        DIM i AS BYTE
        DIM received(32) AS BYTE
        DIM dummy AS BYTE
        SUB Main()
            FOR i = 0 TO 31
                row(i) = i + 80
            NEXT i
            Scroll.LoadRow(8, row)
            PPU.Ctrl($80)
            WAIT_VBLANK
            WAIT_VBLANK
            PPU.Ctrl(0)
            POKE($2006, $20)
            POKE($2006, $20)
            dummy = PEEK($2007)
            FOR i = 0 TO 31
                received(i) = PEEK($2007)
            NEXT i
        END SUB
        "#,
    );
    assert_eq!(emu.ram_snapshot()[0x0380], 0, "NMI consumes the upload");
    assert_eq!(
        &emu.ram_snapshot()[0x05e1..0x0601],
        &(80..112).collect::<Vec<_>>()
    );
}

#[test]
fn text_print_uses_horizontal_increment_and_clips_before_attributes() {
    let emu = execute(
        r#"
        DIM received(11) AS BYTE
        DIM i AS BYTE
        DIM dummy AS BYTE
        DIM edge AS BYTE
        DIM attr AS BYTE
        SUB Main()
            PPU.Ctrl($04)
            Text.SetOffset(1)
            WAIT_VBLANK
            Text.Print(10, 10, "Hello World")
            Text.Print(31, 29, "AB")
            PPU.Ctrl(0)
            POKE($2006, $21)
            POKE($2006, $4A)
            dummy = PEEK($2007)
            FOR i = 0 TO 10
                received(i) = PEEK($2007)
            NEXT i
            POKE($2006, $23)
            POKE($2006, $BF)
            dummy = PEEK($2007)
            edge = PEEK($2007)
            attr = PEEK($2007)
        END SUB
        "#,
    );
    let ram = emu.ram_snapshot();
    assert_eq!(&ram[0x05c0..0x05cb], &b"Hello World".map(|ch| ch + 1));
    assert_eq!(ram[0x05cd], b'A' + 1);
    assert_eq!(ram[0x05ce], 0, "Clipping must preserve attribute bytes");
}

#[test]
fn metasprite_draw_keeps_all_64_tiles_and_attributes_without_pointer_wrap() {
    let mut source = String::from("METASPRITE full\n");
    for tile in 0..64 {
        source.push_str(&format!("TILE 0, 0, {tile}, {}\n", tile % 4));
    }
    source.push_str(
        "END METASPRITE\nSUB Main()\nSprite.Clear()\nSprite.Draw(40, 50, full)\nSprite.Draw(90, 100, full)\nEND SUB\n",
    );
    let emu = execute(&source);
    let ram = emu.ram_snapshot();
    for tile in 0..64 {
        assert_eq!(
            &ram[0x0200 + tile * 4..0x0204 + tile * 4],
            &[50, tile as u8, tile as u8 % 4, 40],
            "Sprite {tile} must retain authored bytes; excess draw must not overwrite OAM"
        );
    }
    assert_eq!(ram[0x1b], 64);
}

#[test]
fn animation_duration_loop_stop_and_restart_execute_exactly() {
    let emu = execute(
        r#"
        METASPRITE first
            TILE 0, 0, 10, 0
        END METASPRITE
        METASPRITE second
            TILE 0, 0, 20, 0
        END METASPRITE
        METASPRITE third
            TILE 0, 0, 30, 0
        END METASPRITE
        ANIMATION repeating
            FRAME first, 2
            FRAME second, 1
            FRAME third, 3
            LOOP
        END ANIMATION
        ANIMATION stopping
            FRAME first, 1
            FRAME second, 1
            FRAME third, 1
        END ANIMATION
        DIM state AS AnimState
        DIM observed(8) AS BYTE
        SUB Main()
            Animation.Play(state, repeating)
            Animation.Update(state)
            observed(0) = state.frame_index
            Animation.Update(state)
            observed(1) = state.frame_index
            Animation.Update(state)
            observed(2) = state.frame_index
            Animation.Update(state)
            Animation.Update(state)
            observed(3) = state.frame_index
            Animation.Update(state)
            observed(4) = state.frame_index
            Animation.Play(state, stopping)
            Animation.Update(state)
            Animation.Update(state)
            Animation.Update(state)
            Animation.Update(state)
            observed(5) = state.frame_index
            observed(6) = state.finished
            Animation.Play(state, repeating)
            observed(7) = state.finished
            Sprite.Clear()
            Animation.Draw(40, 50, state)
        END SUB
        "#,
    );
    let ram = emu.ram_snapshot();
    assert_eq!(&ram[0x05c5..0x05cd], &[0, 1, 2, 2, 0, 2, 255, 0]);
    assert_eq!(&ram[0x0200..0x0204], &[50, 10, 0, 40]);
}

#[test]
fn animation_rejects_empty_zero_duration_and_unencodable_frame_counts() {
    for animation in [
        "ANIMATION invalid\nEND ANIMATION\n".to_string(),
        "ANIMATION invalid\nFRAME sprite, 0\nEND ANIMATION\n".to_string(),
        format!(
            "ANIMATION invalid\n{}END ANIMATION\n",
            "FRAME sprite, 1\n".repeat(256)
        ),
    ] {
        let source = format!(
            "METASPRITE sprite\nTILE 0, 0, 1, 0\nEND METASPRITE\n{animation}SUB Main()\nEND SUB\n"
        );
        let error = compile_source(Some(source), None, None).unwrap_err();
        assert!(
            error.contains("Animation 'invalid'"),
            "Unexpected diagnostic: {error}"
        );
    }
}

#[test]
fn animation_frame_table_crosses_a_page_without_truncating_the_offset() {
    let mut source = String::from("METASPRITE first\nTILE 0, 0, 10, 0\nEND METASPRITE\nMETASPRITE last\nTILE 0, 0, 90, 0\nEND METASPRITE\nANIMATION long_anim\n");
    source.push_str(&"FRAME first, 1\n".repeat(84));
    source.push_str("FRAME last, 3\nEND ANIMATION\nDIM state AS AnimState\nDIM i AS BYTE\nSUB Main()\nAnimation.Play(state, long_anim)\nFOR i = 1 TO 84\nAnimation.Update(state)\nNEXT i\nSprite.Clear()\nAnimation.Draw(40, 50, state)\nEND SUB\n");
    let emu = execute(&source);
    let ram = emu.ram_snapshot();
    assert_eq!(ram[0x05c2], 84);
    assert_eq!(ram[0x05c3], 3);
    assert_eq!(&ram[0x0200..0x0204], &[50, 90, 0, 40]);
}

#[test]
fn scroll_world_coordinates_cross_repeated_seams_and_reverse_direction() {
    for (x, y, expected_x, expected_y, nametable_bits) in [
        (264, 240, 8, 0, 3),
        (512, 480, 0, 0, 0),
        (255, 239, 255, 239, 0),
        (520, 488, 8, 8, 0),
        (256, 248, 0, 8, 3),
    ] {
        let source = format!(
            "DIM row(32) AS BYTE\nSUB Main()\nPPU.Ctrl($10)\nScroll.Set(512, 480)\nScroll.Set({x}, {y})\nScroll.LoadRow({y}, row)\nEND SUB\n"
        );
        let emu = execute(&source);
        let ram = emu.ram_snapshot();
        assert_eq!(ram[0xe0], expected_x);
        assert_eq!(ram[0xe1], expected_y);
        assert_eq!(ram[0xf8] & 3, nametable_bits);
        let row_address =
            0x2000 + u16::from(nametable_bits & 2) * 0x400 + u16::from(expected_y / 8) * 32;
        assert_eq!(ram[0x0382], (row_address >> 8) as u8);
        assert_eq!(ram[0x0383], row_address as u8);
    }
}

#[test]
fn scroll_upload_rejects_short_arrays_and_nonbyte_tiles() {
    for (declaration, call) in [
        ("DIM tiles(29) AS BYTE", "Scroll.LoadColumn(0, tiles)"),
        ("DIM tiles(31) AS BYTE", "Scroll.LoadRow(0, tiles)"),
        ("DIM tiles(32) AS WORD", "Scroll.LoadRow(0, tiles)"),
    ] {
        let source = format!("{declaration}\nSUB Main()\n{call}\nEND SUB\n");
        let error = compile_source(Some(source), None, None).unwrap_err();
        assert!(
            error.contains("expects a BYTE array with at least"),
            "Unexpected diagnostic: {error}"
        );
    }
}

#[test]
fn scroll_row_and_column_attributes_upload_on_a_second_nmi_without_overflow() {
    for (command, count, coordinate, attribute_address, stride) in [
        ("LoadRow", 32, 272, 0x2bc8u16, 1),
        ("LoadColumn", 30, 296, 0x27c1u16, 8),
    ] {
        let source = format!(
            r#"
            DIM tiles({count}) AS BYTE
            DIM attrs(8) AS BYTE
            DIM i AS BYTE
            DIM received(8) AS BYTE
            DIM dummy AS BYTE
            SUB Main()
                PPU.Ctrl(0)
                FOR i = 0 TO {last}
                    tiles(i) = i + 60
                NEXT i
                FOR i = 0 TO 7
                    attrs(i) = i + 160
                NEXT i
                POKE($03BF, $A5)
                POKE($03C0, $5A)
                Scroll.{command}({coordinate}, tiles, attrs)
                Scroll.{command}(0, tiles, attrs)
                PPU.Ctrl($80)
                WAIT_VBLANK
                WAIT_VBLANK
                PPU.Ctrl(0)
                FOR i = 0 TO 7
                    POKE($2006, {high})
                    POKE($2006, {low} + i * {stride})
                    dummy = PEEK($2007)
                    received(i) = PEEK($2007)
                NEXT i
            END SUB
            "#,
            last = count - 1,
            high = attribute_address >> 8,
            low = attribute_address & 0xff,
        );
        let emu = execute(&source);
        let ram = emu.ram_snapshot();
        let received = 0x05c0 + count + 8 + 1;
        assert_eq!(
            &ram[received..received + 8],
            &(160..168).collect::<Vec<_>>(),
            "{command} mapped attributes"
        );
        assert_eq!(ram[0x0380], 0, "Both upload phases must complete");
        assert_eq!(&ram[0x03a6..0x03ae], &(160..168).collect::<Vec<_>>());
        assert_eq!(ram[0x03bf], 0xa5);
        assert_eq!(ram[0x03c0], 0x5a, "Upload must never overwrite string heap");
    }
}

#[test]
fn scroll_optional_attributes_require_eight_byte_entries() {
    for declaration in ["DIM attrs(7) AS BYTE", "DIM attrs(8) AS WORD"] {
        let source = format!("DIM tiles(32) AS BYTE\n{declaration}\nSUB Main()\nScroll.LoadRow(0, tiles, attrs)\nEND SUB\n");
        let error = compile_source(Some(source), None, None).unwrap_err();
        assert!(
            error.contains("attributes require a BYTE array with at least 8 entries"),
            "Unexpected diagnostic: {error}"
        );
    }
}

#[test]
fn metasprite_signed_offsets_and_flicker_rotation_reach_dma_oam() {
    let emu = execute(
        r#"
        METASPRITE character
            TILE -8, 0, 10, 0
            TILE 0, 0, 11, 1
            TILE -8, 8, 12, 2
            TILE 0, 8, 13, 3
            TILE -8, 16, 14, $40
            TILE 0, 16, 15, $41
            TILE -8, 24, 16, $80
            TILE 0, 24, 17, $81
        END METASPRITE
        SUB Main()
            Sprite.SetFlicker(1)
            Sprite.Clear()
            Sprite.Draw(40, 50, character)
            Sprite.Clear()
            Sprite.Draw(40, 50, character)
            PPU.Ctrl($80)
        END SUB
        "#,
    );
    let ram = emu.ram_snapshot();
    let expected = [
        50, 10, 0, 32, 50, 11, 1, 40, 58, 12, 2, 32, 58, 13, 3, 40, 66, 14, 0x40, 32, 66, 15, 0x41,
        40, 74, 16, 0x80, 32, 74, 17, 0x81, 40,
    ];
    assert_eq!(&ram[0x0200 + 88..0x0200 + 120], &expected);
    assert_eq!(&ram[0x0200 + 44..0x0200 + 76], &[255; 32]);
    // The emulator is paused here; its owned OAM storage cannot move while this
    // read-only slice is copied. This checks DMA, not only the compiler's shadow RAM.
    let oam =
        unsafe { std::slice::from_raw_parts(emu.get_oam_data(), emu.get_oam_data_len()) }.to_vec();
    assert_eq!(&oam[88..120], &expected);
}

#[test]
fn collision_excludes_empty_touching_shapes_and_keeps_16bit_endpoint_carry() {
    let emu = execute(
        r#"
        DIM results(8) AS BYTE
        SUB Main()
            results(0) = Collision.Rect(15, 15, 0, 20, 10, 10, 20, 20)
            results(1) = Collision.Rect(10, 10, 20, 20, 30, 10, 10, 10)
            results(2) = Collision.Rect(250, 250, 20, 20, 255, 255, 1, 1)
            results(3) = Collision.Rect(65530, 10, 20, 20, 65535, 15, 1, 1)
            results(4) = Collision.Rect(65535, 15, 1, 1, 65530, 10, 20, 20)
            results(5) = Collision.Point(65535, 15, 65530, 10, 20, 20)
            results(6) = Collision.Point(15, 15, 10, 10, 0, 20)
            results(7) = Collision.Point(30, 15, 10, 10, 20, 20)
        END SUB
        "#,
    );
    assert_eq!(
        &emu.ram_snapshot()[0x05c0..0x05c8],
        &[0, 0, 255, 255, 255, 255, 0, 0]
    );
}

#[test]
fn ten_slot_pool_full_failure_initialization_and_reuse_preserve_neighboring_ram() {
    let emu = execute(
        r#"
        TYPE Bullet
            active AS BYTE
            x AS BYTE
            y AS BYTE
        END TYPE
        DIM ids(10) AS BYTE
        DIM full AS WORD
        DIM reused AS WORD
        DIM i AS BYTE
        DIM guard AS BYTE = $3C
        DIM pool(10) AS Bullet
        DIM tail AS BYTE = $A5
        SUB Main()
            FOR i = 0 TO 9
                ids(i) = Pool.Spawn(pool)
            NEXT i
            full = Pool.Spawn(pool)
            pool(5).x = 77
            pool(5).y = 88
            Pool.Despawn(pool, 5)
            reused = Pool.Spawn(pool)
            Pool.Despawn(pool, 10)
            Pool.Despawn(pool, -1)
        END SUB
        "#,
    );
    let ram = emu.ram_snapshot();
    assert_eq!(&ram[0x05c0..0x05ca], &(0..10).collect::<Vec<_>>());
    assert_eq!(&ram[0x05ca..0x05cc], &[255, 255]);
    assert_eq!(&ram[0x05cc..0x05ce], &[5, 0]);
    assert_eq!(ram[0x05cf], 0x3c);
    assert_eq!(ram[0x05ee], 0xa5);
    for slot in 0..10 {
        assert_eq!(&ram[0x05d0 + slot * 3..0x05d3 + slot * 3], &[1, 0, 0]);
    }
}

#[test]
fn solid_tile_stops_movement_and_screen_out_of_bounds_never_aliases_map_bytes() {
    let mut assets: ProjectAssets = serde_json::from_value(serde_json::json!({
        "chr_bank": [], "palettes": [], "nametables": []
    }))
    .unwrap();
    let mut tiles = vec![0; 960];
    tiles[0] = 4;
    tiles[1] = 9;
    tiles[35] = 7;
    assets.nametables.push(Nametable {
        name: "Walls".into(),
        data: tiles,
        attrs: vec![0xa5; 64],
        metatile_grid: vec![],
    });
    let emu = execute_with_assets(
        r#"
        DIM lookups(5) AS BYTE
        DIM x AS BYTE
        DIM blocked AS BYTE
        DIM i AS BYTE
        SUB Main()
            lookups(0) = Collision.Tile(0, 0)
            lookups(1) = Collision.Tile(8, 0)
            lookups(2) = Collision.Tile(256, 0)
            lookups(3) = Collision.Tile(0, 240)
            lookups(4) = Collision.Tile(0, 256)
            FOR i = 0 TO 31
                IF Collision.Tile(x + 8, 8) = 0 THEN
                    x = x + 1
                ELSE
                    blocked = 1
                END IF
            NEXT i
        END SUB
        "#,
        Some(assets),
    );
    let ram = emu.ram_snapshot();
    assert_eq!(&ram[0x05c0..0x05c5], &[4, 9, 0, 0, 0]);
    assert_eq!(
        ram[0x05c5], 16,
        "Character must stop immediately before solid tile"
    );
    assert_eq!(ram[0x05c6], 1);
}

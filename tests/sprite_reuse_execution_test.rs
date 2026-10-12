use swiss_emulator::Emulator;
use swissarmynes::{compiler::source_map::LinkedSourceMap, server::api::compile_source};

fn run(source: &str) -> (Emulator, LinkedSourceMap, Vec<u8>) {
    let (rom, map) = compile_source(Some(source.into()), None, None)
        .expect("Reusable sprite program must compile");
    let mut emulator = Emulator::new();
    emulator.load_rom(&rom).unwrap();
    for _ in 0..12 {
        emulator.step().unwrap();
        emulator.clear_audio_samples();
    }
    (emulator, map, rom)
}

const ART: &str = r#"
METASPRITE Body
 TILE -8, 0, 10, $22
 TILE 8, 8, 11, $81
END METASPRITE
ANIMATION Walk
 FRAME Body, 2
 FRAME Body, 2
 LOOP
END ANIMATION
DIM actor AS AnimState
"#;

#[test]
fn palette_variants_and_whole_frame_mirrors_preserve_tiles_priority_and_default_draw() {
    let source=format!("{ART}\nSUB Main()\n Animation.Play(actor, Walk)\n Sprite.Clear()\n Animation.DrawVariant(100, 40, actor, 1, $C0)\n Sprite.DrawVariant(100, 80, Body, 2, $40)\n Sprite.Draw(20, 20, Body)\nEND SUB\n");
    let (emulator, map, rom) = run(&source);
    let ram = emulator.ram_snapshot();
    assert_eq!(
        &ram[0x200..0x208],
        &[48, 10, 0xE1, 108, 40, 11, 0x41, 92],
        "Both geometry and per-tile flips mirror, preserving priority"
    );
    assert_eq!(&ram[0x208..0x210], &[80, 10, 0x62, 108, 88, 11, 0xC2, 92]);
    assert_eq!(
        &ram[0x210..0x218],
        &[20, 10, 0x22, 12, 28, 11, 0x81, 28],
        "Default Draw cannot inherit variant overrides"
    );
    assert_eq!(map.asset_reuse.unique_metasprites, 1);
    assert_eq!(map.asset_reuse.unique_animations, 1);
    let recolored = source.replace("actor, 1, $C0", "actor, 3, $C0");
    let (_, other_map, other_rom) = run(&recolored);
    let chr = 16 + usize::from(rom[4]) * 16384;
    let other_chr = 16 + usize::from(other_rom[4]) * 16384;
    assert_eq!(
        &rom[chr..],
        &other_rom[other_chr..],
        "Changing colors must add or change no CHR data"
    );
    assert_eq!(
        map.asset_reuse.metadata_after_dedup, other_map.asset_reuse.metadata_after_dedup,
        "Color choices do not duplicate frames"
    );
}

#[test]
fn identical_metasprites_and_animation_sequences_share_real_rom_addresses() {
    let source=format!("{ART}\nMETASPRITE Clone\n TILE -8,0,10,$22\n TILE 8,8,11,$81\nEND METASPRITE\nANIMATION SameWalk\n FRAME Clone,2\n FRAME Body,2\n LOOP\nEND ANIMATION\nDIM addresses(4) AS WORD\nSUB Main()\n addresses(0)=Body\n addresses(1)=Clone\n addresses(2)=Walk\n addresses(3)=SameWalk\nEND SUB\n");
    let (emulator, map, _) = run(&source);
    let ram = emulator.ram_snapshot();
    // AnimState occupies the first five user bytes; addresses follow it.
    assert_eq!(&ram[0x5c5..0x5c7], &ram[0x5c7..0x5c9]);
    assert_eq!(&ram[0x5c9..0x5cb], &ram[0x5cb..0x5cd]);
    let stats = map.asset_reuse;
    assert_eq!(
        (
            stats.metasprites,
            stats.unique_metasprites,
            stats.animations,
            stats.unique_animations
        ),
        (2, 1, 2, 1)
    );
    assert_eq!(stats.metadata_before_dedup, 38);
    assert_eq!(stats.metadata_after_dedup, 19);
    assert_eq!(stats.metadata_bytes_saved, 19);
}

#[test]
fn full_64_tile_variant_reads_bounds_across_a_page_and_keeps_legacy_64_limit() {
    let tiles = (0..64)
        .map(|n| format!(" TILE {}, {}, {n}, 0\n", (n % 16) * 8, (n / 16) * 8))
        .collect::<String>();
    let source=format!("METASPRITE Large\n{tiles}END METASPRITE\nSUB Main()\n Sprite.Clear()\n Sprite.DrawVariant(32,64,Large,3,$C0)\n Sprite.DrawVariant(32,64,Large,1,0)\nEND SUB\n");
    let (emulator, _, _) = run(&source);
    let ram = emulator.ram_snapshot();
    assert_eq!(ram[0x1b], 64);
    assert_eq!(&ram[0x200..0x204], &[88, 0, 0xC3, 152]);
    assert_eq!(&ram[0x2fc..0x300], &[64, 63, 0xC3, 32]);
}

#[test]
fn invalid_variant_signatures_and_palette_values_are_actionable_errors() {
    for call in [
        "Sprite.DrawVariant(1,2,Body)",
        "Animation.DrawVariant(1,2,actor,1)",
        "Sprite.DrawVariant(1,2,Body,4,0)",
    ] {
        let source = format!("{ART}\nSUB Main()\n {call}\nEND SUB\n");
        let error = compile_source(Some(source), None, None).unwrap_err();
        assert!(
            error.contains("expects 5 arguments") || error.contains("palette must be"),
            "{error}"
        );
    }
}

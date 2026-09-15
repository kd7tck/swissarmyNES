use swiss_emulator::Emulator;
use swissarmynes::server::api::compile_source;

fn run_emulator_for_frames(emu: &mut Emulator, frames: usize) {
    for _ in 0..frames {
        emu.step().expect("Emulator execution failed");
    }
}

#[test]
fn test_phase11_controller_execution() {
    let source = r#"
        DIM p1_held AS BYTE

        SUB Main()
            DO
                Controller.Read()
                LET p1_held = Controller.IsHeld(Button.A)
            LOOP WHILE 1
        END SUB
    "#;

    let (rom_bytes, _) = compile_source(Some(source.to_string()), None, None).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom_bytes).unwrap();

    // Set button A pressed before frame runs
    emu.set_button(0, 0, true);

    run_emulator_for_frames(&mut emu, 5);

    let wram = emu.ram_snapshot();

    // $05C0 = p1_held
    assert_ne!(
        wram[0x05C0] & 0x80,
        0,
        "Button A (bit 7) should be active in State(0)"
    );
    emu.set_button(0, 0, false);
    run_emulator_for_frames(&mut emu, 2);
    assert_eq!(
        emu.ram_snapshot()[0x05c0],
        0,
        "Released A must not remain held"
    );
}

#[test]
fn test_phase15_pool_execution() {
    let source = r#"
        TYPE Bullet
            active AS BYTE
            x AS BYTE
            y AS BYTE
        END TYPE

        DIM id1 AS INT
        DIM id2 AS INT
        DIM id_reuse AS INT
        DIM pool(10) AS Bullet

        SUB Main()
            id1 = Pool.Spawn(pool)
            id2 = Pool.Spawn(pool)

            ' Despawn first slot
            Pool.Despawn(pool, id1)

            ' Next spawn should reuse id1
            id_reuse = Pool.Spawn(pool)
        END SUB
    "#;

    let (rom_bytes, _) = compile_source(Some(source.to_string()), None, None).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom_bytes).unwrap();

    run_emulator_for_frames(&mut emu, 10);

    let wram = emu.ram_snapshot();

    // INT variables occupy one signed byte each.
    let val_id1 = wram[0x05C0];
    let val_id2 = wram[0x05C1];
    let val_id_reuse = wram[0x05C2];

    assert_eq!(val_id1, 0, "First spawn should return index 0");
    assert_eq!(val_id2, 1, "Second spawn should return index 1");
    assert_eq!(
        val_id_reuse, 0,
        "Spawn after despawning slot 0 should reuse index 0"
    );
}

#[test]
fn test_phase16_17_collision_execution() {
    let source = r#"
        DIM hit_rect AS BYTE
        DIM hit_point AS BYTE
        DIM tile_idx AS BYTE
        DIM miss_rect AS BYTE
        DIM miss_point AS BYTE

        SUB Main()
            ' Collision.Rect(x1, y1, w1, h1, x2, y2, w2, h2)
            IF Collision.Rect(10, 10, 20, 20, 15, 15, 10, 10) THEN
                LET hit_rect = 1
            ELSE
                LET hit_rect = 0
            END IF

            ' Collision.Point(px, py, rx, ry, rw, rh)
            IF Collision.Point(15, 15, 10, 10, 20, 20) THEN
                LET hit_point = 1
            ELSE
                LET hit_point = 0
            END IF

            ' Collision.Tile(x, y)
            LET tile_idx = Collision.Tile(16, 16)
            LET miss_rect = Collision.Rect(10, 10, 20, 20, 40, 40, 10, 10)
            LET miss_point = Collision.Point(40, 40, 10, 10, 20, 20)
        END SUB
    "#;

    let (rom_bytes, _) = compile_source(Some(source.to_string()), None, None).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom_bytes).unwrap();

    run_emulator_for_frames(&mut emu, 10);

    let wram = emu.ram_snapshot();

    assert_eq!(
        wram[0x05C0], 1,
        "Collision.Rect overlapping boxes should return 1 (true)"
    );
    assert_eq!(
        wram[0x05C1], 1,
        "Collision.Point inside point should return 1 (true)"
    );
    // The absent nametable asset is initialized to zero.
    assert_eq!(
        wram[0x05C2], 0,
        "Collision.Tile(16,16) should return nametable byte at offset"
    );
    assert_eq!(wram[0x05c3], 0, "Separated rectangles must not collide");
    assert_eq!(wram[0x05c4], 0, "An outside point must not collide");
}

#[test]
fn test_phase20_rng_execution() {
    let source = r#"
        DIM r1 AS BYTE
        DIM r2 AS BYTE

        SUB Main()
            LET r1 = RND(100)
            LET r2 = RND(100)
        END SUB
    "#;

    let (rom_bytes, _) = compile_source(Some(source.to_string()), None, None).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom_bytes).unwrap();

    run_emulator_for_frames(&mut emu, 10);

    let wram = emu.ram_snapshot();

    assert!(
        wram[0x05C0] < 100,
        "RND(100) result r1 (${:02X}) must be < 100",
        wram[0x05C0]
    );
    assert!(
        wram[0x05C1] < 100,
        "RND(100) result r2 (${:02X}) must be < 100",
        wram[0x05C1]
    );
}

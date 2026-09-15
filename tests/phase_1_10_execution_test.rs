use swiss_emulator::Emulator;
use swissarmynes::server::api::compile_source;

fn run_emulator_for_frames(emu: &mut Emulator, frames: usize) {
    for _ in 0..frames {
        emu.step().expect("Emulator execution failed");
    }
}

#[test]
fn test_phase2_string_execution() {
    let source = r#"
        DIM s AS STRING
        SUB Main()
            LET s = "NES"
        END SUB
    "#;

    let (rom_bytes, _) = compile_source(Some(source.to_string()), None, None).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom_bytes).unwrap();
    run_emulator_for_frames(&mut emu, 30);

    let wram = emu.ram_snapshot();

    // Variable 's' (STRING pointer) is at $05C0 / $05C1
    let ptr = (wram[0x05C0] as u16) | ((wram[0x05C1] as u16) << 8);
    assert_ne!(ptr, 0, "String pointer should not be null");
    assert!(
        ptr >= 0xC000,
        "String literal pointer (${:04X}) should be in Bank 7 ROM ($C000+)",
        ptr
    );

    // Read string from Bank 7 ROM in rom_bytes
    let rom_offset = 16 + 114688 + (ptr - 0xC000) as usize;
    let chars = &rom_bytes[rom_offset..rom_offset + 3];
    assert_eq!(chars, b"NES");
}

#[test]
fn test_phase2_string_deduplication() {
    let source = r#"
        DIM s1 AS STRING
        DIM s2 AS STRING
        SUB Main()
            LET s1 = "Hello"
            LET s2 = "Hello"
        END SUB
    "#;

    let (rom_bytes, map) = compile_source(Some(source.to_string()), None, None).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom_bytes).unwrap();
    run_emulator_for_frames(&mut emu, 30);

    let wram = emu.ram_snapshot();

    let ptr1 = (wram[0x05C0] as u16) | ((wram[0x05C1] as u16) << 8);
    let ptr2 = (wram[0x05C2] as u16) | ((wram[0x05C3] as u16) << 8);

    assert_eq!(
        ptr1, ptr2,
        "Identical string literals should share the same ROM address"
    );
    assert!(!map.is_empty(), "Source map should be non-empty");
    assert!(rom_bytes.len() == 139280, "ROM length should match 139280");
}

#[test]
fn test_phase3_data_read_restore_execution() {
    let source = r#"
        DIM a AS BYTE
        DIM b AS BYTE
        DIM c AS BYTE

        DATA 10, 20, 30

        SUB Main()
            READ a
            READ b
            READ c
        END SUB
    "#;

    let (rom_bytes, _) = compile_source(Some(source.to_string()), None, None).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom_bytes).unwrap();
    run_emulator_for_frames(&mut emu, 30);

    let wram = emu.ram_snapshot();

    assert_eq!(wram[0x05C0], 10);
    assert_eq!(wram[0x05C1], 20);
    assert_eq!(wram[0x05C2], 30);
}

#[test]
fn test_phase3_data_read_types_and_restore() {
    let source = r#"
        DIM b AS BYTE
        DIM w AS WORD
        DIM s AS STRING

        DATA 42, 1000, "NES"

        Block2: DATA 99

        SUB Main()
            READ b, w, s
            RESTORE Block2
            READ b
        END SUB
    "#;

    let (rom_bytes, _) = compile_source(Some(source.to_string()), None, None).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom_bytes).unwrap();
    run_emulator_for_frames(&mut emu, 30);

    let wram = emu.ram_snapshot();

    let read_word = (wram[0x05C1] as u16) | ((wram[0x05C2] as u16) << 8);
    let str_ptr = (wram[0x05C3] as u16) | ((wram[0x05C4] as u16) << 8);

    assert_eq!(read_word, 1000, "READ WORD should equal 1000");
    assert_ne!(str_ptr, 0, "String pointer should not be null");

    let rom_offset = 16 + 114688 + (str_ptr - 0xC000) as usize;
    assert_eq!(&rom_bytes[rom_offset..rom_offset + 3], b"NES");

    // RESTORE Block2 was executed, so second READ b gets 99
    assert_eq!(
        wram[0x05C0], 99,
        "RESTORE Block2 should reset read pointer to Block2 (99)"
    );
}

#[test]
fn test_phase3_data_bank5_execution() {
    let source = r#"
        DIM b AS BYTE

        DATA 88

        BANK 5
        SUB ReadData()
            READ b
        END SUB

        BANK 0
        SUB Main()
            ReadData()
        END SUB
    "#;

    let (rom_bytes, _) = compile_source(Some(source.to_string()), None, None).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom_bytes).unwrap();
    run_emulator_for_frames(&mut emu, 30);

    let wram = emu.ram_snapshot();

    assert_eq!(
        wram[0x05C0], 88,
        "READ in Bank 5 should read fixed Bank 7 DATA (88)"
    );
}

#[test]
fn test_phase5_word_math_execution() {
    let source = r#"
        DIM w AS WORD
        SUB Main()
            LET w = 1000 + 500
        END SUB
    "#;

    let (rom_bytes, _) = compile_source(Some(source.to_string()), None, None).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom_bytes).unwrap();
    run_emulator_for_frames(&mut emu, 30);

    let wram = emu.ram_snapshot();

    let val = (wram[0x05C0] as u16) | ((wram[0x05C1] as u16) << 8);
    assert_eq!(val, 1500, "1000 + 500 should equal 1500");
}

#[test]
fn test_phase6_advanced_math_execution() {
    let source = r#"
        DIM prod AS WORD
        DIM cond AS BYTE
        SUB Main()
            LET prod = 200 * 50
            IF -5 < 10 THEN
                LET cond = 1
            ELSE
                LET cond = 0
            END IF
        END SUB
    "#;

    let (rom_bytes, _) = compile_source(Some(source.to_string()), None, None).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom_bytes).unwrap();
    run_emulator_for_frames(&mut emu, 30);

    let wram = emu.ram_snapshot();

    let prod_val = (wram[0x05C0] as u16) | ((wram[0x05C1] as u16) << 8);
    assert_eq!(prod_val, 10000, "200 * 50 should equal 10000");
    assert_eq!(wram[0x05C2], 1, "-5 < 10 should be true (cond = 1)");
}

#[test]
fn test_phase7_select_case_execution() {
    let source = r#"
        DIM target AS BYTE
        DIM result AS BYTE
        SUB Main()
            LET target = 2
            SELECT CASE target
                CASE 1
                    LET result = 10
                CASE 2
                    LET result = 20
                CASE ELSE
                    LET result = 99
            END SELECT
        END SUB
    "#;

    let (rom_bytes, _) = compile_source(Some(source.to_string()), None, None).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom_bytes).unwrap();
    run_emulator_for_frames(&mut emu, 30);

    let wram = emu.ram_snapshot();

    assert_eq!(wram[0x05C1], 20, "CASE 2 should match and set result = 20");
}

#[test]
fn test_phase8_structs_execution() {
    let source = r#"
        TYPE Player
            x AS BYTE
            y AS BYTE
            hp AS WORD
        END TYPE

        DIM p AS Player

        SUB Main()
            LET p.x = 15
            LET p.y = 25
            LET p.hp = 100
        END SUB
    "#;

    let (rom_bytes, _) = compile_source(Some(source.to_string()), None, None).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom_bytes).unwrap();
    run_emulator_for_frames(&mut emu, 30);

    let wram = emu.ram_snapshot();

    assert_eq!(wram[0x05C0], 15, "p.x should be 15");
    assert_eq!(wram[0x05C1], 25, "p.y should be 25");
    let hp = (wram[0x05C2] as u16) | ((wram[0x05C3] as u16) << 8);
    assert_eq!(hp, 100, "p.hp should be 100");
}

#[test]
fn test_phase9_enums_execution() {
    let source = r#"
        ENUM State
            Idle
            Running
            Jumping
        END ENUM

        DIM current AS BYTE

        SUB Main()
            LET current = State.Jumping
        END SUB
    "#;

    let (rom_bytes, _) = compile_source(Some(source.to_string()), None, None).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom_bytes).unwrap();
    run_emulator_for_frames(&mut emu, 30);

    let wram = emu.ram_snapshot();

    assert_eq!(wram[0x05C0], 2, "State.Jumping should be 2");
}

#[test]
fn test_phase10_macros_execution() {
    let source = r#"
        DEF MACRO SetVal(var, val)
            LET var = val
        END MACRO

        DIM x AS BYTE

        SUB Main()
            SetVal(x, 42)
        END SUB
    "#;

    let (rom_bytes, _) = compile_source(Some(source.to_string()), None, None).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom_bytes).unwrap();
    run_emulator_for_frames(&mut emu, 30);

    let wram = emu.ram_snapshot();

    assert_eq!(wram[0x05C0], 42, "Macro expanded set x = 42");
}

#[test]
fn test_phase10_macro_recursion_rejection() {
    let source = r#"
        DEF MACRO Recurse()
            Recurse()
        END MACRO

        SUB Main()
            Recurse()
        END SUB
    "#;

    let res = compile_source(Some(source.to_string()), None, None);
    assert!(res.is_err(), "Recursive macro should fail compilation");
    assert!(
        res.err()
            .unwrap()
            .contains("Macro expansion recursion limit exceeded"),
        "Error should cite macro expansion recursion limit"
    );
}

use swiss_emulator::Emulator;
use swissarmynes::server::api::compile_source;

fn run_emulator_for_frames(emu: &mut Emulator, frames: usize) {
    for _ in 0..frames {
        emu.step().expect("Emulator execution failed");
    }
}

#[test]
fn test_select_case_ranges_and_is_and_multiple_conditions() {
    let source = "
        DIM val AS BYTE
        DIM res1 AS BYTE
        DIM res2 AS BYTE
        DIM res3 AS BYTE

        SUB Main()
            LET val = 5
            SELECT CASE val
                CASE 1 TO 10
                    LET res1 = 100
                CASE ELSE
                    LET res1 = 0
            END SELECT

            LET val = 25
            SELECT CASE val
                CASE IS > 20
                    LET res2 = 200
                CASE ELSE
                    LET res2 = 0
            END SELECT

            LET val = 3
            SELECT CASE val
                CASE 1, 2, 3
                    LET res3 = 99
                CASE ELSE
                    LET res3 = 0
            END SELECT
        END SUB
    ";

    let (rom_bytes, _) = compile_source(Some(source.to_string()), None, None).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom_bytes).unwrap();
    run_emulator_for_frames(&mut emu, 30);

    let wram = emu.ram_snapshot();
    assert_eq!(wram[0x05C1], 100, "res1 should be 100 for CASE 1 TO 10");
    assert_eq!(wram[0x05C2], 200, "res2 should be 200 for CASE IS > 20");
    assert_eq!(wram[0x05C3], 99, "res3 should be 99 for CASE 1, 2, 3");
}

use swiss_emulator::Emulator;
use swissarmynes::server::api::compile_source;

fn run_emulator_for_frames(emu: &mut Emulator, frames: usize) {
    for _ in 0..frames {
        emu.step().expect("Emulator execution failed");
    }
}

#[test]
fn test_for_loop_byte_down_to_zero() {
    let source = "
        DIM i AS BYTE
        DIM count AS BYTE

        SUB Main()
            LET count = 0
            FOR i = 10 TO 0 STEP -1
                LET count = count + 1
            NEXT i
        END SUB
    ";

    let (rom_bytes, _) = compile_source(Some(source.to_string()), None, None).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom_bytes).unwrap();
    run_emulator_for_frames(&mut emu, 30);

    let wram = emu.ram_snapshot();
    assert_eq!(
        wram[0x05C1], 11,
        "FOR i = 10 TO 0 STEP -1 should execute 11 times (count = 11)"
    );
}

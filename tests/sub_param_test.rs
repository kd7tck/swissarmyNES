use swiss_emulator::Emulator;
use swissarmynes::server::api::compile_source;

fn run_emulator_for_frames(emu: &mut Emulator, frames: usize) {
    for _ in 0..frames {
        emu.step().expect("Emulator execution failed");
    }
}

#[test]
fn test_sub_parameter_codegen_and_body_access() {
    let source = "
        DIM res AS BYTE

        SUB AddTen(v AS BYTE)
            LET res = v + 10
        END SUB

        SUB Main()
            CALL AddTen(5)
        END SUB
    ";

    let (rom_bytes, _) = compile_source(Some(source.to_string()), None, None).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom_bytes).unwrap();
    run_emulator_for_frames(&mut emu, 30);

    let wram = emu.ram_snapshot();
    assert_eq!(wram[0x05C0], 15, "AddTen(5) should set res ($05C0) to 15");
}

use swiss_emulator::{CpuState, Emulator};

fn hex_field(line: &str, prefix: &str, length: usize) -> u64 {
    let value = line.split_once(prefix).unwrap().1;
    u64::from_str_radix(&value[..length], 16).unwrap()
}

fn compare_record(state: &CpuState, line: &str) -> Result<(), String> {
    let expected = [
        u64::from_str_radix(&line[..4], 16).unwrap(),
        hex_field(line, "A:", 2),
        hex_field(line, "X:", 2),
        hex_field(line, "Y:", 2),
        hex_field(line, "P:", 2),
        hex_field(line, "SP:", 2),
        line.split_once("CYC:").unwrap().1.trim().parse().unwrap(),
    ];
    let actual = [
        u64::from(state.pc),
        u64::from(state.acc),
        u64::from(state.x),
        u64::from(state.y),
        u64::from(state.status),
        u64::from(state.sp),
        state.cycles as u64,
    ];
    for (index, field) in ["PC", "A", "X", "Y", "P", "SP", "cycles"]
        .iter()
        .enumerate()
    {
        if actual[index] != expected[index] {
            return Err(format!(
                "{field}: expected {}, observed {}",
                expected[index], actual[index]
            ));
        }
    }
    Ok(())
}

#[test]
fn canonical_nestest_full_trace_matches_production_core() {
    let mut rom = include_bytes!("fixtures/nestest.nes").to_vec();
    // The documented automation entry is C000. Change only the reset vector in
    // the in-memory copy; the checked-in fixture and every executed opcode stay intact.
    rom[16 + 0x3ffc..16 + 0x3ffe].copy_from_slice(&0xc000u16.to_le_bytes());
    let mut emulator = Emulator::new();
    emulator.load_rom(&rom).unwrap();
    // Automation only writes these bytes on failure; production power-on RAM
    // is random, and the reference trace never initializes the result slots.
    let initial_results = [emulator.peek_cpu(2), emulator.peek_cpu(3)];
    let trace = include_str!("fixtures/nestest.trace");
    assert_eq!(trace.lines().count(), 8991);
    for (index, line) in trace.lines().enumerate() {
        let state = emulator.trace_instruction().unwrap();
        compare_record(&state, line)
            .unwrap_or_else(|error| panic!("trace record {}: {line}: {error}", index + 1));
    }
    assert_eq!(
        [emulator.peek_cpu(2), emulator.peek_cpu(3)],
        initial_results
    );
}

#[test]
fn comparator_rejects_each_changed_field_and_every_status_bit() {
    let line = include_str!("fixtures/nestest.trace")
        .lines()
        .next()
        .unwrap();
    let baseline = || CpuState {
        pc: 0xc000,
        acc: 0,
        x: 0,
        y: 0,
        status: 0x24,
        sp: 0xfd,
        cycles: 7,
    };
    assert!(compare_record(&baseline(), line).is_ok());
    for field in 0..7 {
        let mut state = baseline();
        match field {
            0 => state.pc ^= 1,
            1 => state.acc ^= 1,
            2 => state.x ^= 1,
            3 => state.y ^= 1,
            4 => state.status ^= 1,
            5 => state.sp ^= 1,
            _ => state.cycles += 1,
        }
        assert!(compare_record(&state, line).is_err(), "field {field}");
    }
    for bit in 0..8 {
        let mut state = baseline();
        state.status ^= 1 << bit;
        assert!(compare_record(&state, line).is_err(), "status bit {bit}");
    }
}

fn status_program(code: &[u8]) -> Emulator {
    let mut rom = vec![0; 16 + 16384 + 8192];
    rom[..8].copy_from_slice(&[b'N', b'E', b'S', 0x1a, 1, 1, 0, 0]);
    rom[16..16 + code.len()].copy_from_slice(code);
    rom[16 + 0x3ffc..16 + 0x3ffe].copy_from_slice(&0x8000u16.to_le_bytes());
    let mut emulator = Emulator::new();
    emulator.load_rom(&rom).unwrap();
    emulator
}

#[test]
fn plp_snapshot_preserves_six_flags_and_php_serializes_stack_bits() {
    // Exercise every pulled value, including all combinations of nonphysical
    // B/U bits. PHP must still push both bits even though snapshots normalize them.
    for value in 0..=255u8 {
        let mut emulator = status_program(&[0xa9, value, 0x48, 0x28, 0x08, 0xea]);
        for _ in 0..3 {
            emulator.trace_instruction().unwrap();
        }
        assert_eq!(emulator.get_cpu_state().status, (value & 0xcf) | 0x20);
        emulator.trace_instruction().unwrap();
        assert_eq!(emulator.peek_cpu(0x1fd), value | 0x30);
    }
}

#[test]
fn rti_restores_pc_and_six_flags_independently_of_stack_bits() {
    for value in 0..=255u8 {
        // Push return PC $8010 high/low, then status, and execute RTI.
        let mut emulator =
            status_program(&[0xa9, 0x80, 0x48, 0xa9, 0x10, 0x48, 0xa9, value, 0x48, 0x40]);
        for _ in 0..7 {
            emulator.trace_instruction().unwrap();
        }
        let state = emulator.get_cpu_state();
        assert_eq!(state.pc, 0x8010);
        assert_eq!(state.sp, 0xfd);
        assert_eq!(state.status, (value & 0xcf) | 0x20);
        assert_eq!(state.cycles, 28);
    }
}

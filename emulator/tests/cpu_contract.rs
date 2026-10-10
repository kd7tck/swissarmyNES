use std::collections::BTreeSet;
use swiss_emulator::Emulator;
use tetanes_core::{
    control_deck::ControlDeck,
    cpu::{instr::Operation, Cpu, Status},
};

// Independent official encoding inventory. Dispatch comes from the production
// core; this list is a coverage oracle, not a second instruction implementation.
const OFFICIAL: &str = "
00 01 05 06 08 09 0A 0D 0E
10 11 15 16 18 19 1D 1E
20 21 24 25 26 28 29 2A 2C 2D 2E
30 31 35 36 38 39 3D 3E
40 41 45 46 48 49 4A 4C 4D 4E
50 51 55 56 58 59 5D 5E
60 61 65 66 68 69 6A 6C 6D 6E
70 71 75 76 78 79 7D 7E
81 84 85 86 88 8A 8C 8D 8E
90 91 94 95 96 98 99 9A 9D
A0 A1 A2 A4 A5 A6 A8 A9 AA AC AD AE
B0 B1 B4 B5 B6 B8 B9 BA BC BD BE
C0 C1 C4 C5 C6 C8 C9 CA CC CD CE
D0 D1 D5 D6 D8 D9 DD DE
E0 E1 E4 E5 E6 E8 E9 EA EC ED EE
F0 F1 F5 F6 F8 F9 FD FE";

fn image(code: &[u8]) -> Vec<u8> {
    let mut rom = vec![0; 16 + 16384 + 8192];
    rom[..8].copy_from_slice(&[b'N', b'E', b'S', 0x1a, 1, 1, 0, 0]);
    rom[16..16 + code.len()].copy_from_slice(code);
    rom[16 + 0x100] = 0x40;
    rom[16 + 0x3ffa..16 + 0x4000].copy_from_slice(&[0, 0x81, 0, 0x80, 0, 0x81]);
    rom
}

#[test]
fn all_256_encodings_are_classified_and_151_official_encodings_have_execution_evidence() {
    let official: BTreeSet<_> = OFFICIAL
        .split_whitespace()
        .map(|hex| u8::from_str_radix(hex, 16).unwrap())
        .collect();
    assert_eq!(official.len(), 151);
    let mut mnemonics = BTreeSet::new();
    let mut jams = Vec::new();
    for (opcode, instr) in Cpu::INSTRUCTIONS.iter().enumerate() {
        assert_eq!(instr.opcode(), opcode as u8, "dispatcher entry identity");
        assert!((2..=8).contains(&instr.cycles()));
        if official.contains(&(opcode as u8)) {
            assert_ne!(instr.op(), Operation::XXX);
            mnemonics.insert(format!("{:?}", instr.op()));
        } else if instr.op() == Operation::XXX {
            jams.push(opcode as u8);
        }
    }
    assert_eq!(mnemonics.len(), 56);
    assert_eq!(
        jams,
        [0x02, 0x12, 0x22, 0x32, 0x42, 0x52, 0x62, 0x72, 0x92, 0xb2, 0xd2, 0xf2]
    );
    let traced: BTreeSet<_> = include_str!("fixtures/nestest.trace")
        .lines()
        .map(|line| u8::from_str_radix(&line[6..8], 16).unwrap())
        .collect();
    let missing: Vec<_> = official.difference(&traced).copied().collect();
    // These two encodings have explicit integration regressions: BRK/status
    // stacking in cpu_interrupts.rs and CLI below (plus held APU IRQ polling).
    assert_eq!(missing, [0x00, 0x58]);
    let mut emulator = Emulator::new();
    emulator.load_rom(&image(&[0x78, 0x58, 0xea])).unwrap();
    emulator.trace_instruction().unwrap();
    let before = emulator.get_cpu_state();
    emulator.trace_instruction().unwrap();
    let after = emulator.get_cpu_state();
    assert_eq!(after.status & 4, 0);
    assert_eq!(after.cycles - before.cycles, 2);
}

#[test]
fn adc_and_sbc_are_binary_for_every_operand_carry_and_decimal_flag() {
    for (opcode, subtract) in [(0x65, false), (0xe5, true)] {
        let mut deck = ControlDeck::new();
        deck.load_rom("alu.nes", &mut &image(&[opcode, 0x00])[..])
            .unwrap();
        for decimal in [0, 8] {
            for carry in 0..=1u8 {
                for left in 0..=255u8 {
                    for right in 0..=255u8 {
                        let cpu = deck.cpu_mut();
                        cpu.pc = 0x8000;
                        cpu.acc = left;
                        cpu.status = Status::from_bits_retain(0x24 | decimal | carry);
                        cpu.bus.wram[0] = right;
                        let (value, carry_out, overflow) = if subtract {
                            let difference =
                                i16::from(left) - i16::from(right) - i16::from(1 - carry);
                            let value = difference as u8;
                            (
                                value,
                                difference >= 0,
                                ((left ^ value) & (left ^ right) & 0x80) != 0,
                            )
                        } else {
                            let sum = u16::from(left) + u16::from(right) + u16::from(carry);
                            let value = sum as u8;
                            (
                                value,
                                sum > 255,
                                (!(left ^ right) & (left ^ value) & 0x80) != 0,
                            )
                        };
                        assert_eq!(deck.clock_instr().unwrap(), 3);
                        let expected_flags = 0x24
                            | decimal
                            | u8::from(carry_out)
                            | if value == 0 { 2 } else { 0 }
                            | (value & 0x80)
                            | if overflow { 0x40 } else { 0 };
                        assert_eq!(
                            (deck.cpu().acc, deck.cpu().status.bits()),
                            (value, expected_flags),
                            "opcode {opcode:02X}, {left:02X}, {right:02X}, C={carry}, D={decimal}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn all_jam_encodings_error_without_hanging_and_reset_recovers() {
    for opcode in [
        0x02, 0x12, 0x22, 0x32, 0x42, 0x52, 0x62, 0x72, 0x92, 0xb2, 0xd2, 0xf2,
    ] {
        let mut emulator = Emulator::new();
        // Start at a JMP whose target comes from RAM, initially pointing at JAM.
        let rom = image(&[0x6c, 0, 0x02, opcode, 0xea]);
        emulator.load_rom(&rom).unwrap();
        use swiss_emulator::cartridge::Mapper;
        emulator.write_prg(0x200, 3);
        emulator.write_prg(0x201, 0x80);
        emulator.trace_instruction().unwrap();
        assert!(emulator.trace_instruction().is_err(), "JAM {opcode:02X}");
        assert!(
            emulator.step().is_err(),
            "halted core must not hang in the frame loop"
        );
        emulator.reset();
        emulator.write_prg(0x200, 4);
        emulator.write_prg(0x201, 0x80);
        assert_eq!(emulator.trace_instruction().unwrap().pc, 0x8000);
        assert_eq!(emulator.trace_instruction().unwrap().pc, 0x8004);
    }
}

#[test]
fn indexed_reads_and_branches_charge_page_crossings_and_zero_page_wraps() {
    // LDA $02FF,X crosses into $0300; LDA $0300,X does not. LDA $FF,X
    // wraps in zero page, while ($FF),Y wraps its pointer high read to $00.
    let mut emulator = Emulator::new();
    let code = [
        0xa2, 1, 0xbd, 0xff, 2, 0xbd, 0, 3, 0xb5, 0xff, 0xa0, 1, 0xb1, 0xff,
    ];
    emulator.load_rom(&image(&code)).unwrap();
    use swiss_emulator::cartridge::Mapper;
    emulator.write_prg(0x0300, 0x5a);
    emulator.write_prg(0x0301, 0xa5);
    emulator.write_prg(0x0400, 0);
    emulator.write_prg(0, 0x03);
    emulator.write_prg(0xff, 0xff);
    emulator.trace_instruction().unwrap();
    for (expected, cycles) in [(0x5a, 5), (0xa5, 4), (0x03, 4)] {
        let before = emulator.get_cpu_state().cycles;
        emulator.trace_instruction().unwrap();
        let state = emulator.get_cpu_state();
        assert_eq!((state.acc, state.cycles - before), (expected, cycles));
    }
    emulator.trace_instruction().unwrap();
    let before = emulator.get_cpu_state().cycles;
    emulator.trace_instruction().unwrap();
    assert_eq!(
        (
            emulator.get_cpu_state().acc,
            emulator.get_cpu_state().cycles - before
        ),
        (0, 6)
    );

    for (entry, offset, taken, cycles) in [
        (0x8000u16, 0x00, true, 3),
        (0x80fd, 0x01, true, 4),
        (0x80fd, 0x01, false, 2),
    ] {
        let mut rom = image(&[]);
        let origin = 16 + usize::from(entry - 0x8000);
        rom[origin..origin + 2].copy_from_slice(&[if taken { 0xd0 } else { 0xf0 }, offset]);
        rom[16 + 0x3ffc..16 + 0x3ffe].copy_from_slice(&entry.to_le_bytes());
        emulator.load_rom(&rom).unwrap();
        let before = emulator.get_cpu_state();
        emulator.trace_instruction().unwrap();
        assert_eq!(emulator.get_cpu_state().cycles - before.cycles, cycles);
        assert_eq!(
            emulator.get_cpu_state().pc,
            entry + 2 + if taken { u16::from(offset) } else { 0 }
        );
    }
}

use swiss_emulator::Emulator;

// These small NROM programs exercise the production loader, bus and CPU together.
fn program(code: &[u8]) -> Emulator {
    let mut rom = vec![0; 16 + 16384 + 8192];
    rom[..8].copy_from_slice(&[b'N', b'E', b'S', 0x1a, 1, 1, 0, 0]);
    rom[16..16 + code.len()].copy_from_slice(code);
    rom[16 + 0x100] = 0x40; // RTI at $8100, IRQ/BRK vector target.
    rom[16 + 0x3ffa..16 + 0x4000].copy_from_slice(&[0, 0x81, 0, 0x80, 0, 0x81]);
    let mut emulator = Emulator::new();
    emulator.load_rom(&rom).unwrap();
    emulator
}

#[test]
fn brk_pushes_two_byte_return_and_status_before_setting_i() {
    for value in 0..=255u8 {
        // LDA #status; PHA; PLP; BRK; padding; NOP.
        let mut emulator = program(&[0xa9, value, 0x48, 0x28, 0x00, 0xea, 0xea]);
        for _ in 0..3 {
            emulator.trace_instruction().unwrap();
        }
        let before = emulator.get_cpu_state();
        emulator.trace_instruction().unwrap();
        let entered = emulator.get_cpu_state();
        assert_eq!(entered.pc, 0x8100);
        assert_eq!(entered.sp, 0xfa);
        assert_eq!(entered.cycles - before.cycles, 7);
        assert_eq!(entered.status, (value & 0xcf) | 0x24);
        assert_eq!(emulator.peek_cpu(0x1fd), 0x80);
        assert_eq!(emulator.peek_cpu(0x1fc), 0x06);
        assert_eq!(emulator.peek_cpu(0x1fb), value | 0x30);
        emulator.trace_instruction().unwrap();
        let returned = emulator.get_cpu_state();
        assert_eq!(returned.pc, 0x8006);
        assert_eq!(returned.sp, before.sp);
        assert_eq!(returned.status, before.status);
        assert_eq!(returned.cycles - entered.cycles, 6);
    }
}

#[test]
fn stack_wraps_within_page_one() {
    // Start SP at zero. The second push wraps to $01FF, never $00FF.
    let mut emulator = program(&[
        0xa2, 0, 0x9a, 0xa9, 0x12, 0x48, 0xa9, 0x34, 0x48, 0x68, 0x68,
    ]);
    let zero_page = emulator.peek_cpu(0xff);
    for _ in 0..6 {
        emulator.trace_instruction().unwrap();
    }
    assert_eq!(emulator.get_cpu_state().sp, 0xfe);
    assert_eq!(emulator.peek_cpu(0x100), 0x12);
    assert_eq!(emulator.peek_cpu(0x1ff), 0x34);
    assert_eq!(emulator.peek_cpu(0xff), zero_page);
    emulator.trace_instruction().unwrap();
    assert_eq!(emulator.get_cpu_state().acc, 0x34);
    emulator.trace_instruction().unwrap();
    assert_eq!(emulator.get_cpu_state().acc, 0x12);
    assert_eq!(emulator.get_cpu_state().sp, 0);
}

#[test]
fn indirect_jump_high_byte_wraps_in_pointer_page() {
    // Distinguish the NMOS $02FF->$0200 wrap from an ordinary $0300 read.
    let mut emulator = program(&[
        0xa9, 0x34, 0x8d, 0xff, 0x02, 0xa9, 0x81, 0x8d, 0x00, 0x02, 0xa9, 0x82, 0x8d, 0x00, 0x03,
        0x6c, 0xff, 0x02,
    ]);
    for _ in 0..6 {
        emulator.trace_instruction().unwrap();
    }
    let before = emulator.get_cpu_state().cycles;
    emulator.trace_instruction().unwrap();
    assert_eq!(emulator.get_cpu_state().pc, 0x8134);
    assert_eq!(emulator.get_cpu_state().cycles - before, 5);
}

#[test]
fn ppu_nmi_uses_cartridge_vector_and_pushes_hardware_status() {
    // Enable vblank NMI with I still set. NMI must ignore I, unlike IRQ.
    let mut emulator = program(&[0xa9, 0x80, 0x8d, 0x00, 0x20, 0x4c, 0x05, 0x80]);
    for _ in 0..2 {
        emulator.trace_instruction().unwrap();
    }
    let flags = emulator.get_cpu_state().status;
    let mut entered = false;
    for _ in 0..40000 {
        let before = emulator.get_cpu_state();
        emulator.trace_instruction().unwrap();
        let after = emulator.get_cpu_state();
        if after.pc == 0x8100 {
            assert_eq!(after.cycles - before.cycles, 10); // JMP (3) + NMI (7)
            assert_eq!(after.sp, 0xfa);
            assert_eq!(emulator.peek_cpu(0x1fd), 0x80);
            assert_eq!(emulator.peek_cpu(0x1fc), 0x05);
            assert_eq!(emulator.peek_cpu(0x1fb), flags & !0x10);
            emulator.trace_instruction().unwrap();
            assert_eq!(emulator.get_cpu_state().pc, 0x8005);
            assert_eq!(emulator.get_cpu_state().status, flags);
            assert_eq!(emulator.get_cpu_state().sp, 0xfd);
            // A held vblank signal must not immediately retrigger after RTI.
            emulator.trace_instruction().unwrap();
            assert_eq!(emulator.get_cpu_state().pc, 0x8005);
            entered = true;
            break;
        }
    }
    assert!(entered, "PPU never delivered NMI within four NTSC frames");
}

#[test]
fn oam_dma_stalls_reads_for_both_cpu_alignments() {
    let mut observed = Vec::new();
    for padding in [vec![0xea], vec![0x24, 0x00]] {
        let mut code = padding;
        code.extend_from_slice(&[0xa9, 0x02, 0x8d, 0x14, 0x40, 0xea]);
        let mut emulator = program(&code);
        for _ in 0..3 {
            emulator.trace_instruction().unwrap();
        }
        let before = emulator.get_cpu_state();
        emulator.trace_instruction().unwrap();
        let after = emulator.get_cpu_state();
        assert_eq!(after.pc, before.pc + 1);
        observed.push(after.cycles - before.cycles);
    }
    observed.sort_unstable();
    // DMA consumes 513 or 514 cycles; the pending NOP still consumes its own two.
    assert_eq!(observed, [515, 516]);
}

#[test]
fn frame_irq_is_masked_until_cli_delay_and_remains_asserted_after_rti() {
    // Start the APU four-step frame sequencer, then wait longer than one frame
    // with I set. No handler acknowledges $4015, so the IRQ level stays asserted.
    let mut emulator = program(&[
        0x78, 0xa9, 0x00, 0x8d, 0x17, 0x40, 0xa0, 0x20, 0xa2, 0x00, 0xca, 0xd0, 0xfd, 0x88, 0xd0,
        0xf8, 0x58, 0x4c, 0x11, 0x80,
    ]);
    let mut reached_cli = false;
    for _ in 0..25000 {
        let state = emulator.get_cpu_state();
        assert_ne!(state.pc, 0x8100, "IRQ was taken while I was set");
        if state.pc == 0x8010 {
            reached_cli = true;
            break;
        }
        emulator.trace_instruction().unwrap();
    }
    assert!(reached_cli);
    assert_ne!(
        emulator.peek_cpu(0x4015) & 0x40,
        0,
        "frame IRQ never asserted"
    );
    emulator.trace_instruction().unwrap(); // CLI does not take the IRQ yet.
    let after_cli = emulator.get_cpu_state();
    assert_eq!(after_cli.pc, 0x8011);
    assert_eq!(after_cli.status & 4, 0);
    emulator.trace_instruction().unwrap(); // JMP followed by IRQ entry.
    let entered = emulator.get_cpu_state();
    assert_eq!(entered.pc, 0x8100);
    assert_eq!(entered.cycles - after_cli.cycles, 10);
    assert_eq!(emulator.peek_cpu(0x1fd), 0x80);
    assert_eq!(emulator.peek_cpu(0x1fc), 0x11);
    assert_eq!(emulator.peek_cpu(0x1fb) & 0x34, 0x20);
    emulator.trace_instruction().unwrap(); // RTI immediately re-enters held IRQ.
    let reentered = emulator.get_cpu_state();
    assert_eq!(reentered.pc, 0x8100);
    assert_eq!(reentered.sp, entered.sp);
    assert_eq!(reentered.cycles - entered.cycles, 13);
}

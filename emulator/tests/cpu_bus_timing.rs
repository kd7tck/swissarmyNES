use tetanes_core::{
    control_deck::ControlDeck,
    cpu::{Cpu, Irq, Status},
    mem::{Read, Write},
};

fn program(code: &[u8]) -> ControlDeck {
    let mut rom = vec![0; 16 + 16384 + 8192];
    rom[..8].copy_from_slice(&[b'N', b'E', b'S', 0x1a, 1, 1, 0, 0]);
    rom[16..16 + code.len()].copy_from_slice(code);
    rom[16 + 0x100] = 0x40; // IRQ RTI at $8100.
    rom[16 + 0x200] = 0x40; // NMI RTI at $8200.
    rom[16 + 0x400] = 0xa5; // DMC sample at mirrored $C400.
    rom[16 + 0x3ffa..16 + 0x4000].copy_from_slice(&[0, 0x82, 0, 0x80, 0, 0x81]);
    let mut deck = ControlDeck::new();
    deck.load_rom("cpu-bus-timing.nes", &mut &rom[..]).unwrap();
    deck
}

#[test]
fn official_and_unofficial_rmw_issue_old_then_new_ppudata_writes() {
    // PPUDATA increments its VRAM address on a read AND each write. The old
    // dummy write therefore occupies $2001 and the result occupies $2002.
    // A single final write cannot satisfy these device-observable assertions.
    for (opcode, result) in [
        (0xee, 0x13), // INC
        (0xce, 0x11), // DEC
        (0x0e, 0x24), // ASL
        (0x4e, 0x09), // LSR
        (0x2e, 0x25), // ROL through carry
        (0x6e, 0x89), // ROR through carry
        (0x0f, 0x24), // SLO
        (0x2f, 0x25), // RLA
        (0x4f, 0x09), // SRE
        (0x6f, 0x89), // RRA
        (0xcf, 0x11), // DCP
        (0xef, 0x13), // ISC
    ] {
        let mut deck = program(&[opcode, 0x07, 0x20, 0xea]);
        // This is power-on fixture setup, not a replacement CPU/bus implementation.
        deck.ppu_mut().emulate_warmup = false;
        deck.ppu_mut().vram_buffer = 0x12;
        deck.ppu_mut().bus.write(0x2000, 0x34);
        deck.ppu_mut().bus.write(0x2001, 0x56);
        deck.ppu_mut().bus.write(0x2002, 0x78);
        deck.bus_mut().write(0x2006, 0x20);
        deck.bus_mut().write(0x2006, 0x00);
        deck.cpu_mut().status.insert(Status::C);
        let cycles = deck.clock_instr().unwrap();
        assert_eq!(cycles, 6, "opcode {opcode:02X}");
        assert_eq!(deck.cpu().pc, 0x8003);
        assert_eq!(
            deck.ppu().bus.peek(0x2000),
            0x34,
            "read must not write opcode {opcode:02X}"
        );
        assert_eq!(
            deck.ppu().bus.peek(0x2001),
            0x12,
            "old write opcode {opcode:02X}"
        );
        assert_eq!(
            deck.ppu().bus.peek(0x2002),
            result,
            "new write opcode {opcode:02X}"
        );
    }
}

#[test]
fn dmc_dma_fetches_cartridge_byte_and_stalls_both_cpu_alignments() {
    let mut stalls = Vec::new();
    for padding in [vec![0xea], vec![0x24, 0x00]] {
        let mut code = padding;
        code.extend_from_slice(&[
            0x78, // SEI: observe DMA independently of IRQ entry.
            0xa9, 0x0f, 0x8d, 0x10, 0x40, // Fastest DMC rate, no IRQ or loop.
            0xa9, 0x10, 0x8d, 0x12, 0x40, // Sample $C400.
            0xa9, 0x01, 0x8d, 0x13, 0x40, // Seventeen bytes: load plus reload DMA.
            0xa9, 0x10, 0x8d, 0x15, 0x40, // Start DMC through CPU write.
        ]);
        let start = 0x8000 + code.len() as u16;
        code.extend_from_slice(&[0xea, 0x4c, start as u8, (start >> 8) as u8]);
        let mut deck = program(&code);
        while deck.cpu().pc < start {
            deck.clock_instr().unwrap();
        }
        let mut fetched = 0;
        for _ in 0..10_000 {
            let before = deck.cpu().pc;
            let remaining = deck.apu().dmc.bytes_remaining;
            let cycles = deck.clock_instr().unwrap();
            let base_cycles = if before == start { 2 } else { 3 };
            let next_pc = if before == start { start + 1 } else { start };
            assert_eq!(
                deck.cpu().pc,
                next_pc,
                "DMA must resume the pending instruction"
            );
            let extra = cycles - base_cycles;
            if deck.apu().dmc.bytes_remaining < remaining {
                assert_eq!(deck.apu().dmc.bytes_remaining, remaining - 1);
                assert!((3..=4).contains(&extra), "DMC stall {extra}");
                if fetched == 0 {
                    assert_eq!(deck.apu().dmc.sample_buffer, 0xa5);
                    // Initial enable adjusts the request delay for write parity.
                    assert_eq!(extra, 3, "initial load DMA");
                }
                stalls.push(extra);
                fetched += 1;
            } else {
                assert_eq!(extra, 0, "ordinary loop instruction");
            }
            if deck.apu().dmc.bytes_remaining == 0 {
                break;
            }
        }
        assert_eq!(fetched, 17);
        assert_eq!(deck.apu().dmc.addr, 0xc411);
        assert_eq!(deck.apu().dmc.bytes_remaining, 0);
        assert!(!Cpu::has_irq(Irq::DMC));
    }
    stalls.sort_unstable();
    stalls.dedup();
    assert_eq!(stalls, [3, 4], "halt/dummy/alignment/get cycles");
}

#[test]
fn dmc_irq_is_masked_and_acknowledging_one_source_preserves_the_other() {
    let mut deck = program(&[
        0x78, 0xa9, 0x8f, 0x8d, 0x10, 0x40, // DMC IRQ enabled while I is set.
        0xa9, 0x10, 0x8d, 0x12, 0x40, 0xa9, 0x00, 0x8d, 0x13, 0x40, 0xa9, 0x10, 0x8d, 0x15, 0x40,
        0x4c, 0x15, 0x80, // JMP to self with IRQ still masked.
    ]);
    for _ in 0..20_000 {
        assert_ne!(deck.cpu().pc, 0x8100);
        if Cpu::has_irq(Irq::DMC) && Cpu::has_irq(Irq::FRAME_COUNTER) {
            break;
        }
        deck.clock_instr().unwrap();
    }
    assert!(Cpu::has_irq(Irq::DMC));
    assert!(Cpu::has_irq(Irq::FRAME_COUNTER));
    assert_eq!(deck.cpu().bus.peek(0x4015) & 0xc0, 0xc0);
    // A real APU status read acknowledges only the frame sequencer IRQ.
    assert_eq!(deck.bus_mut().read(0x4015) & 0xc0, 0xc0);
    assert!(Cpu::has_irq(Irq::DMC));
    assert!(!Cpu::has_irq(Irq::FRAME_COUNTER));
    for _ in 0..20_000 {
        if Cpu::has_irq(Irq::FRAME_COUNTER) {
            break;
        }
        deck.clock_instr().unwrap();
    }
    assert!(Cpu::has_irq(Irq::FRAME_COUNTER));
    // $4015 acknowledges DMC without clearing a different device's held IRQ.
    deck.bus_mut().write(0x4015, 0);
    assert!(!Cpu::has_irq(Irq::DMC));
    assert!(Cpu::has_irq(Irq::FRAME_COUNTER));
    deck.bus_mut().write(0x4017, 0x40);
    assert!(!Cpu::has_irq(Irq::FRAME_COUNTER));
}

#[test]
fn ppu_nmi_wins_when_rti_unmasks_a_held_apu_irq() {
    let mut deck = program(&[
        0xa9, 0x00, 0x8d, 0x17, 0x40, // Four-step frame IRQ enabled.
        0x58, 0x4c, 0x06, 0x80, // CLI then self-loop.
    ]);
    // Wait for actual APU IRQ delivery with RTI about to execute during vblank.
    // No direct CPU IRQ/NMI event injection is used: enabling PPUCTRL during
    // the PPU's real vblank raises NMI while the APU's IRQ remains asserted.
    for _ in 0..40_000 {
        if deck.cpu().pc == 0x8100
            && deck.ppu().status.in_vblank
            && Cpu::has_irq(Irq::FRAME_COUNTER)
        {
            break;
        }
        deck.clock_instr().unwrap();
    }
    assert_eq!(deck.cpu().pc, 0x8100);
    assert!(deck.ppu().status.in_vblank);
    deck.bus_mut().write(0x2000, 0x80);
    let cycles = deck.clock_instr().unwrap();
    assert_eq!(deck.cpu().pc, 0x8200, "NMI vector must beat IRQ after RTI");
    assert_eq!(cycles, 13, "RTI plus hardware NMI entry");
    assert!(
        Cpu::has_irq(Irq::FRAME_COUNTER),
        "NMI must not acknowledge APU IRQ"
    );
    assert_eq!(deck.cpu().bus.peek(0x1fd), 0x80);
    assert_eq!(deck.cpu().bus.peek(0x1fc), 0x06);
    assert_eq!(deck.cpu().bus.peek(0x1fb) & 0x34, 0x20);
    assert_eq!(deck.clock_instr().unwrap(), 13);
    assert_eq!(deck.cpu().pc, 0x8100, "held IRQ resumes after NMI RTI");
}

fn schedule_vblank_edge(deck: &mut ControlDeck, preceding_cpu_cycles: u32) {
    // Cycle-scheduling seam: position the PPU just before real vblank rather than
    // injecting Cpu::nmi. Its normal per-dot clock raises the edge while the CPU
    // performs the interrupt's bus/stack cycles. NTSC advances three PPU dots/cycle.
    let master_clock = deck.cpu().master_clock;
    let ppu = deck.ppu_mut();
    ppu.emulate_warmup = false;
    ppu.status.set_in_vblank(false);
    ppu.scanline = 240;
    ppu.cycle = 340 - preceding_cpu_cycles * 3;
    ppu.master_clock = master_clock - 4;
    deck.bus_mut().write(0x2000, 0x80);
}

#[test]
fn scheduled_ppu_nmi_hijacks_brk_only_before_vector_selection() {
    for preceding_cycles in 0..=6 {
        let mut deck = program(&[0x00, 0xea, 0xea]);
        schedule_vblank_edge(&mut deck, preceding_cycles);
        assert_eq!(deck.clock_instr().unwrap(), 7);
        assert_eq!(
            deck.cpu().pc,
            if preceding_cycles <= 3 {
                0x8200
            } else {
                0x8100
            },
            "edge after {preceding_cycles} cycles"
        );
        assert_eq!(deck.cpu().bus.peek(0x1fd), 0x80);
        assert_eq!(
            deck.cpu().bus.peek(0x1fc),
            0x02,
            "BRK retains its two-byte return convention"
        );
        assert_eq!(
            deck.cpu().bus.peek(0x1fb) & 0x30,
            0x30,
            "hijack must retain BRK's B bit"
        );
        if preceding_cycles > 3 {
            assert_eq!(deck.clock_instr().unwrap(), 13);
            assert_eq!(
                deck.cpu().pc,
                0x8200,
                "late edge remains pending after IRQ-vector RTI"
            );
        }
    }
}

#[test]
fn scheduled_ppu_nmi_hijacks_actual_apu_irq_entry() {
    for irq_cycles in 0..=5 {
        let mut deck = program(&[0x78, 0x4c, 1, 0x80, 0x58, 0x4c, 5, 0x80]);
        for _ in 0..20_000 {
            if Cpu::has_irq(Irq::FRAME_COUNTER) {
                break;
            }
            deck.clock_instr().unwrap();
        }
        assert!(Cpu::has_irq(Irq::FRAME_COUNTER));
        // Fixture jumps from the masked wait loop to an actual CLI instruction.
        // The held request was raised by the APU, not a CPU IRQ injection.
        deck.cpu_mut().pc = 0x8004;
        assert_eq!(deck.clock_instr().unwrap(), 2);
        schedule_vblank_edge(&mut deck, 3 + irq_cycles); // JMP's three cycles precede IRQ.
        assert_eq!(deck.clock_instr().unwrap(), 10);
        assert_eq!(
            deck.cpu().pc,
            if irq_cycles <= 3 { 0x8200 } else { 0x8100 },
            "IRQ edge after {irq_cycles} entry cycles"
        );
        assert_eq!(deck.cpu().bus.peek(0x1fd), 0x80);
        assert_eq!(deck.cpu().bus.peek(0x1fc), 0x05);
        assert_eq!(
            deck.cpu().bus.peek(0x1fb) & 0x34,
            0x20,
            "hardware status has B=0 and saved I=0"
        );
        assert!(Cpu::has_irq(Irq::FRAME_COUNTER));
    }
}

#[test]
fn resets_clear_pending_dma_irq_and_nmi_before_next_instruction() {
    use tetanes_core::common::{Reset, ResetKind};
    for reset in [ResetKind::Soft, ResetKind::Hard] {
        let mut deck = program(&[
            0x78, 0xa9, 0x8f, 0x8d, 0x10, 0x40, 0xa9, 0x10, 0x8d, 0x12, 0x40, 0xa9, 0x00, 0x8d,
            0x13, 0x40, 0xa9, 0x10, 0x8d, 0x15, 0x40, 0x4c, 0x15, 0x80,
        ]);
        for _ in 0..20_000 {
            if Cpu::has_irq(Irq::DMC) && Cpu::has_irq(Irq::FRAME_COUNTER) {
                break;
            }
            deck.clock_instr().unwrap();
        }
        assert!(Cpu::has_irq(Irq::DMC) && Cpu::has_irq(Irq::FRAME_COUNTER));
        deck.ppu_mut().emulate_warmup = false;
        deck.ppu_mut().status.set_in_vblank(true);
        deck.bus_mut().write(0x2000, 0x80);
        deck.bus_mut().write(0x4014, 2);
        assert!(Cpu::nmi_pending());
        assert_eq!(Cpu::dmas_running(), Some((false, true)));
        let sp = deck.cpu().sp;
        deck.reset(reset);
        assert!(Cpu::irqs().is_empty());
        assert!(!Cpu::nmi_pending());
        assert_eq!(Cpu::dmas_running(), None);
        assert!(!Cpu::halt_for_dma());
        assert!(!Cpu::dma_dummy_read());
        assert_eq!(deck.apu().dmc.bytes_remaining, 0);
        assert_eq!(deck.cpu().pc, 0x8000);
        assert_eq!(
            deck.cpu().sp,
            if matches!(reset, ResetKind::Hard) {
                0xfd
            } else {
                sp.wrapping_sub(3)
            }
        );
        assert_ne!(deck.cpu().status.bits() & 4, 0);
        assert_eq!(deck.cpu().cycle, 7);
        assert_eq!(
            deck.clock_instr().unwrap(),
            2,
            "reset must not retain a DMA stall or interrupt"
        );
    }
    let mut fresh = program(&[0xea, 0xea]);
    assert!(Cpu::irqs().is_empty());
    assert_eq!(Cpu::dmas_running(), None);
    assert_eq!(fresh.clock_instr().unwrap(), 2);
}

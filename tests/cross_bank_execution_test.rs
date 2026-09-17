use swiss_emulator::Emulator;
use swissarmynes::server::api::compile_source;

#[test]
fn main_in_switchable_bank_and_interrupt_in_fixed_bank_execute() {
    let source = "DIM ready AS BYTE\nDIM ticks AS BYTE\nBANK 3\nSUB Main()\nLET ready = 42\nPOKE($2000, $80)\nEND SUB\nINTERRUPT NMI()\nLET ticks = ticks + 1\nEND INTERRUPT";
    let (rom, _) = compile_source(Some(source.into()), None, None).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();
    for _ in 0..10 {
        emu.step().expect("Execution failed");
    }
    let ram = emu.ram_snapshot();
    assert_eq!(ram[0x5c0], 42);
    assert!(ram[0x5c1] > 0, "NMI handler should execute after startup");
    assert!(u16::from_le_bytes([ram[0x7f4], ram[0x7f5]]) >= 0xc000);
}

#[test]
fn test_vector_layout_in_bank7() {
    let source = r#"
        SUB Main()
        END SUB
    "#;

    let (rom_bytes, _) = compile_source(Some(source.to_string()), None, None).unwrap();
    let prg_end = 16 + 131072;
    let vectors = &rom_bytes[prg_end - 6..prg_end];

    let vec_nmi = (vectors[0] as u16) | ((vectors[1] as u16) << 8);
    let vec_reset = (vectors[2] as u16) | ((vectors[3] as u16) << 8);
    let vec_irq = (vectors[4] as u16) | ((vectors[5] as u16) << 8);

    assert!(
        vec_reset >= 0xC000,
        "VecReset (${:04X}) must point to Bank 7 ($C000+)",
        vec_reset
    );
    assert!(
        vec_nmi >= 0xC000,
        "VecNMI (${:04X}) must point to Bank 7 ($C000+)",
        vec_nmi
    );
    assert!(
        vec_irq >= 0xC000,
        "VecIRQ (${:04X}) must point to Bank 7 ($C000+)",
        vec_irq
    );
}

#[test]
fn test_cross_bank_nested_calls_execution() {
    let source = r#"
        DIM flag0 AS BYTE
        DIM flag3 AS BYTE
        DIM flag5 AS BYTE
        DIM p AS WORD

        BANK 3
        SUB Padding()
            LET p = 1234
            LET p = p + 1
        END SUB

        SUB Foo()
            LET flag3 = 33
            Bar()
        END SUB

        BANK 5
        SUB Bar()
            LET flag5 = 55
        END SUB

        BANK 0
        SUB Main()
            Foo()
            LET flag0 = 11
        END SUB
    "#;

    let (rom_bytes, _) = compile_source(Some(source.to_string()), None, None).unwrap();
    let mut emu = Emulator::new();
    emu.load_rom(&rom_bytes).unwrap();

    for _ in 0..30 {
        emu.step().expect("Emulator execution failed");
    }

    let wram = emu.ram_snapshot();

    assert_eq!(wram[0x05C0], 11, "flag0 in Bank 0 should be 11");
    assert_eq!(wram[0x05C1], 33, "flag3 in Bank 3 should be 33");
    assert_eq!(wram[0x05C2], 55, "flag5 in Bank 5 should be 55");

    // Reserved PRG bank location $07F0 must equal 0 (returned to caller Bank 0)
    assert_eq!(
        wram[0x07F0], 0,
        "current_prg_bank at $07F0 should return to 0"
    );
}

#[test]
fn interrupt_preserves_cross_bank_return_scratch() {
    let source = r#"
        DIM observed AS BYTE
        DIM ticks AS BYTE
        SUB Main()
            POKE($07F1, 93)
            POKE($2000, $80)
            WHILE ticks = 0
            WEND
            LET observed = PEEK($07F1)
        END SUB
        INTERRUPT NMI()
            POKE($07F1, 17)
            LET ticks = ticks + 1
        END INTERRUPT
    "#;
    let (rom, _) = compile_source(Some(source.into()), None, None).unwrap();
    let mut emulator = Emulator::new();
    emulator.load_rom(&rom).unwrap();
    for _ in 0..10 {
        emulator.step().unwrap();
    }
    assert_eq!(emulator.peek_cpu(0x5c0), 93);
    assert!(emulator.peek_cpu(0x5c1) > 0);
}

#[test]
fn irq_preserves_cross_bank_return_scratch() {
    let source = r#"
        DIM observed AS BYTE
        DIM ticks AS BYTE
        SUB Main()
            POKE($07F1, 93)
            POKE($4017, 0)
            ASM
                CLI
            END ASM
            WHILE ticks = 0
            WEND
            LET observed = PEEK($07F1)
        END SUB
        INTERRUPT IRQ()
            POKE($07F1, 17)
            POKE($4017, $40)
            LET ticks = ticks + 1
        END INTERRUPT
    "#;
    let (rom, _) = compile_source(Some(source.into()), None, None).unwrap();
    let mut emulator = Emulator::new();
    emulator.load_rom(&rom).unwrap();
    for _ in 0..10 {
        emulator.step().unwrap();
    }
    assert_eq!(emulator.peek_cpu(0x5c0), 93);
    assert!(emulator.peek_cpu(0x5c1) > 0);
}

#[test]
fn interrupts_during_each_mmc1_write_boundary_restore_mapping() {
    use tetanes_core::{control_deck::ControlDeck, mem::Read};
    let source = r#"
        DIM done AS BYTE
        DIM ticks AS BYTE
        DIM called AS BYTE
        BANK 3
        SUB Work()
            LET called = 33
        END SUB
        BANK 5
        SUB InterruptWork()
            LET ticks = ticks + 1
        END SUB
        BANK 0
        SUB Main()
            Work()
            LET done = 42
        END SUB
        INTERRUPT NMI()
            InterruptWork()
        END INTERRUPT
    "#;
    for interrupt in ["NMI", "IRQ"] {
        let source = source.replace("INTERRUPT NMI()", &format!("INTERRUPT {interrupt}()"));
        let (rom, _) = compile_source(Some(source), None, None).unwrap();
        // Resolve the setter through the generated bank-3 call trampoline, without
        // depending on its changing position within fixed-bank runtime code.
        let fixed = &rom[16 + 7 * 16384..16 + 8 * 16384];
        let call = fixed
            .windows(6)
            .position(|w| w[0..3] == [0xa9, 3, 0x20] && w[5] == 0x20)
            .unwrap();
        let setter = u16::from_le_bytes([fixed[call + 3], fixed[call + 4]]);
        let at_setter = || {
            let mut deck = ControlDeck::new();
            deck.load_rom("interrupt.nes", &mut &rom[..]).unwrap();
            for _ in 0..200000 {
                if deck.cpu().pc == setter && deck.cpu().acc == 3 {
                    return deck;
                }
                deck.clock_instr().unwrap();
            }
            panic!("never entered target bank setter");
        };
        let mut baseline = at_setter();
        let mut boundaries = 0;
        while baseline.cpu().bus.peek(baseline.cpu().pc) != 0x60 {
            baseline.clock_instr().unwrap();
            boundaries += 1;
            assert!(boundaries < 64);
        }
        for boundary in 0..=boundaries {
            let mut deck = at_setter();
            for _ in 0..boundary {
                deck.clock_instr().unwrap();
            }
            // Inject only the arrival event. The production CPU performs interrupt
            // entry, cartridge vector reads, handler execution and RTI normally.
            deck.cpu_mut().nmi = interrupt == "NMI";
            deck.cpu_mut().irq();
            for _ in 0..5000 {
                if deck.cpu().bus.peek(0x5c0) == 42 {
                    break;
                }
                deck.clock_instr().unwrap();
            }
            assert_eq!(deck.cpu().bus.peek(0x5c0), 42, "boundary {boundary}");
            assert_eq!(deck.cpu().bus.peek(0x5c1), 1, "boundary {boundary}");
            assert_eq!(deck.cpu().bus.peek(0x5c2), 33, "boundary {boundary}");
            assert_eq!(deck.cpu().bus.peek(0x7f0), 0, "boundary {boundary}");
        }
    }
}

#[test]
fn dynamic_interrupt_binding_rebinds_to_banked_routines() {
    for (event, enable) in [
        ("NMI", "POKE($2000, $80)"),
        ("IRQ", "POKE($4017, 0)\nASM\nCLI\nEND ASM"),
    ] {
        let source = format!(
            r#"
            DIM ticks AS BYTE
            DIM done AS BYTE
            BANK 3
            SUB First()
                LET ticks = 1
                ON {event} DO Second
            END SUB
            BANK 5
            SUB Second()
                POKE($4017, $40)
                LET ticks = 2
            END SUB
            BANK 0
            SUB Main()
                ON {event} DO First
                {enable}
                WHILE ticks < 2
                WEND
                LET done = 42
            END SUB
        "#
        );
        let (rom, _) = compile_source(Some(source), None, None).unwrap();
        let mut emulator = Emulator::new();
        emulator.load_rom(&rom).unwrap();
        for _ in 0..10 {
            emulator.step().unwrap();
        }
        assert_eq!(emulator.peek_cpu(0x5c0), 2, "{event}");
        assert_eq!(emulator.peek_cpu(0x5c1), 42, "{event}");
        assert_eq!(emulator.peek_cpu(0x7f0), 0, "{event}");
    }
}

#[test]
fn invalid_dynamic_interrupt_handlers_fail_compilation() {
    for source in [
        "SUB Main()\nON NMI DO Missing\nEND SUB",
        "DIM value AS BYTE\nSUB Main()\nON IRQ DO value\nEND SUB",
        "SUB Handler(value AS BYTE)\nEND SUB\nSUB Main()\nON NMI DO Handler\nEND SUB",
    ] {
        let error = compile_source(Some(source.into()), None, None).unwrap_err();
        assert!(error.contains("zero-argument routine"), "{error}");
    }
}

#[test]
fn dynamic_binding_publication_is_atomic_at_each_instruction_boundary() {
    use tetanes_core::{control_deck::ControlDeck, mem::Read};
    let source = "DIM selected AS BYTE\nDIM done AS BYTE\nBANK 3\nSUB First()\nLET selected = 1\nEND SUB\nBANK 5\nSUB Second()\nLET selected = 2\nEND SUB\nBANK 0\nSUB Main()\nON NMI DO First\nON NMI DO Second\nLET done = 42\nEND SUB";
    let (rom, map) = compile_source(Some(source.into()), None, None).unwrap();
    let binding = map.entries.iter().find(|entry| entry.line == 14).unwrap();
    assert_eq!(rom[binding.rom_offset], 0xa9);
    assert_eq!(
        &rom[binding.rom_offset + 2..binding.rom_offset + 5],
        &[0x8d, 0xf8, 7]
    );
    for boundary in 0..=2 {
        let mut deck = ControlDeck::new();
        deck.load_rom("atomic-binding.nes", &mut &rom[..]).unwrap();
        let mut reached = false;
        for _ in 0..200000 {
            if deck.cpu().pc == binding.cpu_start {
                reached = true;
                break;
            }
            deck.clock_instr().unwrap();
        }
        assert!(reached);
        for _ in 0..boundary {
            deck.clock_instr().unwrap();
        }
        deck.cpu_mut().nmi = true;
        deck.cpu_mut().irq();
        for _ in 0..5000 {
            if deck.cpu().bus.peek(0x5c1) == 42 {
                break;
            }
            deck.clock_instr().unwrap();
        }
        assert_eq!(deck.cpu().bus.peek(0x5c1), 42);
        assert_eq!(
            deck.cpu().bus.peek(0x5c0),
            if boundary < 2 { 1 } else { 2 },
            "boundary {boundary}"
        );
    }
}

#[test]
fn interrupts_preserve_helper_arguments_and_audio_temporaries() {
    let scratch: Vec<u8> = (0x14..=0x17).chain(0xf0..=0xf7).collect();
    for (event, enable) in [
        ("NMI", "LDA #$80\nSTA $2000"),
        ("IRQ", "LDA #0\nSTA $4017\nCLI"),
    ] {
        let initialize = scratch
            .iter()
            .enumerate()
            .map(|(index, address)| format!("LDA #{}\nSTA ${address:02X}", 80 + index))
            .collect::<Vec<_>>()
            .join("\n");
        let record = scratch
            .iter()
            .enumerate()
            .map(|(index, address)| format!("LDA ${address:02X}\nSTA ${:04X}", 0x600 + index))
            .collect::<Vec<_>>()
            .join("\n");
        let clobber = scratch
            .iter()
            .map(|address| format!("STA ${address:02X}"))
            .collect::<Vec<_>>()
            .join("\n");
        let source = format!(
            r#"
            DIM ticks AS BYTE
            SUB Main()
                ASM
                    {initialize}
                    {enable}
                WaitForScratchHandler:
                    LDA $05C0
                    BEQ WaitForScratchHandler
                    {record}
                END ASM
            END SUB
            INTERRUPT {event}()
                ASM
                    LDA #17
                    {clobber}
                END ASM
                POKE($4017, $40)
                POKE($18, 99)
                LET ticks = 1
            END INTERRUPT
        "#
        );
        let (rom, _) = compile_source(Some(source), None, None).unwrap();
        let mut emulator = Emulator::new();
        emulator.load_rom(&rom).unwrap();
        for _ in 0..10 {
            emulator.step().unwrap();
        }
        for (index, address) in scratch.iter().enumerate() {
            assert_eq!(
                emulator.peek_cpu(0x600 + index as u16),
                80 + index as u8,
                "{event} scratch {address:02X}"
            );
        }
        // Text offset is persistent application state, not temporary scratch.
        assert_eq!(emulator.peek_cpu(0x18), 99);
    }
}

#[test]
fn idle_nmi_with_dma_fits_ntsc_vblank_budget() {
    use tetanes_core::{control_deck::ControlDeck, mem::Read};
    let (rom, _) = compile_source(
        Some("DIM ready AS BYTE\nSUB Main()\nLET ready = 42\nEND SUB".into()),
        None,
        None,
    )
    .unwrap();
    let mut deck = ControlDeck::new();
    deck.load_rom("nmi-budget.nes", &mut &rom[..]).unwrap();
    for _ in 0..200000 {
        if deck.cpu().bus.peek(0x5c0) == 42 {
            break;
        }
        deck.clock_instr().unwrap();
    }
    assert_eq!(deck.cpu().bus.peek(0x5c0), 42);
    for _ in 0..2 {
        let pc = deck.cpu().pc;
        let sp = deck.cpu().sp;
        let before = deck.cpu().cycle;
        deck.cpu_mut().nmi = true;
        deck.cpu_mut().irq();
        for _ in 0..5000 {
            if deck.cpu().pc == pc && deck.cpu().sp == sp {
                break;
            }
            deck.clock_instr().unwrap();
        }
        assert_eq!((deck.cpu().pc, deck.cpu().sp), (pc, sp));
        let cycles = deck.cpu().cycle - before;
        // About 20 NTSC scanlines are available; active audio, buffered VRAM
        // transfers and user code need a separate budget beyond this idle case.
        assert!(cycles < 2273, "idle NMI used {cycles} cycles");
        println!("idle NMI including entry and DMA: {cycles} CPU cycles");
        deck.clock_instr().unwrap();
    }
}

#[test]
fn sound_update_visits_active_channels_once_and_leaves_inactive_state_alone() {
    use tetanes_core::{
        control_deck::ControlDeck,
        mem::{Read, Write},
    };
    let (rom, _) = compile_source(
        Some("DIM ready AS BYTE\nSUB Main()\nLET ready = 42\nEND SUB".into()),
        None,
        None,
    )
    .unwrap();
    for active_mask in [0b0101, 0b1010, 0b1111] {
        let mut deck = ControlDeck::new();
        deck.load_rom("channel-update.nes", &mut &rom[..]).unwrap();
        for _ in 0..200000 {
            if deck.cpu().bus.peek(0x5c0) == 42 {
                break;
            }
            deck.clock_instr().unwrap();
        }
        assert_eq!(deck.cpu().bus.peek(0x5c0), 42);
        for channel in 0..4u16 {
            let base = 0x300 + channel * 0x20;
            if active_mask & (1 << channel) != 0 {
                deck.cpu_mut().bus.write(base, 1);
                deck.cpu_mut().bus.write(base + 1, 10);
                for offset in [6, 9, 0x0d, 0x13] {
                    deck.cpu_mut().bus.write(base + offset, 0xff);
                }
            }
        }
        let pc = deck.cpu().pc;
        let sp = deck.cpu().sp;
        deck.cpu_mut().nmi = true;
        deck.cpu_mut().irq();
        for _ in 0..2000 {
            if deck.cpu().pc == pc && deck.cpu().sp == sp {
                break;
            }
            deck.clock_instr().unwrap();
        }
        assert_eq!((deck.cpu().pc, deck.cpu().sp), (pc, sp));
        for channel in 0..4u16 {
            let base = 0x300 + channel * 0x20;
            if active_mask & (1 << channel) != 0 {
                assert_eq!(deck.cpu().bus.peek(base + 1), 9, "channel {channel}");
            } else {
                for offset in 0..32 {
                    assert_eq!(
                        deck.cpu().bus.peek(base + offset),
                        0,
                        "inactive channel {channel}, offset {offset}"
                    );
                }
            }
        }
    }
}

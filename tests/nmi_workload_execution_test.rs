use swissarmynes::server::{api::compile_source, project::*};
use tetanes_core::{control_deck::ControlDeck, mem::Read};

fn active_audio_assets() -> ProjectAssets {
    let mut assets: ProjectAssets = serde_json::from_value(serde_json::json!({
        "chr_bank": [], "palettes": [], "nametables": []
    }))
    .unwrap();
    assets.sound_effects = (0..3)
        .map(|channel| SoundEffect {
            name: format!("Loop{channel}"),
            channel,
            priority: 10,
            speed: 1,
            vol_sequence: vec![15],
            pitch_sequence: vec![0],
            duty_sequence: vec![0],
            arpeggio_sequence: vec![0],
            does_loop: true,
        })
        .collect();
    assets.samples.push(DpcmSample {
        name: "Sample".into(),
        data: vec![0xaa; 17],
    });
    assets.audio_tracks.push(AudioTrack {
        name: "DMC".into(),
        channel: 3,
        instrument: 15,
        priority: 0,
        vol_env: None,
        pitch_env: None,
        arpeggio_env: None,
        notes: vec![AudioNote {
            pitch: 0,
            col: 0,
            row: 0,
            duration: 255,
        }],
    });
    assets
}

#[test]
fn active_audio_upload_ppu_writes_fit_vblank_and_work_fits_frame() {
    for (command, count) in [("LoadRow", 32), ("LoadColumn", 30)] {
        let source = format!(
            r#"
            DIM ready AS BYTE
            DIM tiles({count}) AS BYTE
            DIM attrs(8) AS BYTE
            SUB Main()
                PPU.Ctrl(0)
                Sound.Play(0)
                PLAY_SFX(0)
                PLAY_SFX(1)
                PLAY_SFX(2)
                Scroll.{command}(0, tiles, attrs)
                ready = 42
            END SUB
            "#,
        );
        let (rom, _) = compile_source(Some(source), None, Some(active_audio_assets())).unwrap();
        let mut deck = ControlDeck::new();
        deck.load_rom("nmi-workload.nes", &mut &rom[..]).unwrap();
        for _ in 0..200_000 {
            if deck.cpu().bus.peek(0x05c0) == 42 {
                break;
            }
            deck.clock_instr().unwrap();
        }
        assert_eq!(deck.cpu().bus.peek(0x05c0), 42);
        for channel in 0..3 {
            assert_eq!(deck.cpu().bus.peek(0x0300 + channel * 32), 2);
        }
        assert_eq!(deck.cpu().bus.peek(0x0360), 1);
        // Complete Main's return before injecting a hardware interrupt.
        for _ in 0..8 {
            deck.clock_instr().unwrap();
        }
        for phase in [2, 0] {
            let pc = deck.cpu().pc;
            let sp = deck.cpu().sp;
            let before = deck.cpu().cycle;
            let mut last_ppu_write = 0;
            deck.cpu_mut().nmi = true;
            deck.cpu_mut().irq();
            for _ in 0..10_000 {
                if deck.cpu().pc == pc && deck.cpu().sp == sp {
                    break;
                }
                let instruction = deck.cpu().pc;
                let is_ppu_write = deck.cpu().bus.peek(instruction) == 0x8d
                    && deck.cpu().bus.peek(instruction.wrapping_add(2)) == 0x20
                    && deck.cpu().bus.peek(instruction.wrapping_add(1)) <= 7;
                deck.clock_instr().unwrap();
                if is_ppu_write {
                    last_ppu_write = deck.cpu().cycle - before;
                }
            }
            assert_eq!((deck.cpu().pc, deck.cpu().sp), (pc, sp));
            assert_eq!(deck.cpu().bus.peek(0x0380), phase);
            let cycles = deck.cpu().cycle - before;
            println!(
                "{command} phase->{phase}: NMI {cycles} cycles, last PPU write {last_ppu_write}"
            );
            assert!(
                last_ppu_write > 0 && last_ppu_write < 2273,
                "{command} PPU writes exceed VBlank: {last_ppu_write} cycles"
            );
            // APU updates can continue into visible scanlines. The full workload
            // must finish within the frame; only PPU writes require VBlank.
            assert!(
                cycles < 29_780,
                "{command} NMI exceeds frame budget: {cycles} cycles"
            );
            deck.clock_instr().unwrap();
        }
    }
}

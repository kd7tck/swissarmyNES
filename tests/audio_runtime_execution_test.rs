use swissarmynes::compiler::audio::generate_period_table;
use swissarmynes::server::{api::compile_source, project::*};
use tetanes_core::{control_deck::ControlDeck, mem::Read};

fn assets() -> ProjectAssets {
    serde_json::from_value(serde_json::json!({
        "chr_bank": [], "palettes": [], "nametables": []
    }))
    .unwrap()
}

fn track(channel: u8, pitch: u8, priority: u8) -> AudioTrack {
    AudioTrack {
        name: "Music".into(),
        channel,
        instrument: 0x3f,
        priority,
        vol_env: None,
        pitch_env: None,
        arpeggio_env: None,
        notes: vec![AudioNote {
            pitch,
            row: 0,
            col: 0,
            duration: 32,
        }],
    }
}

fn effect(priority: u8) -> SoundEffect {
    SoundEffect {
        name: "Effect".into(),
        channel: 0,
        priority,
        speed: 1,
        vol_sequence: vec![15, 8],
        pitch_sequence: vec![0, -2],
        duty_sequence: vec![1, 2],
        arpeggio_sequence: vec![0, 7],
        does_loop: false,
    }
}

fn deck(body: &str, assets: ProjectAssets) -> ControlDeck {
    // Drive the real generated helper explicitly for deterministic frame boundaries.
    // The ROM still uses the production parser, compiler, linker and backend CPU/APU.
    let source = format!("DIM checkpoint AS BYTE\nSUB Main()\nPOKE($2000, 0)\n{body}\nEND SUB");
    let (rom, _) = compile_source(Some(source), None, Some(assets)).unwrap();
    let mut deck = ControlDeck::new();
    deck.load_rom("audio-runtime.nes", &mut &rom[..]).unwrap();
    deck
}

fn until(deck: &mut ControlDeck, checkpoint: u8) {
    for _ in 0..200_000 {
        if deck.cpu().bus.peek(0x5c0) == checkpoint {
            return;
        }
        deck.clock_instr().unwrap();
    }
    panic!("audio checkpoint {checkpoint} was never reached");
}

fn period(note: usize) -> u64 {
    let table = generate_period_table();
    u16::from_le_bytes([table[note * 2], table[note * 2 + 1]]) as u64
}

#[test]
fn sound_stop_silences_existing_hardware_and_clears_channels() {
    let mut deck = deck(
        r#"
        POKE($4015, $0F)
        POKE($4000, $3F)
        POKE($4002, $6A)
        POKE($4003, 0)
        POKE($4004, $3F)
        POKE($4007, 0)
        POKE($4008, $FF)
        POKE($400B, 0)
        POKE($400C, $3F)
        POKE($400F, 0)
        POKE($4011, 64)
        POKE($0300, 1)
        POKE($5C0, 1)
        Sound.Stop()
        POKE($5C0, 2)
    "#,
        assets(),
    );
    until(&mut deck, 1);
    assert_eq!(deck.apu().pulse1.envelope.volume, 15);
    assert_eq!(deck.apu().dmc.output_level, 64);
    until(&mut deck, 2);
    assert_eq!(deck.apu().pulse1.envelope.volume, 0);
    assert_eq!(deck.apu().pulse2.envelope.volume, 0);
    assert_eq!(deck.apu().noise.envelope.volume, 0);
    assert_eq!(deck.apu().pulse1.length.counter, 0);
    assert_eq!(deck.apu().triangle.length.counter, 0);
    assert_eq!(deck.apu().dmc.output_level, 0);
    assert_eq!(deck.apu().dmc.bytes_remaining, 0);
    assert!((0x300..0x380).all(|address| deck.cpu().bus.peek(address) == 0));
}

#[test]
fn effects_obey_priority_exact_duration_and_resume_music() {
    let mut assets = assets();
    assets.audio_tracks.push(track(0, 36, 5));
    assets.sound_effects = vec![effect(4), effect(9)];
    let mut deck = deck(
        r#"
        Sound.Play(0)
        ASM
            JSR Sound_Update
        END ASM
        POKE($5C0, 1)
        PLAY_SFX(0)
        POKE($5C0, 2)
        PLAY_SFX(1)
        ASM
            JSR Sound_Update
        END ASM
        POKE($5C0, 3)
        ASM
            JSR Sound_Update
        END ASM
        POKE($5C0, 4)
        PLAY_SFX(1)
        ASM
            JSR Sound_Update
        END ASM
        POKE($5C0, 5)
        ASM
            JSR Sound_Update
            JSR Sound_Update
        END ASM
        POKE($5C0, 6)
    "#,
        assets,
    );
    until(&mut deck, 1);
    assert_eq!(deck.apu().pulse1.real_period, period(36));
    until(&mut deck, 2);
    assert_eq!(deck.cpu().bus.peek(0x305), 5, "lower priority is rejected");
    assert_eq!(deck.cpu().bus.peek(0x300), 1);
    until(&mut deck, 3);
    assert_eq!(deck.cpu().bus.peek(0x300), 2);
    assert_eq!(deck.apu().pulse1.real_period, period(48));
    assert_eq!(deck.apu().pulse1.duty, 1);
    until(&mut deck, 4);
    assert_eq!(deck.apu().pulse1.real_period, period(55) - 2);
    assert_eq!(deck.apu().pulse1.envelope.volume, 8);
    assert_eq!(deck.apu().pulse1.duty, 2);
    until(&mut deck, 5);
    assert_eq!(
        deck.apu().pulse1.real_period,
        period(48),
        "equal priority retriggers"
    );
    until(&mut deck, 6);
    assert_eq!(deck.cpu().bus.peek(0x300), 1);
    assert_eq!(deck.cpu().bus.peek(0x305), 5);
    assert_eq!(
        deck.cpu().bus.peek(0x301),
        32,
        "music timer is paused during effect"
    );
    assert_eq!(deck.cpu().bus.peek(0x311), 36);
    assert_eq!(deck.apu().pulse1.real_period, period(36));
    assert_eq!(deck.apu().pulse1.envelope.volume, 15);
    assert_eq!(deck.apu().pulse1.duty, 0);
}

#[test]
fn standalone_effect_starts_and_waits_for_longest_sequence() {
    let mut assets = assets();
    let mut effect = effect(0);
    effect.vol_sequence = vec![15];
    effect.duty_sequence.clear();
    effect.arpeggio_sequence = vec![0, 4, 7];
    assets.sound_effects.push(effect);
    let mut deck = deck(
        r#"
        PLAY_SFX(0)
        ASM
            JSR Sound_Update
        END ASM
        POKE($5C0, 1)
        ASM
            JSR Sound_Update
            JSR Sound_Update
        END ASM
        POKE($5C0, 2)
        ASM
            JSR Sound_Update
        END ASM
        POKE($5C0, 3)
    "#,
        assets,
    );
    until(&mut deck, 1);
    assert_eq!(deck.cpu().bus.peek(0x300), 2);
    assert_eq!(deck.apu().pulse1.real_period, period(48));
    assert_eq!(deck.apu().pulse1.envelope.volume, 15);
    until(&mut deck, 2);
    assert_eq!(
        deck.cpu().bus.peek(0x300),
        2,
        "short envelopes must not end longer ones"
    );
    assert_eq!(deck.apu().pulse1.real_period, period(55) - 2);
    until(&mut deck, 3);
    assert_eq!(deck.cpu().bus.peek(0x300), 0);
    assert_eq!(deck.apu().pulse1.envelope.volume, 0);
}

#[test]
fn music_arpeggio_is_signed_bounded_and_does_not_drift() {
    let mut assets = assets();
    assets.envelopes = vec![
        AudioEnvelope {
            name: "Arp".into(),
            steps: vec![(0, 1), (4, 1), (7, 1), (-128, 1), (127, 1)],
            loop_index: Some(0),
        },
        AudioEnvelope {
            name: "Pitch".into(),
            steps: vec![(-2, 1)],
            loop_index: None,
        },
    ];
    let mut track = track(0, 48, 0);
    track.arpeggio_env = Some(0);
    track.pitch_env = Some(1);
    assets.audio_tracks.push(track);
    let mut body = String::from("Sound.Play(0)\n");
    for checkpoint in 1..=6 {
        body.push_str(&format!(
            "ASM\nJSR Sound_Update\nEND ASM\nPOKE($5C0, {checkpoint})\n"
        ));
    }
    let mut deck = deck(&body, assets);
    for (checkpoint, note) in [48, 52, 55, 0, 95, 48].into_iter().enumerate() {
        until(&mut deck, checkpoint as u8 + 1);
        assert_eq!(deck.apu().pulse1.real_period, period(note) - 2);
        assert_eq!(
            deck.cpu().bus.peek(0x311),
            48,
            "base note must be preserved"
        );
    }
}

#[test]
fn dmc_track_uses_sample_address_length_and_authored_rate() {
    let mut assets = assets();
    assets.samples = vec![
        DpcmSample {
            name: "First".into(),
            data: vec![0xaa; 33],
        },
        DpcmSample {
            name: "Second".into(),
            data: vec![0x55; 17],
        },
    ];
    let mut track = track(3, 1, 0);
    track.instrument = 7;
    assets.audio_tracks.push(track);
    let mut deck = deck(
        r#"
        Sound.Play(0)
        ASM
            JSR Sound_Update
        END ASM
        POKE($5C0, 1)
        Sound.Stop()
        POKE($5C0, 2)
    "#,
        assets,
    );
    until(&mut deck, 1);
    assert_eq!(deck.apu().dmc.sample_addr, 0xe080);
    assert_eq!(deck.apu().dmc.sample_length, 17);
    // The backend counts down through zero, storing the 214-cycle interval minus one.
    assert_eq!(deck.apu().dmc.timer.period, 213);
    assert!(deck.apu().dmc.bytes_remaining > 0);
    until(&mut deck, 2);
    assert_eq!(deck.apu().dmc.bytes_remaining, 0);
}

#[test]
fn looping_effect_has_exact_speed_and_independent_channel_state() {
    let mut assets = assets();
    assets.audio_tracks.push(track(0, 36, 10));
    let mut effect = effect(1);
    effect.channel = 1;
    effect.speed = 2;
    effect.does_loop = true;
    effect.vol_sequence = vec![15];
    effect.pitch_sequence.clear();
    effect.duty_sequence = vec![0, 3];
    effect.arpeggio_sequence = vec![0, 4, 7];
    assets.sound_effects.push(effect);
    let mut body = String::from("Sound.Play(0)\nASM\nJSR Sound_Update\nEND ASM\nPLAY_SFX(0)\n");
    for checkpoint in 1..=7 {
        body.push_str(&format!(
            "ASM\nJSR Sound_Update\nEND ASM\nPOKE($5C0, {checkpoint})\n"
        ));
    }
    let mut deck = deck(&body, assets);
    for (checkpoint, note) in [48, 48, 52, 52, 55, 55, 48].into_iter().enumerate() {
        until(&mut deck, checkpoint as u8 + 1);
        assert_eq!(deck.apu().pulse2.real_period, period(note));
        assert_eq!(deck.apu().pulse2.envelope.volume, 15);
        assert_eq!(deck.cpu().bus.peek(0x320), 2);
        assert_eq!(deck.cpu().bus.peek(0x300), 1);
        assert_eq!(deck.apu().pulse1.real_period, period(36));
    }
}

#[test]
fn music_rest_advances_and_finite_envelopes_restart_at_next_note() {
    let mut assets = assets();
    assets.envelopes.push(AudioEnvelope {
        name: "Volume".into(),
        steps: vec![(15, 1), (3, 1)],
        loop_index: None,
    });
    let mut track = track(0, 48, 0);
    track.notes[0].duration = 2;
    track.notes.push(AudioNote {
        pitch: 52,
        row: 0,
        col: 1,
        duration: 2,
    });
    track.vol_env = Some(0);
    assets.audio_tracks.push(track);
    let mut body = String::from("Sound.Play(0)\n");
    for checkpoint in 1..=9 {
        body.push_str(&format!(
            "ASM\nJSR Sound_Update\nEND ASM\nPOKE($5C0, {checkpoint})\n"
        ));
    }
    let mut deck = deck(&body, assets);
    until(&mut deck, 1);
    assert_eq!(deck.apu().pulse1.envelope.volume, 15);
    until(&mut deck, 2);
    assert_eq!(deck.apu().pulse1.envelope.volume, 3);
    until(&mut deck, 3);
    assert_eq!(deck.apu().pulse1.envelope.volume, 0);
    assert_eq!(deck.cpu().bus.peek(0x311), 255);
    until(&mut deck, 9);
    assert_eq!(deck.cpu().bus.peek(0x311), 52);
    assert_eq!(deck.apu().pulse1.real_period, period(52));
    assert_eq!(deck.apu().pulse1.envelope.volume, 15);
}

#[test]
fn missing_music_and_effect_ids_preserve_active_channel() {
    let mut assets = assets();
    assets.audio_tracks.push(track(0, 36, 10));
    assets.sound_effects.push(effect(15));
    let mut deck = deck(
        r#"
        Sound.Play(0)
        ASM
            JSR Sound_Update
        END ASM
        Sound.Play(127)
        PLAY_SFX(127)
        Sound.Play(256)
        PLAY_SFX(256)
        POKE($5C0, 1)
    "#,
        assets,
    );
    until(&mut deck, 1);
    assert_eq!(deck.cpu().bus.peek(0x300), 1);
    assert_eq!(deck.cpu().bus.peek(0x305), 10);
    assert_eq!(deck.cpu().bus.peek(0x301), 32);
    assert_eq!(deck.apu().pulse1.real_period, period(36));
}

#[test]
fn generated_nmi_advances_music_envelopes_each_real_frame() {
    let mut assets = assets();
    assets.envelopes = vec![
        AudioEnvelope {
            name: "Chord".into(),
            steps: vec![(0, 1), (4, 1), (7, 1)],
            loop_index: Some(0),
        },
        AudioEnvelope {
            name: "Volume".into(),
            steps: vec![(15, 1), (7, 1), (3, 1)],
            loop_index: Some(0),
        },
    ];
    let mut track = track(0, 48, 0);
    track.arpeggio_env = Some(0);
    track.vol_env = Some(1);
    assets.audio_tracks.push(track);
    let source = "DIM ticks AS BYTE\nSUB Main()\nSound.Play(0)\nPOKE($2000, $80)\nFOR ticks = 0 TO 255\nWAIT_VBLANK\nNEXT ticks\nEND SUB\nINTERRUPT NMI()\nLET ticks = ticks + 1\nEND INTERRUPT";
    let (rom, _) = compile_source(Some(source.into()), None, Some(assets)).unwrap();
    let mut deck = ControlDeck::new();
    deck.load_rom("frame-audio.nes", &mut &rom[..]).unwrap();
    for (frame, (note, volume)) in [(48, 15), (52, 7), (55, 3), (48, 15)]
        .into_iter()
        .enumerate()
    {
        until(&mut deck, frame as u8 + 1);
        assert_eq!(deck.apu().pulse1.real_period, period(note));
        assert_eq!(deck.apu().pulse1.envelope.volume, volume);
        assert_eq!(deck.cpu().bus.peek(0x311), 48);
    }
}

#[test]
fn invalid_audio_data_is_rejected_and_old_sfx_json_gets_empty_arpeggio() {
    let old: SoundEffect =
        serde_json::from_value(serde_json::json!({"name": "Legacy", "channel": 0})).unwrap();
    assert!(old.arpeggio_sequence.is_empty());
    let mut assets = assets();
    assets.envelopes.push(AudioEnvelope {
        name: "Invalid".into(),
        steps: vec![(0, 0)],
        loop_index: None,
    });
    assert!(compile_source(
        Some("SUB Main()\nEND SUB".into()),
        None,
        Some(assets.clone())
    )
    .unwrap_err()
    .contains("nonzero-duration"));
    assets.envelopes[0].steps[0].1 = 1;
    assets.envelopes[0].loop_index = Some(1);
    assert!(assets
        .validate_for_compile()
        .unwrap_err()
        .contains("loop index"));
    assets.envelopes.clear();
    assets.samples.push(DpcmSample {
        name: "Too big".into(),
        data: vec![0; 4082],
    });
    assert!(assets.validate_for_compile().unwrap_err().contains("4081"));
}

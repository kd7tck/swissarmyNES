use swissarmynes::compiler::audio::compile_audio_data;
use swissarmynes::server::project::{AudioNote, AudioTrack, ProjectAssets};

#[test]
fn test_audio_priority_compilation() {
    let track1 = AudioTrack {
        name: "Low Prio".to_string(),
        notes: vec![AudioNote {
            pitch: 10,
            row: 0,
            col: 0,
            duration: 8,
        }],
        channel: 0,
        instrument: 0x3F,
        priority: 0,
        vol_env: None,
        pitch_env: None,
        arpeggio_env: None,
    };

    let track2 = AudioTrack {
        name: "High Prio".to_string(),
        notes: vec![AudioNote {
            pitch: 20,
            row: 0,
            col: 0,
            duration: 8,
        }],
        channel: 0,
        instrument: 0x7F,
        priority: 10,
        vol_env: None,
        pitch_env: None,
        arpeggio_env: None,
    };

    let assets = ProjectAssets {
        chr_bank: vec![],
        palettes: vec![],
        nametables: vec![],
        audio_tracks: vec![track1, track2],
        envelopes: vec![],
        samples: vec![],
        sound_effects: vec![],
        metatiles: vec![],
        world: None,
        metasprites: vec![],
        animations: vec![],
    };

    let blob = compile_audio_data(&Some(assets)).unwrap();

    // Header: Count (1) + Ptrs (4) = 5 bytes
    assert_eq!(blob[0], 2); // Count

    // Track 1
    // Ptr 1: blob[1], blob[2]
    let addr1 = (blob[1] as u16) | ((blob[2] as u16) << 8);
    // Track 2
    // Ptr 2: blob[3], blob[4]
    let addr2 = (blob[3] as u16) | ((blob[4] as u16) << 8);

    assert!(addr1 < addr2);

    // Verify Track 1 Data
    // Offset relative to MUSIC_DATA_ADDR ($D100)
    // First track starts at offset 5.
    let offset1 = (addr1 - 0xD100) as usize;
    assert_eq!(offset1, 5);

    // Channel
    assert_eq!(blob[offset1], 0);
    // Instrument
    assert_eq!(blob[offset1 + 1], 0x3F);
    // Priority (New!)
    assert_eq!(blob[offset1 + 2], 0);

    // Verify Track 2 Data
    let offset2 = (addr2 - 0xD100) as usize;
    // Channel
    assert_eq!(blob[offset2], 0);
    // Instrument
    assert_eq!(blob[offset2 + 1], 0x7F);
    // Priority
    assert_eq!(blob[offset2 + 2], 10);
}

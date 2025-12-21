use swissarmynes::compiler::audio;
use swissarmynes::server::project::{AudioNote, AudioTrack, DpcmSample, ProjectAssets};

#[test]
fn test_compile_samples() {
    // Logic: len=20. (20-1)%16 = 3. Needed = 13.
    // Total len = 33.
    // L = (33-1)/16 = 2.
    // Played len = 2*16 + 1 = 33.
    let sample1_data = vec![0xFF; 20];

    let sample1 = DpcmSample {
        name: "Test1".to_string(),
        data: sample1_data,
    };

    // Sample 2: 1 byte.
    // Logic: len=1. (1-1)%16 = 0. Needed=0.
    // Total len = 1.
    // L = 0.
    let sample2 = DpcmSample {
        name: "Test2".to_string(),
        data: vec![0xAA],
    };

    let assets = ProjectAssets {
        samples: vec![sample1, sample2],
        chr_bank: vec![],
        palettes: vec![],
        nametables: vec![],
        audio_tracks: vec![],
    };

    let (samples_blob, table_blob) = audio::compile_samples(&Some(assets));

    // Check Table
    // Entry 1: Addr (relative to $C000 >> 6), Len (L)
    // Start Addr = $E040. ($E040 - $C000) >> 6 = $2040 >> 6 = 8256 >> 6 = 129.
    // So A = 129 ($81).
    // Len = 2.

    assert_eq!(table_blob[0], 129, "Sample 1 Address incorrect");
    assert_eq!(table_blob[1], 2, "Sample 1 Length incorrect");

    // Entry 2:
    // Start Addr = $E040 + 33 bytes.
    // $E040 + 33 = $E061.
    // Alignment check: $E061 % 64 = 57441 % 64 = 33.
    // Need padding: 64 - 33 = 31 bytes padding.
    // New Start = $E080.
    // Diff = $E080 - $C000 = 8320.
    // 8320 / 64 = 130.
    // So A = 130 ($82).
    // Len = 0.

    assert_eq!(table_blob[2], 130, "Sample 2 Address incorrect");
    assert_eq!(table_blob[3], 0, "Sample 2 Length incorrect");

    // Check Samples Blob
    // Length: 33 (sample1) + 31 (padding) + 1 (sample2) = 65 bytes.
    assert_eq!(samples_blob.len(), 65, "Samples blob length incorrect");
}

#[test]
fn test_compile_audio_dmc() {
    let track = AudioTrack {
        name: "DMC Track".to_string(),
        channel: 3,
        instrument: 0x0F, // Rate F
        priority: 0,
        notes: vec![
            AudioNote {
                col: 0,
                row: 0,
                pitch: 0,
                duration: 8,
            }, // Play Sample 0
        ],
    };

    let assets = ProjectAssets {
        audio_tracks: vec![track],
        chr_bank: vec![],
        palettes: vec![],
        nametables: vec![],
        samples: vec![],
    };

    let blob = audio::compile_audio_data(&Some(assets));

    // Check Header
    // Count = 1
    assert_eq!(blob[0], 1);

    // Pointer (2 bytes)
    // Data starts at 1 + 2 = 3.
    // Addr = $D100 + 3 = $D103.
    assert_eq!(blob[1], 0x03);
    assert_eq!(blob[2], 0xD1);

    // Track Data
    // Channel = 3
    assert_eq!(blob[3], 3);
    // Instrument = 0x0F
    assert_eq!(blob[4], 0x0F);
    // Priority = 0
    assert_eq!(blob[5], 0);
    // Note: Duration 8, Pitch 0
    assert_eq!(blob[6], 8);
    assert_eq!(blob[7], 0);
    // Terminator
    assert_eq!(blob[8], 0);
}

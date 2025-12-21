#[cfg(test)]
mod tests {
    use swissarmynes::compiler::audio::{compile_audio_data, generate_period_table};
    use swissarmynes::server::project::{AudioNote, AudioTrack, ProjectAssets};

    #[test]
    fn test_period_table_size() {
        let table = generate_period_table();
        assert_eq!(table.len(), 192); // 96 notes * 2 bytes
    }

    #[test]
    fn test_audio_compilation_basic() {
        // FRAMES_PER_STEP = 8
        let track1 = AudioTrack {
            name: "Test Track".to_string(),
            notes: vec![
                AudioNote {
                    pitch: 10,
                    row: 0,
                    col: 0,      // Time 0
                    duration: 8, // Ends at 8
                },
                AudioNote {
                    pitch: 12,
                    row: 0,
                    col: 1,      // Time 8
                    duration: 8, // Ends at 16
                },
            ],
            channel: 0, // P1
            instrument: 0x9F,
            priority: 0,
            vol_env: None,
            pitch_env: None,
            arpeggio_env: None,
        };

        let assets = ProjectAssets {
            chr_bank: vec![],
            palettes: vec![],
            nametables: vec![],
            audio_tracks: vec![track1],
            envelopes: vec![],
            samples: vec![],
            sound_effects: vec![],
        };

        let blob = compile_audio_data(&Some(assets));

        // Validation
        // Header: Count(1) + Ptrs(2*1) = 3 bytes
        assert_eq!(blob[0], 1); // Count
        assert_eq!(blob[1], 3); // Ptr Low (Addr $D103) -> 03
        assert_eq!(blob[2], 0xD1); // Ptr High -> D1

        // Track Data
        // Channel (1) + Inst (1) + Prio (1) + VolEnv (1) + PitchEnv (1) + ArpEnv (1)
        assert_eq!(blob[3], 0); // Channel 0
        assert_eq!(blob[4], 0x9F); // Instrument
        assert_eq!(blob[5], 0); // Priority
        assert_eq!(blob[6], 0xFF); // VolEnv (None)
        assert_eq!(blob[7], 0xFF); // PitchEnv (None)
        assert_eq!(blob[8], 0xFF); // ArpEnv (None)

        // Notes
        // Note 1: Dur(8), Pitch(10)
        assert_eq!(blob[9], 8);
        assert_eq!(blob[10], 10);

        // Note 2: Dur(8), Pitch(12)
        assert_eq!(blob[11], 8);
        assert_eq!(blob[12], 12);

        // Terminator
        assert_eq!(blob[13], 0);
    }

    #[test]
    fn test_audio_compilation_with_gaps() {
        // FRAMES_PER_STEP = 8
        let track1 = AudioTrack {
            name: "Gap Track".to_string(),
            notes: vec![
                AudioNote {
                    pitch: 10,
                    row: 0,
                    col: 0,      // Time 0
                    duration: 8, // Ends 8
                },
                // Gap from 8 to 24 (Step 3 * 8 = 24)
                // Gap size: 24 - 8 = 16
                AudioNote {
                    pitch: 12,
                    row: 0,
                    col: 3, // Time 24
                    duration: 4,
                },
            ],
            channel: 0,
            instrument: 0x9F,
            priority: 0,
            vol_env: None,
            pitch_env: None,
            arpeggio_env: None,
        };

        let assets = ProjectAssets {
            chr_bank: vec![],
            palettes: vec![],
            nametables: vec![],
            audio_tracks: vec![track1],
            envelopes: vec![],
            samples: vec![],
            sound_effects: vec![],
        };

        let blob = compile_audio_data(&Some(assets));

        // Offset: 3 (Header) + 6 (Chan/Inst/Prio/VolEnv/PitchEnv/ArpEnv) = 9
        // Note 1: 9,10 -> Time ends at 8.
        assert_eq!(blob[9], 8);
        assert_eq!(blob[10], 10);

        // Gap: 16 ticks (8 to 24).
        // Silence: Dur(16), Pitch(0xFF)
        assert_eq!(blob[11], 16);
        assert_eq!(blob[12], 0xFF);
        // Note 2:
        assert_eq!(blob[13], 4);
        assert_eq!(blob[14], 12);
    }

    #[test]
    fn test_audio_compilation_empty() {
        let blob = compile_audio_data(&None);
        assert_eq!(blob[0], 0);
    }
}

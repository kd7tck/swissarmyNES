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
                    col: 0, // Time 0
                    duration: 8, // Ends at 8
                },
                AudioNote {
                    pitch: 12,
                    row: 0,
                    col: 1, // Time 8
                    duration: 8, // Ends at 16
                },
            ],
            channel: 0, // P1
            instrument: 0x9F,
        };

        let assets = ProjectAssets {
            chr_bank: vec![],
            palettes: vec![],
            nametables: vec![],
            audio_tracks: vec![track1],
        };

        let blob = compile_audio_data(&Some(assets));

        // Validation
        // Header: Count(1) + Ptrs(2*1) = 3 bytes
        assert_eq!(blob[0], 1); // Count
        assert_eq!(blob[1], 3 & 0xFF); // Ptr Low (Addr $D103) -> 03
        assert_eq!(blob[2], 0xD1); // Ptr High -> D1

        // Track Data
        // Channel (1 byte) + Instrument (1 byte)
        assert_eq!(blob[3], 0); // Channel 0
        assert_eq!(blob[4], 0x9F); // Instrument

        // Notes
        // Note 1: Dur(8), Pitch(10)
        assert_eq!(blob[5], 8);
        assert_eq!(blob[6], 10);

        // Note 2: Dur(8), Pitch(12)
        assert_eq!(blob[7], 8);
        assert_eq!(blob[8], 12);

        // Terminator
        assert_eq!(blob[9], 0);
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
                    col: 0, // Time 0
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
        };

        let assets = ProjectAssets {
            chr_bank: vec![],
            palettes: vec![],
            nametables: vec![],
            audio_tracks: vec![track1],
        };

        let blob = compile_audio_data(&Some(assets));

        // Offset: 3 (Header) + 2 (Chan/Inst) = 5
        // Note 1: 5,6 -> Time ends at 8.
        assert_eq!(blob[5], 8);
        assert_eq!(blob[6], 10);

        // Gap: 16 ticks (8 to 24).
        // Silence: Dur(16), Pitch(0xFF)
        assert_eq!(blob[7], 16);
        assert_eq!(blob[8], 0xFF);
        // Note 2:
        assert_eq!(blob[9], 4);
        assert_eq!(blob[10], 12);
    }

    #[test]
    fn test_audio_compilation_empty() {
        let blob = compile_audio_data(&None);
        assert_eq!(blob[0], 0);
    }
}

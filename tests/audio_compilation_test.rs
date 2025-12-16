#[cfg(test)]
mod tests {
    use swissarmynes::compiler::audio;
    use swissarmynes::server::project::{AudioNote, AudioTrack, ProjectAssets};

    #[test]
    fn test_period_table_size() {
        let table = audio::generate_period_table();
        assert_eq!(table.len(), 192); // 96 notes * 2 bytes
    }

    #[test]
    fn test_audio_compilation_empty() {
        let data = audio::compile_audio_data(&None);
        assert_eq!(data, vec![0]); // Count = 0
    }

    #[test]
    fn test_audio_compilation_basic() {
        let track = AudioTrack {
            name: "Test".to_string(),
            envelope: 0,
            notes: vec![
                AudioNote {
                    pitch: 10,
                    row: 0,
                    col: 0,
                    duration: 4,
                },
                AudioNote {
                    pitch: 20,
                    row: 1,
                    col: 0, // Same col (overlap) - logic should take one?
                    duration: 0,
                }, // Should be filtered out
                AudioNote {
                    pitch: 30,
                    row: 2,
                    col: 4, // Starts after first note ends (0+4=4)
                    duration: 8,
                },
            ],
        };
        let assets = ProjectAssets {
            chr_bank: vec![],
            palettes: vec![],
            nametables: vec![],
            audio_tracks: vec![track],
        };

        let data = audio::compile_audio_data(&Some(assets));

        // Expected format:
        // Header:
        //   Count: 1 byte (1)
        //   Pointer: 2 bytes (Ptr Low, Ptr High)
        // Data:
        //   Channel: 1 byte (0)
        //   Note 1: Duration (4), Pitch (10)
        //   Note 2: Duration (8), Pitch (30) -- Note duration 0 skipped
        //   Terminator: 0

        assert_eq!(data[0], 1); // Count

        // Pointer is at data[1], data[2]. Should point to $D100 + 1 + 2 = $D103.
        // Base address is 0xD100.
        // Offset of track data start is 3 (1 byte count + 2 bytes ptr).
        // So Absolute Addr = 0xD100 + 3 = 0xD103.
        let ptr = (data[1] as u16) | ((data[2] as u16) << 8);
        assert_eq!(ptr, 0xD103);

        // Track Data at index 3
        assert_eq!(data[3], 0); // Envelope/Channel
        assert_eq!(data[4], 4); // Dur 1
        assert_eq!(data[5], 10); // Pitch 1
        assert_eq!(data[6], 8); // Dur 2
        assert_eq!(data[7], 30); // Pitch 2
        assert_eq!(data[8], 0); // Terminator
        assert_eq!(data.len(), 9);
    }

    #[test]
    fn test_audio_compilation_with_gaps() {
        let track = AudioTrack {
            name: "GapTest".to_string(),
            envelope: 0,
            notes: vec![
                AudioNote {
                    pitch: 10,
                    row: 0,
                    col: 0,
                    duration: 2,
                },
                // Gap from 2 to 4 (Duration 2)
                AudioNote {
                    pitch: 20,
                    row: 0,
                    col: 4,
                    duration: 2,
                },
            ],
        };
        let assets = ProjectAssets {
            chr_bank: vec![],
            palettes: vec![],
            nametables: vec![],
            audio_tracks: vec![track],
        };

        let data = audio::compile_audio_data(&Some(assets));

        // Expected Data (starting at index 3):
        // Envelope (0)
        // Note 1: Dur 2, Pitch 10
        // Rest: Dur 2 (4-2), Pitch 255 ($FF)
        // Note 2: Dur 2, Pitch 20
        // Terminator

        assert_eq!(data[3], 0); // Envelope

        // Note 1
        assert_eq!(data[4], 2);
        assert_eq!(data[5], 10);

        // Rest
        assert_eq!(data[6], 2); // Gap duration
        assert_eq!(data[7], 0xFF); // Silence Pitch

        // Note 2
        assert_eq!(data[8], 2);
        assert_eq!(data[9], 20);

        // Terminator
        assert_eq!(data[10], 0);
    }
}

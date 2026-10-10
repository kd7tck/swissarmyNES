#[cfg(test)]
mod tests {
    use swissarmynes::compiler::audio::{compile_envelopes, compile_sfx_data};
    use swissarmynes::server::project::{ProjectAssets, SoundEffect};

    #[test]
    fn test_sfx_compilation() {
        let sfx1 = SoundEffect {
            name: "Jump".to_string(),
            channel: 0,
            priority: 10,
            speed: 2,
            vol_sequence: vec![15, 10, 5, 0],
            pitch_sequence: vec![0, 1, 2],
            arpeggio_sequence: vec![],
            duty_sequence: vec![0, 1],
            does_loop: false,
        };

        let assets = ProjectAssets {
            chr_bank: vec![],
            palettes: vec![],
            nametables: vec![],
            audio_tracks: vec![],
            envelopes: vec![], // No user envelopes
            samples: vec![],
            sound_effects: vec![sfx1],
            metatiles: vec![],
            world: None,
            metasprites: vec![],
            animations: vec![],
        };

        // 1. Check Envelopes
        let env_blob = compile_envelopes(&Some(assets.clone())).unwrap();

        // Count: 0 User + 4 SFX = 4 (volume, timer offset, duty, semitones)
        assert_eq!(env_blob[0], 4);

        // Pointers: 4 * 2 = 8 bytes.
        // Ptr1 (Vol): Start + 9.
        // Start = $DA00.
        // Offset = 9.
        // Ptr1 = $DA09. -> 09 DA
        assert_eq!(env_blob[1], 0x09);
        assert_eq!(env_blob[2], 0xDA);

        // Vol Data:
        // Loop: FF (No Loop)
        assert_eq!(env_blob[9], 0xFF);
        // Steps: (15, 2), (10, 2), (5, 2), (0, 2) -> 8 bytes
        assert_eq!(env_blob[10], 15);
        assert_eq!(env_blob[11], 2);
        assert_eq!(env_blob[16], 0);
        assert_eq!(env_blob[17], 2);
        // Term: 0, 0
        assert_eq!(env_blob[18], 0);
        assert_eq!(env_blob[19], 0);

        // Next Env (Pitch) starts at 20.
        // Ptr2 = $DA00 + 20 = $DA14 -> 14 DA
        assert_eq!(env_blob[3], 0x14);
        assert_eq!(env_blob[4], 0xDA);

        // Pitch Data:
        // Loop: FF
        assert_eq!(env_blob[20], 0xFF);
        // Steps: (0, 2), (1, 2), (2, 2) -> 6 bytes
        // Term: 0, 0

        // Next Env (Duty) starts at 20 + 1 + 6 + 2 = 29.
        // Ptr3 = $DA1D -> 1D DA
        assert_eq!(env_blob[5], 0x1D);
        assert_eq!(env_blob[6], 0xDA);

        // 2. Check SFX Data
        let sfx_blob = compile_sfx_data(&Some(assets)).unwrap();

        // Count: 1
        assert_eq!(sfx_blob[0], 1);

        // Ptrs: 2 bytes.
        // SFX1 Start = $D900 + 3
        assert_eq!(sfx_blob[1], 0x03);
        assert_eq!(sfx_blob[2], 0xD9);

        // Data:
        // Channel: 0
        assert_eq!(sfx_blob[3], 0);
        // Prio: 10
        assert_eq!(sfx_blob[4], 10);
        // VolEnvID: 0
        assert_eq!(sfx_blob[5], 0);
        // PitchEnvID: 1
        assert_eq!(sfx_blob[6], 1);
        // DutyEnvID: 2
        assert_eq!(sfx_blob[7], 2);
        assert_eq!(sfx_blob[8], 3); // Arpeggio
    }

    #[test]
    fn test_sfx_looping() {
        let sfx1 = SoundEffect {
            name: "Loop".to_string(),
            channel: 0,
            priority: 10,
            speed: 2,
            vol_sequence: vec![15],
            pitch_sequence: vec![0],
            arpeggio_sequence: vec![],
            duty_sequence: vec![0],
            does_loop: true,
        };

        let assets = ProjectAssets {
            chr_bank: vec![],
            palettes: vec![],
            nametables: vec![],
            audio_tracks: vec![],
            envelopes: vec![],
            samples: vec![],
            sound_effects: vec![sfx1],
            metatiles: vec![],
            world: None,
            metasprites: vec![],
            animations: vec![],
        };

        let env_blob = compile_envelopes(&Some(assets)).unwrap();

        // Ptr1 (Vol) at Start+9
        // Vol Loop should be 0 (Start index) instead of 0xFF
        assert_eq!(env_blob[9], 0);
    }
}

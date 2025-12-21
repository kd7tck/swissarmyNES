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
            duty_sequence: vec![0, 1],
        };

        let assets = ProjectAssets {
            chr_bank: vec![],
            palettes: vec![],
            nametables: vec![],
            audio_tracks: vec![],
            envelopes: vec![], // No user envelopes
            samples: vec![],
            sound_effects: vec![sfx1],
        };

        // 1. Check Envelopes
        let env_blob = compile_envelopes(&Some(assets.clone()));

        // Count: 0 User + 3 SFX = 3
        assert_eq!(env_blob[0], 3);

        // Pointers: 3 * 2 = 6 bytes.
        // Ptr1 (Vol): Start + 7.
        // Start = $DA00.
        // Offset = 7.
        // Ptr1 = $DA07. -> 07 DA
        assert_eq!(env_blob[1], 0x07);
        assert_eq!(env_blob[2], 0xDA);

        // Vol Data:
        // Loop: FF (No Loop)
        assert_eq!(env_blob[7], 0xFF);
        // Steps: (15, 2), (10, 2), (5, 2), (0, 2) -> 8 bytes
        assert_eq!(env_blob[8], 15);
        assert_eq!(env_blob[9], 2);
        assert_eq!(env_blob[14], 0);
        assert_eq!(env_blob[15], 2);
        // Term: 0, 0
        assert_eq!(env_blob[16], 0);
        assert_eq!(env_blob[17], 0);

        // Next Env (Pitch) starts at 18
        // Ptr2 = $DA00 + 18 = $DA12 -> 12 DA
        assert_eq!(env_blob[3], 0x12);
        assert_eq!(env_blob[4], 0xDA);

        // Pitch Data:
        // Loop: FF
        assert_eq!(env_blob[18], 0xFF);
        // Steps: (0, 2), (1, 2), (2, 2) -> 6 bytes
        // Term: 0, 0

        // Next Env (Duty) starts at 18 + 1 + 6 + 2 = 27
        // Ptr3 = $DA1B -> 1B DA
        assert_eq!(env_blob[5], 0x1B);
        assert_eq!(env_blob[6], 0xDA);

        // 2. Check SFX Data
        let sfx_blob = compile_sfx_data(&Some(assets));

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
    }
}

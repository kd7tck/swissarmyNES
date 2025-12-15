#[cfg(test)]
mod tests {
    use swissarmynes::compiler::{
        ast::{Program, TopLevel},
        codegen::CodeGenerator,
        symbol_table::SymbolTable,
    };
    use swissarmynes::server::project::{AudioNote, AudioTrack, ProjectAssets};

    #[test]
    fn test_audio_codegen_integration() {
        // 1. Setup minimal program
        let program = Program {
            declarations: vec![TopLevel::Sub("Main".to_string(), vec![], vec![])],
        };

        // 2. Setup Symbols
        let st = SymbolTable::new();

        // 3. Setup Assets with Audio
        let track1 = AudioTrack {
            name: "TestTrack1".to_string(),
            notes: vec![
                AudioNote {
                    row: 0,
                    col: 0,
                    duration: 1,
                    pitch: 0,
                }, // Highest note at step 0
                AudioNote {
                    row: 23,
                    col: 4,
                    duration: 1,
                    pitch: 0,
                }, // Lowest note at step 4
            ],
            envelope: 0,
        };

        let assets = ProjectAssets {
            chr_bank: vec![],
            palettes: vec![],
            nametables: vec![],
            audio_tracks: vec![track1.clone(), track1.clone(), track1.clone()], // 3 tracks
        };

        // 4. Generate Code
        let mut cg = CodeGenerator::new(st);
        let output = cg
            .generate(&program, Some(&assets))
            .expect("Codegen failed");

        // 5. Verify Output contains Audio Engine
        let code = output.join("\n");

        assert!(code.contains("Sound_Init:"));
        assert!(code.contains("Sound_Update:"));
        assert!(code.contains("MusicData_Track1:"));
        assert!(code.contains("PeriodTable_Low:"));

        // Check for specific data values
        // Note: row 0 -> note index 24 (0x18).
        // Note: row 23 -> note index 1 (0x01).
        // Rest -> 0x00.
        // Step 0: 0x18. Step 1-3: 0x00. Step 4: 0x01.

        // We can't easily check binary packing in string without exact matching,
        // but we can check if the byte directives are there.
        // We expect .BYTE directives in MusicData_Track1
        assert!(code.contains(".BYTE $18, $00, $00, $00, $01"));
    }
}

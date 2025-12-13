use rs6502::Assembler as Rs6502Assembler;

pub struct Assembler {
    // Wrapper around rs6502
}

impl Assembler {
    pub fn new() -> Self {
        Self {}
    }

    /// Assembles a string of 6502 assembly code into a binary ROM.
    /// This currently returns just the PRG-ROM part.
    /// In a full NES build, we'd wrap this with an iNES header.
    pub fn assemble(&self, source: &str) -> Result<Vec<u8>, String> {
        let mut assembler = Rs6502Assembler::new();
        // 0 as offset means no global offset override, respect .ORG
        let segments = assembler
            .assemble_string(source, 0)
            .map_err(|e| format!("Assembler error: {:?}", e))?;

        // NES ROMs are usually contiguous in PRG-ROM, but the assembler returns segments based on .ORG
        // For a simple NROM (32KB PRG), we expect code at $8000.
        // We need to flatten these segments into a single binary blob padded with zeros/FFs.

        // Let's assume NROM-256 (32KB PRG) for now, mapped at $8000-$FFFF.
        let mut prg_rom = vec![0u8; 32768];

        for segment in segments {
            let start = segment.address;
            let code = segment.code;

            // Check bounds
            if start < 0x8000 {
                return Err(format!(
                    "Code segment starts at ${:04X}, which is outside PRG-ROM space ($8000+)",
                    start
                ));
            }

            let offset = (start - 0x8000) as usize;
            if offset + code.len() > prg_rom.len() {
                return Err(format!(
                    "Code segment at ${:04X} exceeds PRG-ROM size",
                    start
                ));
            }

            // Copy code into PRG ROM buffer
            for (i, byte) in code.iter().enumerate() {
                prg_rom[offset + i] = *byte;
            }
        }

        Ok(prg_rom)
    }
}

impl Default for Assembler {
    fn default() -> Self {
        Self::new()
    }
}

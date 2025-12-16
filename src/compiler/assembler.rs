use rs6502::Assembler as Rs6502Assembler;

pub struct Assembler;

impl Assembler {
    pub fn new() -> Self {
        Self
    }

    /// Assembles a string of 6502 assembly code into a binary ROM with iNES header.
    /// Returns a complete .nes file as a byte vector.
    ///
    /// # Arguments
    /// * `source` - The assembly source code
    /// * `chr_data` - Optional 8KB CHR-ROM data
    /// * `injections` - A list of (Address, Data) tuples to inject into PRG-ROM
    pub fn assemble(
        &self,
        source: &str,
        chr_data: Option<&[u8]>,
        injections: Vec<(u16, Vec<u8>)>,
    ) -> Result<Vec<u8>, String> {
        let mut assembler = Rs6502Assembler::new();
        // 0 as offset means no global offset override, respect .ORG in source
        let segments = assembler
            .assemble_string(source, 0)
            .map_err(|e| format!("Assembler error: {:?}", e))?;

        // Initialize NROM-256 (32KB PRG) buffer.
        // NROM-256 is mapped at $8000-$FFFF.
        let mut prg_rom = vec![0u8; 32768];

        // Process Assembler Output Segments
        for segment in segments {
            let start = segment.address;
            let code = segment.code;

            if code.is_empty() {
                continue;
            }

            // Check bounds to ensure code resides within PRG-ROM ($8000-$FFFF)
            if start < 0x8000 {
                return Err(format!(
                    "Code segment starts at ${:04X}, which is outside PRG-ROM space ($8000+). Code: {:?}",
                    start, code
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

        // Apply Binary Injections
        // This is used for data tables, assets (Nametables, Music), and any other fixed-address blobs
        // that are better handled as binary than as huge ASM source files.
        for (addr, data) in injections {
            if addr < 0x8000 {
                return Err(format!(
                    "Injection address ${:04X} is outside PRG-ROM space ($8000+)",
                    addr
                ));
            }
            let offset = (addr - 0x8000) as usize;

            if offset + data.len() > prg_rom.len() {
                return Err(format!(
                    "Injection at ${:04X} with length {} exceeds PRG-ROM size",
                    addr,
                    data.len()
                ));
            }

            for (i, byte) in data.iter().enumerate() {
                prg_rom[offset + i] = *byte;
            }
        }

        // Construct iNES Header (16 bytes)
        // Format Spec: https://www.nesdev.org/wiki/INES
        // Mapper 0 (NROM), 32KB PRG, 8KB CHR
        let header = vec![
            0x4E, 0x45, 0x53, 0x1A, // 'N', 'E', 'S', EOF
            0x02, // PRG-ROM size: 2 x 16KB = 32KB
            0x01, // CHR-ROM size: 1 x 8KB = 8KB
            0x01, // Flags 6: Vertical Mirroring, No Battery, Mapper Lower 0
            0x00, // Flags 7: Mapper Upper 0
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Padding
        ];

        let mut final_rom = header;
        final_rom.extend_from_slice(&prg_rom);

        // Append 8KB CHR-ROM
        // If user provided CHR data, use it (padded to 8KB).
        // Otherwise, use 8KB of zeros.
        let mut final_chr = vec![0u8; 8192];
        if let Some(data) = chr_data {
            let len = data.len().min(8192);
            final_chr[..len].copy_from_slice(&data[..len]);
        }
        final_rom.extend(final_chr);

        Ok(final_rom)
    }
}

impl Default for Assembler {
    fn default() -> Self {
        Self::new()
    }
}

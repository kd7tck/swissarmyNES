use rs6502::Assembler as Rs6502Assembler;
use std::collections::HashMap;

pub struct Assembler;

impl Assembler {
    pub fn new() -> Self {
        Self
    }

    /// Assembles multiple strings of 6502 assembly code (one per bank) into a binary ROM with iNES header.
    /// Returns a complete .nes file as a byte vector.
    ///
    /// # Arguments
    /// * `sources` - Map of Bank ID to assembly source code
    /// * `chr_data` - Optional 8KB CHR-ROM data
    /// * `injections` - A list of (Address, Data) tuples to inject into PRG-ROM (Applied to Fixed Bank 7 by default)
    pub fn assemble(
        &self,
        sources: &HashMap<u8, String>,
        chr_data: Option<&[u8]>,
        injections: Vec<(u16, Vec<u8>)>,
    ) -> Result<Vec<u8>, String> {
        // Initialize MMC1 (Mapper 1) PRG buffer.
        // 128KB PRG = 8 Banks of 16KB.
        let mut prg_rom = vec![0u8; 128 * 1024]; // 128KB
        let mut usage_map = vec![false; 128 * 1024];

        for (bank_id, source) in sources {
            let mut assembler = Rs6502Assembler::new();
            // 0 as offset means no global offset override, respect .ORG in source
            let segments = assembler
                .assemble_string(source, 0)
                .map_err(|e| format!("Assembler error in Bank {}: {:?}", bank_id, e))?;

            // Determine Bank Offset in the PRG ROM
            // Banks 0-7 are simply linear in the file.
            // Bank N corresponds to the 16KB chunk at offset N * 16KB.
            // However, assemblers output addresses like $8000 or $C000.
            // We need to map (BankID, Address) -> ROM Offset.

            // Logic:
            // If Address >= $C000: It's in the Fixed Bank window. This usually means Bank 7.
            // If Address >= $8000 and < $C000: It's in the Switchable Bank window.
            // BUT, the user might assemble code for Bank 2 at $8000.
            // So we rely on `bank_id`.

            let bank_offset = (*bank_id as usize) * 16384;

            if *bank_id > 7 {
                return Err(format!("Bank ID {} exceeds MMC1 128KB limit (0-7)", bank_id));
            }

            for segment in segments {
                let start = segment.address;
                let code = segment.code;

                if code.is_empty() {
                    continue;
                }

                // Calculate relative offset within the 16KB bank
                // If code is at $8000, rel = 0.
                // If code is at $C000, rel = 0. (Wait, $C000 starts the upper 16KB)
                // Actually, for Bank 7 (Fixed), it sits at $C000.
                // For Bank 0-6, they map to $8000.

                let rel_offset = if start >= 0xC000 {
                    (start - 0xC000) as usize
                } else if start >= 0x8000 {
                    (start - 0x8000) as usize
                } else {
                    return Err(format!(
                        "Code segment starts at ${:04X}, outside valid PRG space ($8000+)",
                        start
                    ));
                };

                if rel_offset >= 16384 {
                     return Err(format!(
                        "Code segment at ${:04X} exceeds 16KB bank size",
                        start
                    ));
                }

                let final_offset = bank_offset + rel_offset;

                if final_offset + code.len() > prg_rom.len() {
                    return Err(format!(
                        "Code segment at ${:04X} (Bank {}) exceeds PRG-ROM size",
                        start, bank_id
                    ));
                }

                // Copy code into PRG ROM buffer and check overlap
                for (i, byte) in code.iter().enumerate() {
                    let addr = final_offset + i;
                    if usage_map[addr] {
                        return Err(format!(
                            "Code segment at ${:04X} (Bank {}) overlaps with existing data at offset {:05X}",
                            start, bank_id, addr
                        ));
                    }
                    prg_rom[addr] = *byte;
                    usage_map[addr] = true;
                }
            }
        }

        // Apply Binary Injections
        // Injections are typically data tables or vectors.
        // We assume they go into the LAST bank (Bank 7, Fixed) which maps to $C000-$FFFF.
        // Or if they are explicitly addresses $8000-$BFFF, they go to Bank 0?
        // Let's enforce injections go to Bank 7 for now, as that's where Vectors live.
        // If an injection is at $E000, it's definitely Bank 7.
        // If at $D500, it's Bank 7.
        // If the user wants to inject to Bank 0, they can't via this API currently.

        let last_bank_offset = 7 * 16384;

        for (addr, data) in injections {
            // Assume injections are for the Fixed Bank ($C000-$FFFF)
            // But wait, DPCM samples ($E040) must be in the Fixed Bank or switched in.
            // Nametables ($D500) -> Fixed.
            // Vectors ($FFFA) -> Fixed.
            // So default to Bank 7.

            if addr < 0xC000 {
                 // Warn or allow?
                 // Some injections might be intended for $8000 (Bank 0).
                 // For now, let's map $8000-$BFFF to Bank 0?
                 // But in `CodeGenerator`, we defaulted `current_bank = 7`.
                 // So actually, most generated code is in Bank 7.
                 // If we have an injection at $8000, we might overwrite Bank 0 code?
                 // Let's stick to: Injections are for Fixed Bank (7) unless we expand the API.
                 // Most injections in SwissArmyNES are system tables ($D000+), so this is safe.
                 if addr < 0x8000 {
                     return Err(format!("Injection at ${:04X} is invalid", addr));
                 }
                 // If < $C000, it's ambiguous. But let's assume if the system generates it, it knows.
                 // Actually, Sound Engine tables are at $Dxxx.
                 // So they are > $C000.
                 // The only exception might be if we try to inject something lower.
            }

            let rel_offset = if addr >= 0xC000 {
                (addr - 0xC000) as usize
            } else {
                // If it's $8000-$BFFF, treat as Bank 0??
                // No, existing injections are all high memory.
                // Let's default to mapping $8000+ to Bank 7 relative? No that's wrong.
                // Let's error if < $C000 for now to be safe for MMC1.
                // Wait, Nametables are $D500. Correct.
                // Palettes $E000. Correct.
                return Err(format!("Injection at ${:04X} must be in fixed bank range ($C000-$FFFF)", addr));
            };

            let final_offset = last_bank_offset + rel_offset;

            if final_offset + data.len() > prg_rom.len() {
                return Err(format!(
                    "Injection at ${:04X} with length {} exceeds PRG-ROM size",
                    addr,
                    data.len()
                ));
            }

            for (i, byte) in data.iter().enumerate() {
                let current_addr = final_offset + i;
                if usage_map[current_addr] {
                    return Err(format!(
                        "Injection at ${:04X} overlaps with existing data at offset {:05X}",
                        addr,
                        current_addr
                    ));
                }
                prg_rom[current_addr] = *byte;
                usage_map[current_addr] = true;
            }
        }

        // Construct iNES Header (16 bytes)
        // Format Spec: https://www.nesdev.org/wiki/INES
        // Mapper 1 (MMC1), 128KB PRG (8x16), 8KB CHR
        let header = vec![
            0x4E, 0x45, 0x53, 0x1A, // 'N', 'E', 'S', EOF
            0x08, // PRG-ROM size: 8 x 16KB = 128KB
            0x01, // CHR-ROM size: 1 x 8KB = 8KB
            0x11, // Flags 6: Vertical(1) | Mapper Lower(1) = 0x11
            0x00, // Flags 7: Mapper Upper(0) = 0x00
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

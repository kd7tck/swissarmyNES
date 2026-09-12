// KEEP
use rs6502::Assembler as Rs6502Assembler;
use std::collections::HashMap;

pub struct Assembler;

impl Assembler {
    pub fn new() -> Self {
        Self
    }

    pub fn assemble(
        &self,
        asm: &[String],
        chr_data: Option<&[u8]>,
        injections: Vec<(u16, Vec<u8>)>,
    ) -> Result<Vec<u8>, String> {
        let mut banks = HashMap::new();
        banks.insert(0, asm.to_vec());
        self.assemble_banks(&banks, chr_data, injections)
    }

    pub fn assemble_banks(
        &self,
        banks: &HashMap<u8, Vec<String>>,
        chr_data: Option<&[u8]>,
        injections: Vec<(u16, Vec<u8>)>,
    ) -> Result<Vec<u8>, String> {
        let mut assembler = Rs6502Assembler::new();

        let mut prg_rom = vec![0u8; 131072];
        let mut usage_map = vec![false; 131072];

        let mut bank0_and_7 = String::new();
        if let Some(b0) = banks.get(&0) {
            bank0_and_7.push_str(&b0.join("\n"));
            bank0_and_7.push('\n');
        }
        if let Some(b7) = banks.get(&7) {
            bank0_and_7.push_str(&b7.join("\n"));
            bank0_and_7.push('\n');
        }

        let segments_0_7 = assembler
            .assemble_string(&bank0_and_7, 0)
            .map_err(|e| format!("Assembler error (Banks 0 & 7): {:?}", e))?;
        for segment in segments_0_7 {
            let start = segment.address;
            let code = segment.code;
            if code.is_empty() {
                continue;
            }

            let bank_idx = if start >= 0xC000 { 7 } else { 0 };
            let base_addr = if start >= 0xC000 { 0xC000 } else { 0x8000 };
            let offset = (bank_idx * 16384) + (start - base_addr) as usize;
            if offset + code.len() > prg_rom.len() {
                return Err(format!(
                    "Code segment at ${:04X} exceeds PRG-ROM size",
                    start
                ));
            }

            for (i, byte) in code.iter().enumerate() {
                let current_addr = offset + i;
                if usage_map[current_addr] {
                    return Err(format!("Code segment at ${:04X} overlaps", start));
                }
                prg_rom[current_addr] = *byte;
                usage_map[current_addr] = true;
            }
        }

        let bank7_str = if let Some(b7) = banks.get(&7) {
            b7.join("\n")
        } else {
            String::new()
        };

        for i in 1..=6 {
            if let Some(b) = banks.get(&i) {
                let mut combined = b.join("\n");
                combined.push('\n');
                combined.push_str(&bank7_str);

                let segments = assembler
                    .assemble_string(&combined, 0)
                    .map_err(|e| format!("Assembler error (Bank {}): {:?}", i, e))?;

                for segment in segments {
                    let start = segment.address;
                    let code = segment.code;
                    if code.is_empty() {
                        continue;
                    }
                    if start >= 0xC000 {
                        continue;
                    }
                    if start < 0x8000 {
                        return Err(format!(
                            "Code segment in Bank {} starts at ${:04X}",
                            i, start
                        ));
                    }

                    let offset = (i as usize * 16384) + (start - 0x8000) as usize;
                    if offset + code.len() > prg_rom.len() {
                        return Err(format!(
                            "Code segment at ${:04X} exceeds PRG-ROM size",
                            start
                        ));
                    }

                    for (j, byte) in code.iter().enumerate() {
                        let current_addr = offset + j;
                        if usage_map[current_addr] {
                            return Err(format!(
                                "Code segment in Bank {} at ${:04X} overlaps",
                                i, start
                            ));
                        }
                        prg_rom[current_addr] = *byte;
                        usage_map[current_addr] = true;
                    }
                }
            }
        }

        for (addr, data) in injections {
            if addr < 0x8000 {
                return Err(format!(
                    "Injection address ${:04X} is outside PRG-ROM space",
                    addr
                ));
            }
            let bank_idx = if addr >= 0xC000 { 7 } else { 0 };
            let base_addr = if addr >= 0xC000 { 0xC000 } else { 0x8000 };
            let offset = (bank_idx * 16384) + (addr - base_addr) as usize;
            if offset + data.len() > prg_rom.len() {
                return Err(format!("Injection at ${:04X} exceeds PRG-ROM size", addr));
            }

            for (i, byte) in data.iter().enumerate() {
                let current_addr = offset + i;
                if usage_map[current_addr] {
                    return Err(format!("Injection at ${:04X} overlaps", addr));
                }
                prg_rom[current_addr] = *byte;
                usage_map[current_addr] = true;
            }
        }

        let header = vec![
            0x4E, 0x45, 0x53, 0x1A, 0x08, 0x01, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];

        let mut final_rom = header;
        final_rom.extend_from_slice(&prg_rom);

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

//! Checked cartridge metadata, decoded before the backend may allocate memory.
//! Format reference: https://www.nesdev.org/wiki/NES_2.0

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartridgeInfo {
    pub nes2: bool,
    pub mapper: u16,
    pub submapper: u8,
    pub prg_bytes: usize,
    pub chr_bytes: usize,
    pub prg_ram_bytes: usize,
    pub prg_nvram_bytes: usize,
    pub chr_ram_bytes: usize,
    pub chr_nvram_bytes: usize,
    pub trainer: bool,
    pub vertical_mirroring: bool,
    pub four_screen: bool,
    pub battery: bool,
    pub timing: u8,
    pub console: u8,
    pub console_detail: u8,
    pub misc_roms: u8,
    pub expansion_device: u8,
}

fn rom_size(low: u8, high: u8, unit: usize) -> Result<usize, String> {
    let bytes = if high == 15 {
        1usize
            .checked_shl(u32::from(low >> 2))
            .and_then(|power| power.checked_mul(usize::from((low & 3) * 2 + 1)))
    } else {
        (usize::from(low) | (usize::from(high) << 8)).checked_mul(unit)
    }
    .ok_or("ROM size overflow")?;
    // Limit allocations consistently on native and wasm32 hosts.
    if bytes > 64 * 1024 * 1024 {
        return Err("ROM region exceeds supported 64 MiB limit".into());
    }
    Ok(bytes)
}

fn ram_size(shift: u8) -> usize {
    if shift == 0 {
        0
    } else {
        64usize << shift
    }
}

/// Bus/debug adapter to the production core's mapper routing.
/// CPU cycles and PPU accesses already clock mapper hardware in the backend.
pub trait Mapper {
    fn read_prg(&self, addr: u16) -> u8;
    fn write_prg(&mut self, addr: u16, val: u8);
    fn read_chr(&self, addr: u16) -> u8;
    fn write_chr(&mut self, addr: u16, val: u8);
    /// Query the mapper IRQ level without advancing time or acknowledging it.
    /// The historical name remains compatible with existing callers.
    fn step_irq(&mut self) -> bool;
}

impl CartridgeInfo {
    /// Execution capabilities are stricter than metadata parsing. Reject layouts
    /// the backend cannot faithfully address before replacing a running game.
    pub fn validate_supported_layout(&self) -> Result<(), String> {
        if self.mapper == 0 {
            if !matches!(self.prg_bytes, 16384 | 32768) || !matches!(self.chr_bytes, 0 | 8192) {
                return Err("NROM supports only 16/32 KiB PRG ROM and 0/8 KiB CHR ROM".into());
            }
            if self.submapper != 0 {
                return Err("Unsupported NROM submapper".into());
            }
            if self.total_prg_ram() > 8192 {
                return Err("NROM supports at most 8 KiB of PRG RAM".into());
            }
            if self.four_screen {
                return Err("Unsupported NROM four-screen nametable layout".into());
            }
        }
        if self.console != 0 || self.misc_roms != 0 {
            return Err("Unsupported cartridge console or miscellaneous ROM layout".into());
        }
        if self.nes2 && self.expansion_device > 1 {
            return Err("Unsupported cartridge expansion device; only unspecified/standard controllers are supported".into());
        }
        if self.nes2 && self.chr_bytes > 0 && self.chr_ram_bytes + self.chr_nvram_bytes > 0 {
            return Err("Unsupported mixed CHR ROM and CHR RAM layout".into());
        }
        Ok(())
    }
    pub fn total_prg_ram(&self) -> usize {
        self.prg_ram_bytes.saturating_add(self.prg_nvram_bytes)
    }

    pub fn has_battery_backup(&self) -> bool {
        self.battery || self.prg_nvram_bytes > 0 || self.chr_nvram_bytes > 0
    }

    pub fn parse(rom: &[u8]) -> Result<Self, String> {
        let header = rom.get(..16).ok_or("Truncated iNES header")?;
        if &header[..4] != b"NES\x1a" {
            return Err("Invalid iNES signature".into());
        }
        if matches!(header[7] & 12, 4 | 12) {
            return Err("Unsupported or corrupted iNES header variant".into());
        }
        let nes2 = header[7] & 12 == 8;
        // Ignoring reserved data can make the wrapper and backend interpret
        // different mappers (notably old signature garbage in iNES headers).
        if nes2 {
            if header[12] & 0xfc != 0
                || header[14] & 0xfc != 0
                || header[15] & 0xc0 != 0
                || (header[7] & 3 == 0 && header[13] != 0)
            {
                return Err("Nonzero reserved NES 2.0 header bits".into());
            }
        } else if header[9] & 0xfe != 0 || header[10..16].iter().any(|byte| *byte != 0) {
            return Err("Unsupported legacy header extension or nonzero reserved bytes".into());
        }
        let high = if nes2 { header[9] } else { 0 };
        let prg_bytes = rom_size(header[4], high & 15, 16384)?;
        let chr_bytes = rom_size(header[5], high >> 4, 8192)?;
        if prg_bytes == 0 {
            return Err("Cartridge has no PRG ROM".into());
        }
        let trainer = header[6] & 4 != 0;
        let expected = 16usize
            .checked_add(if trainer { 512 } else { 0 })
            .and_then(|size| size.checked_add(prg_bytes))
            .and_then(|size| size.checked_add(chr_bytes))
            .ok_or("Cartridge size overflow")?;
        if rom.len() < expected {
            return Err(format!(
                "Truncated cartridge: expected at least {expected} bytes, got {}",
                rom.len()
            ));
        }
        let battery = header[6] & 2 != 0;
        let legacy_ram = usize::from(header[8].max(1)) * 8192;
        Ok(Self {
            nes2,
            mapper: u16::from(header[6] >> 4)
                | u16::from(header[7] & 0xf0)
                | if nes2 {
                    u16::from(header[8] & 15) << 8
                } else {
                    0
                },
            submapper: if nes2 { header[8] >> 4 } else { 0 },
            prg_bytes,
            chr_bytes,
            prg_ram_bytes: if nes2 {
                ram_size(header[10] & 15)
            } else if battery {
                0
            } else {
                legacy_ram
            },
            prg_nvram_bytes: if nes2 {
                ram_size(header[10] >> 4)
            } else if battery {
                legacy_ram
            } else {
                0
            },
            chr_ram_bytes: if nes2 {
                ram_size(header[11] & 15)
            } else if chr_bytes == 0 {
                8192
            } else {
                0
            },
            chr_nvram_bytes: if nes2 { ram_size(header[11] >> 4) } else { 0 },
            trainer,
            vertical_mirroring: header[6] & 1 != 0,
            four_screen: header[6] & 8 != 0,
            battery,
            timing: if nes2 { header[12] & 3 } else { header[9] & 1 },
            console: header[7] & 3,
            console_detail: if nes2 { header[13] } else { 0 },
            misc_roms: if nes2 { header[14] & 3 } else { 0 },
            expansion_device: if nes2 { header[15] & 63 } else { 0 },
        })
    }
}

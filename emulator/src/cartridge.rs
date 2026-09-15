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

impl CartridgeInfo {
    pub fn parse(rom: &[u8]) -> Result<Self, String> {
        let header = rom.get(..16).ok_or("Truncated iNES header")?;
        if &header[..4] != b"NES\x1a" {
            return Err("Invalid iNES signature".into());
        }
        if matches!(header[7] & 12, 4 | 12) {
            return Err("Unsupported or corrupted iNES header variant".into());
        }
        let nes2 = header[7] & 12 == 8;
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
            misc_roms: if nes2 { header[14] & 3 } else { 0 },
            expansion_device: if nes2 { header[15] & 63 } else { 0 },
        })
    }
}

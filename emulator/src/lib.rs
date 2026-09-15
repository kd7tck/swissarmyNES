use std::cell::RefCell;
pub mod cartridge;
use std::rc::Rc;
use tetanes_core::common::{NesRegion, Regional, Reset, ResetKind};
use tetanes_core::control_deck::ControlDeck;
use tetanes_core::input::{JoypadBtn, Player};
use tetanes_core::mapper::{MapRead, MappedRead};
use tetanes_core::mem::{Read, Write};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct CpuState {
    pub pc: u16,
    pub sp: u8,
    pub acc: u8,
    pub x: u8,
    pub y: u8,
    pub status: u8,
    pub cycles: usize,
}

#[wasm_bindgen]
pub struct Emulator {
    deck: Rc<RefCell<ControlDeck>>,
    breakpoints: Vec<(u8, u16)>,
    stopped_at: Option<(u8, u16)>,
    sample_rate: Option<f32>,
    // Visualization buffers
    pattern_table_buffer: Vec<u8>,
    nametable_buffer: Vec<u8>,
    palette_buffer: Vec<u8>,
}

#[wasm_bindgen]
impl Emulator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Emulator {
        let deck = ControlDeck::new();
        Emulator {
            deck: Rc::new(RefCell::new(deck)),
            breakpoints: Vec::new(),
            stopped_at: None,
            sample_rate: None,
            // 256x128 * 4 bytes (RGBA)
            pattern_table_buffer: vec![0; 256 * 128 * 4],
            // 512x480 * 4 bytes (RGBA) - showing full 2x2 nametable space
            nametable_buffer: vec![0; 512 * 480 * 4],
            // The PPU exposes 32 palette entries as a 16-by-2 RGBA image.
            palette_buffer: vec![0; 32 * 4],
            // OAM: load_oam likely draws sprites. 256x240?
        }
    }

    pub fn load_rom(&mut self, rom_data: &[u8]) -> Result<(), String> {
        let metadata = cartridge::CartridgeInfo::parse(rom_data)?;
        if metadata.console != 0 || metadata.misc_roms != 0 {
            return Err("Unsupported cartridge console or miscellaneous ROM layout".into());
        }
        let mut backend_rom = rom_data.to_vec();
        if metadata.nes2 {
            // The pinned backend understands linear bank counts, but not exponent fields.
            if metadata.prg_bytes % 16384 != 0 || metadata.chr_bytes % 8192 != 0 {
                return Err("Unsupported mapper ROM geometry: partial PRG/CHR banks".into());
            }
            let prg = metadata.prg_bytes / 16384;
            let chr = metadata.chr_bytes / 8192;
            if prg > 0xeff || chr > 0xeff {
                return Err("ROM exceeds backend linear bank capacity".into());
            }
            backend_rom[4] = prg as u8;
            backend_rom[5] = chr as u8;
            backend_rom[9] = ((prg >> 8) | ((chr >> 8) << 4)) as u8;
        }
        if metadata.trainer {
            backend_rom.drain(16..528);
            backend_rom[6] &= !4;
        }
        // A malformed replacement must not destroy the currently running game.
        let mut replacement = ControlDeck::new();
        if let Some(rate) = self.sample_rate {
            replacement.set_sample_rate(rate);
        }
        replacement
            .load_rom("game.nes", &mut &backend_rom[..])
            .map_err(|error| format!("Failed to load ROM: {error:?}"))?;
        if metadata.trainer {
            for (offset, value) in rom_data[16..528].iter().enumerate() {
                replacement
                    .cpu_mut()
                    .bus
                    .write(0x7000 + offset as u16, *value);
            }
            for (offset, value) in rom_data[16..528].iter().enumerate() {
                if replacement.cpu().bus.peek(0x7000 + offset as u16) != *value {
                    return Err("Trainer destination is not writable for this mapper".into());
                }
            }
        }
        *self.deck.borrow_mut() = replacement;
        self.stopped_at = None;
        Ok(())
    }

    pub fn add_breakpoint(&mut self, bank: u8, addr: u16) {
        if !self.breakpoints.contains(&(bank, addr)) {
            self.breakpoints.push((bank, addr));
        }
    }

    pub fn remove_breakpoint(&mut self, bank: u8, addr: u16) {
        if let Some(pos) = self.breakpoints.iter().position(|&x| x == (bank, addr)) {
            self.breakpoints.remove(pos);
        }
    }

    pub fn clear_breakpoints(&mut self) {
        self.breakpoints.clear();
        self.stopped_at = None;
    }

    /// Execute to the next frame or stop immediately before a breakpoint.
    /// Continuing skips only the previously reported instruction, then re-arms it.
    pub fn step(&mut self) -> Result<bool, String> {
        let mut deck = self.deck.borrow_mut();
        let mut skip = self.stopped_at.take();
        let start_frame = deck.frame_number();
        loop {
            let pc = deck.cpu().pc;
            let bank = match deck.ppu().bus.mapper.map_peek(pc) {
                MappedRead::PrgRom(offset) => Some((offset / 16384) as u8),
                _ => None,
            };
            if let Some(bank) = bank {
                let location = (bank, pc);
                if self.breakpoints.contains(&location) && skip != Some(location) {
                    self.stopped_at = Some(location);
                    return Ok(true);
                }
            }
            skip = None;
            deck.clock_instr()
                .map_err(|error| format!("Emulation error: {error:?}"))?;
            if deck.frame_number() != start_frame {
                return Ok(false);
            }
        }
    }

    /// Debug inspection must not advance controller shifts or acknowledge PPU/APU status.
    pub fn peek_cpu(&self, address: u16) -> u8 {
        self.deck.borrow().cpu().bus.peek(address)
    }

    /// Physical PRG offset, excluding the iNES header; -1 denotes non-ROM memory.
    pub fn prg_offset(&self, address: u16) -> i32 {
        match self.deck.borrow().ppu().bus.mapper.map_peek(address) {
            MappedRead::PrgRom(offset) => offset as i32,
            _ => -1,
        }
    }
    pub fn reset(&mut self) {
        let mut deck = self.deck.borrow_mut();
        deck.reset(ResetKind::Hard);
        self.stopped_at = None;
    }

    /// Capture pre-instruction state and execute exactly one instruction on the production core.
    pub fn trace_instruction(&mut self) -> Result<CpuState, String> {
        let state = self.get_cpu_state();
        self.stopped_at = None;
        self.deck
            .borrow_mut()
            .clock_instr()
            .map_err(|error| format!("Emulation error: {error:?}"))?;
        Ok(state)
    }

    /// Nominal frame cadence from the active backend region and CPU clock.
    pub fn frame_rate(&self) -> f64 {
        let deck = self.deck.borrow();
        let (ppu_per_cpu, frame_dots) = match deck.region() {
            // NTSC skips one dot on alternate rendered frames.
            NesRegion::Auto | NesRegion::Ntsc => (3.0, 341.0 * 262.0 - 0.5),
            NesRegion::Pal => (16.0 / 5.0, 341.0 * 312.0),
            NesRegion::Dendy => (3.0, 341.0 * 312.0),
        };
        f64::from(deck.clock_rate()) * ppu_per_cpu / frame_dots
    }

    pub fn set_sample_rate(&mut self, rate: f32) {
        if !rate.is_finite() || rate <= 0.0 {
            return;
        }
        self.sample_rate = Some(rate);
        let mut deck = self.deck.borrow_mut();
        deck.set_sample_rate(rate);
    }

    pub fn get_pixels(&self) -> *const u8 {
        let mut deck = self.deck.borrow_mut();
        deck.frame_buffer().as_ptr()
    }

    pub fn get_pixels_len(&self) -> usize {
        let mut deck = self.deck.borrow_mut();
        deck.frame_buffer().len()
    }

    pub fn get_audio_samples(&self) -> *const f32 {
        let deck = self.deck.borrow();
        deck.audio_samples().as_ptr()
    }

    pub fn get_audio_samples_len(&self) -> usize {
        let deck = self.deck.borrow();
        deck.audio_samples().len()
    }

    pub fn clear_audio_samples(&mut self) {
        let mut deck = self.deck.borrow_mut();
        deck.clear_audio_samples();
    }

    pub fn set_button(&mut self, player: usize, button: u8, pressed: bool) {
        let mut deck = self.deck.borrow_mut();
        let p = if player == 0 {
            Player::One
        } else {
            Player::Two
        };
        let btn = match button {
            0 => JoypadBtn::A,
            1 => JoypadBtn::B,
            2 => JoypadBtn::Select,
            3 => JoypadBtn::Start,
            4 => JoypadBtn::Up,
            5 => JoypadBtn::Down,
            6 => JoypadBtn::Left,
            7 => JoypadBtn::Right,
            _ => return,
        };
        deck.joypad_mut(p).set_button(btn, pressed);
    }

    pub fn get_cpu_state(&self) -> CpuState {
        let deck = self.deck.borrow();
        let cpu = deck.cpu();
        CpuState {
            pc: cpu.pc,
            sp: cpu.sp,
            acc: cpu.acc,
            x: cpu.x,
            y: cpu.y,
            // B and U are stack serialization bits, not physical CPU flags.
            // Publish the nestest/debugger convention (U=1, B=0), independent
            // of the backend's different PLP and RTI internal representations.
            status: (cpu.status.bits() & !0x10) | 0x20,
            cycles: cpu.cycle as usize,
        }
    }

    pub fn get_wram(&self) -> *const u8 {
        let deck = self.deck.borrow();
        deck.wram().as_ptr()
    }

    pub fn get_wram_len(&self) -> usize {
        let deck = self.deck.borrow();
        deck.wram().len()
    }

    /// Owned snapshots remain valid across execution and WASM memory growth.
    pub fn ram_snapshot(&self) -> Vec<u8> {
        self.deck.borrow().wram().to_vec()
    }

    // --- PPU DEBUGGING ---

    pub fn update_pattern_tables(&mut self) {
        let deck = self.deck.borrow();
        deck.ppu()
            .load_pattern_tables(&mut self.pattern_table_buffer);
    }

    pub fn get_pattern_tables(&self) -> *const u8 {
        self.pattern_table_buffer.as_ptr()
    }

    pub fn get_pattern_tables_len(&self) -> usize {
        self.pattern_table_buffer.len()
    }

    pub fn update_nametables(&mut self) {
        let deck = self.deck.borrow();
        deck.ppu().load_nametables(&mut self.nametable_buffer);
    }

    pub fn get_nametables(&self) -> *const u8 {
        self.nametable_buffer.as_ptr()
    }

    pub fn get_nametables_len(&self) -> usize {
        self.nametable_buffer.len()
    }

    pub fn update_palettes(&mut self) {
        let deck = self.deck.borrow();
        // One raw color index per entry; avoid allocating on each viewer refresh.
        let mut scratch = [0u8; 32];
        deck.ppu()
            .load_palettes(&mut self.palette_buffer, &mut scratch);
    }

    pub fn get_palettes(&self) -> *const u8 {
        self.palette_buffer.as_ptr()
    }

    pub fn get_palettes_len(&self) -> usize {
        self.palette_buffer.len()
    }

    // Raw OAM Data Access
    pub fn get_oam_data(&self) -> *const u8 {
        let deck = self.deck.borrow();
        // deck.ppu().oamdata is ConstSlice<u8, 256>
        let ptr = deck.ppu().oamdata.as_ptr();
        ptr
    }

    pub fn get_oam_data_len(&self) -> usize {
        256
    }
}

impl Default for Emulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod palette_tests {
    use super::Emulator;

    #[test]
    fn palette_export_has_exactly_32_opaque_rgba_entries() {
        let mut emulator = Emulator::new();
        emulator.update_palettes();
        assert_eq!(emulator.get_palettes_len(), 128);
        assert!(emulator
            .palette_buffer
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| pixel[3] == 255));
    }
}

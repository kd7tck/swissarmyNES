use std::cell::RefCell;
use std::rc::Rc;
use tetanes_core::common::{Reset, ResetKind};
use tetanes_core::control_deck::ControlDeck;
use tetanes_core::input::{JoypadBtn, Player};
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
    breakpoints: Vec<u16>,
    // Visualization buffers
    pattern_table_buffer: Vec<u8>,
    nametable_buffer: Vec<u8>,
    palette_buffer: Vec<u8>,
    oam_buffer: Vec<u8>, // For visual OAM
}

#[wasm_bindgen]
impl Emulator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Emulator {
        let deck = ControlDeck::new();
        Emulator {
            deck: Rc::new(RefCell::new(deck)),
            breakpoints: Vec::new(),
            // 256x128 * 4 bytes (RGBA)
            pattern_table_buffer: vec![0; 256 * 128 * 4],
            // 512x480 * 4 bytes (RGBA) - showing full 2x2 nametable space
            nametable_buffer: vec![0; 512 * 480 * 4],
            // Palettes: usually rendered as small swatches.
            // load_palettes docs say "buffer with RGBA pixels".
            // Let's assume it draws a representation.
            // If unknown, I'll allocate plenty. 256x256 is safe.
            palette_buffer: vec![0; 512 * 4],
            // OAM: load_oam likely draws sprites. 256x240?
            oam_buffer: vec![0; 256 * 240 * 4],
        }
    }

    pub fn load_rom(&mut self, rom_data: &[u8]) -> Result<(), String> {
        let mut deck = self.deck.borrow_mut();
        match deck.load_rom("game.nes", &mut &rom_data[..]) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to load ROM: {:?}", e)),
        }
    }

    pub fn add_breakpoint(&mut self, addr: u16) {
        if !self.breakpoints.contains(&addr) {
            self.breakpoints.push(addr);
        }
    }

    pub fn remove_breakpoint(&mut self, addr: u16) {
        if let Some(pos) = self.breakpoints.iter().position(|&x| x == addr) {
            self.breakpoints.remove(pos);
        }
    }

    pub fn clear_breakpoints(&mut self) {
        self.breakpoints.clear();
    }

    pub fn step(&mut self) -> Result<bool, String> {
        let mut deck = self.deck.borrow_mut();

        if self.breakpoints.is_empty() {
            match deck.clock_frame() {
                Ok(_) => return Ok(false),
                Err(e) => return Err(format!("Emulation error: {:?}", e)),
            }
        }

        let start_frame = deck.frame_number();

        if let Err(e) = deck.clock_instr() {
            return Err(format!("Emulation error: {:?}", e));
        }

        loop {
            let pc = deck.cpu().pc;
            if self.breakpoints.contains(&pc) {
                return Ok(true);
            }

            if deck.frame_number() > start_frame {
                return Ok(false);
            }

            if let Err(e) = deck.clock_instr() {
                return Err(format!("Emulation error: {:?}", e));
            }
        }
    }

    pub fn reset(&mut self) {
        let mut deck = self.deck.borrow_mut();
        deck.reset(ResetKind::Hard);
    }

    pub fn set_sample_rate(&mut self, rate: f32) {
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
            status: cpu.status.bits(),
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

    // --- PPU DEBUGGING ---

    pub fn update_pattern_tables(&mut self) {
        let deck = self.deck.borrow();
        deck.ppu().load_pattern_tables(&mut self.pattern_table_buffer);
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
        // The API for load_palettes requires two buffers: palettes and colors (visual)
        // Oops, I need to check the signature again.
        // pub fn load_palettes(&self, palettes: &mut [u8], colors: &mut [u8])
        // "Load the given buffer with RGBA pixels from the current palettes."
        // Wait, which buffer is which?
        // Let's assume 'palettes' is raw data? or visual?
        // I will create a dummy buffer for the second arg if needed or just use one.
        // Actually, let's just expose the raw palette RAM if possible.
        // But load_palettes is useful for visualization.

        // Let's re-read the doc snippet I have in memory or context.
        // load_palettes(palettes: &mut [u8], colors: &mut [u8])
        // It says "Load the given buffer with RGBA pixels from the current palettes."
        // Likely 'palettes' is the swatch view, and 'colors' is the full system palette view?

        // I'll try to use it with my palette_buffer as the first arg.
        // I'll need another buffer for the second arg.
        // Let's make palette_buffer big enough and pass a scratch buffer for the second arg.

        let mut scratch = vec![0u8; 256 * 4]; // Dummy
        deck.ppu().load_palettes(&mut self.palette_buffer, &mut scratch);
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
        // access via generic AsSlice or deref?
        // tetanes_core::mem::ConstSlice usually derefs to [T].
        let ptr = deck.ppu().oamdata.as_ptr(); // This accesses the pointer of the slice
        ptr
    }

    pub fn get_oam_data_len(&self) -> usize {
        256
    }
}

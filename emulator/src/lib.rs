use std::cell::RefCell;
use std::rc::Rc;
use tetanes_core::control_deck::ControlDeck;
use tetanes_core::input::{JoypadBtn, Player};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Emulator {
    deck: Rc<RefCell<ControlDeck>>,
}

#[wasm_bindgen]
impl Emulator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Emulator {
        let deck = ControlDeck::new();
        Emulator {
            deck: Rc::new(RefCell::new(deck)),
        }
    }

    pub fn load_rom(&mut self, rom_data: &[u8]) -> Result<(), String> {
        let mut deck = self.deck.borrow_mut();
        match deck.load_rom("game.nes", &mut &rom_data[..]) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to load ROM: {:?}", e)),
        }
    }

    pub fn step(&mut self) -> Result<(), String> {
        let mut deck = self.deck.borrow_mut();
        match deck.clock_frame() {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Emulation error: {:?}", e)),
        }
    }

    pub fn get_pixels(&self) -> *const u8 {
        let mut deck = self.deck.borrow_mut();
        deck.frame_buffer().as_ptr()
    }

    pub fn get_pixels_len(&self) -> usize {
        let mut deck = self.deck.borrow_mut();
        deck.frame_buffer().len()
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
}

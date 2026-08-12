use rand::RngExt;
use raylib::prelude::*;
use raylib::{
    drawing::RaylibDrawHandle,
    ffi::{Color, KeyboardKey},
};
use std::{
    fs,
    time::{Duration, Instant},
};

use rand::rngs::ThreadRng;

use crate::font::FONTS;

pub const SCREEN_HEIGHT: usize = 32;
pub const SCREEN_WIDTH: usize = 64;
pub const PROGRAM_START_ADDR: u16 = 0x200;
pub const FONTS_START_ADDR: u16 = 0x050;

pub struct Chip8 {
    ram: [u8; 4096],                                    // RAM
    pub display: [[bool; SCREEN_WIDTH]; SCREEN_HEIGHT], // VRAM
    gp_registers: [u8; 16],                             // General Purpose Registers
    pc: u16,                                            // Program Counter
    index_register: u16,                                // I Register
    stack: Vec<u16>,                                    // Routine Stack
    delay_timer: u8,                                    // Delay Timer, decrements at 60Hz
    sound_timer: u8,                                    // Sound Timer, Not Implemented

    last_delay_updated: Instant,        // Timer to track delay timer
    random_number_generator: ThreadRng, // Random Number Generator
}

impl Chip8 {
    pub fn new(rom_path: &String) -> Self {
        let mut chip8 = Chip8 {
            ram: [0; 4096],
            display: [[false; SCREEN_WIDTH]; SCREEN_HEIGHT],
            gp_registers: [0; 16],
            pc: 0,
            index_register: 0,
            stack: vec![],
            delay_timer: 0,
            sound_timer: 0,
            last_delay_updated: Instant::now(),
            random_number_generator: rand::rng(),
        };
        chip8.load_fonts();
        let bytes = fs::read(rom_path).expect("ROM path not found.");
        chip8.load_rom(&bytes);

        chip8
    }

    fn load_rom(&mut self, bytes: &Vec<u8>) {
        for i in 0..bytes.len() {
            self.ram[PROGRAM_START_ADDR as usize + i] = bytes[i];
        }
    }

    fn load_fonts(&mut self) {
        for i in 0..16 {
            for j in 0..5 {
                self.ram[FONTS_START_ADDR as usize + i + j] = FONTS[i][j];
            }
        }
    }

    fn get_instruction_and_increment_pc(&mut self) -> u16 {
        let ins: u16 =
            (self.ram[self.pc as usize] as u16) << 8 | self.ram[self.pc as usize + 1] as u16;
        self.increment_pc();

        return ins;
    }

    fn clear_display(&mut self) {
        for row in &mut self.display {
            row.fill(false);
        }
    }

    fn increment_pc(&mut self) {
        self.pc += 2;
    }

    pub fn fde_cycle(&mut self, rl: &mut RaylibDrawHandle) {
        let ins = self.get_instruction_and_increment_pc();

        match (ins & 0xF000) >> 12 {
            0x0 => {
                match ins & 0x00FF {
                    0xE0 => {
                        // clear
                        self.clear_display();
                    }
                    0xEE => {
                        // return
                        self.pc = self.stack.pop().unwrap();
                    }
                    _ => {}
                }
            }
            0x1 => {
                // jump NNN
                self.pc = ins & 0x0FFF;
            }
            0x2 => {
                // Call subroutine at NNN
                self.stack.push(self.pc);
                self.pc = ins & 0x0FFF;
            }
            0x3 => {
                // if vx != NN then
                let register_value = self.gp_registers[((0x0F00 & ins) >> 8) as usize] as u16;
                let compare_value = 0x00FF & ins;

                if register_value == compare_value {
                    self.increment_pc();
                }
            }
            0x4 => {
                // if vx == NN then
                let register_value = self.gp_registers[((0x0F00 & ins) >> 8) as usize] as u16;
                let compare_value = 0x00FF & ins;

                if register_value != compare_value {
                    self.increment_pc();
                }
            }
            0x5 => {
                // if vx != vy then
                let r1 = ((0x0F00 & ins) >> 8) as usize;
                let r2 = ((0x00F0 & ins) >> 4) as usize;

                if self.gp_registers[r1] == self.gp_registers[r2] {
                    self.increment_pc();
                }
            }
            0x6 => {
                // SET GP
                let reg = (ins & 0x0F00) >> 8;
                self.gp_registers[reg as usize] = (ins & 0x00FF) as u8;
            }
            0x7 => {
                // ADD GP
                let reg = (ins & 0x0F00) >> 8;
                self.gp_registers[reg as usize] =
                    self.gp_registers[reg as usize].wrapping_add((ins & 0x00FF) as u8)
            }
            0x8 => {
                let x_reg = ((ins & 0x0F00) >> 8) as usize;
                let y_reg = ((ins & 0x00F0) >> 4) as usize;

                match ins & 0x000F {
                    0x0 => {
                        // vx := vy
                        self.gp_registers[x_reg] = self.gp_registers[y_reg];
                    }
                    0x1 => {
                        // vx |= vy
                        self.gp_registers[x_reg] |= self.gp_registers[y_reg];
                    }
                    0x2 => {
                        // vx &= vy
                        self.gp_registers[x_reg] &= self.gp_registers[y_reg];
                    }
                    0x3 => {
                        // vx ^= vy
                        self.gp_registers[x_reg] ^= self.gp_registers[y_reg];
                    }
                    0x4 => {
                        // vx += vy, vf = 1 on carry
                        let (sum, overflowed) =
                            self.gp_registers[x_reg].overflowing_add(self.gp_registers[y_reg]);

                        self.gp_registers[x_reg] = sum;

                        if overflowed {
                            self.gp_registers[0xF] = 1;
                        } else {
                            self.gp_registers[0xF] = 0;
                        }
                    }
                    0x5 => {
                        // vx -= vy, vf = 0 on borrow
                        let (res, overflowed) =
                            self.gp_registers[x_reg].overflowing_sub(self.gp_registers[y_reg]);
                        self.gp_registers[x_reg] = res;

                        if overflowed {
                            self.gp_registers[0xF] = 0;
                        } else {
                            self.gp_registers[0xF] = 1;
                        }
                    }
                    0x6 => {
                        // vx >>= vy, vf = old least significant bit
                        let old_bit = self.gp_registers[y_reg] & 1;
                        self.gp_registers[x_reg] = self.gp_registers[y_reg] >> 1;
                        self.gp_registers[0xF] = old_bit;
                    }
                    0x7 => {
                        // vx =- vy, vf = 0 on borrow
                        let (res, overflowed) =
                            self.gp_registers[y_reg].overflowing_sub(self.gp_registers[x_reg]);
                        self.gp_registers[x_reg] = res;

                        if overflowed {
                            self.gp_registers[0xF] = 0;
                        } else {
                            self.gp_registers[0xF] = 1;
                        }
                    }
                    0xE => {
                        // vx <<= vy, vf = old most significant bit
                        let old_bit = (self.gp_registers[y_reg] & 0x80) >> 6;
                        self.gp_registers[x_reg] = self.gp_registers[y_reg] << 1;
                        self.gp_registers[0xF] = old_bit;
                    }
                    _ => {}
                }
            }
            0x9 => {
                // if vx == vy then
                let r1 = ((0x0F00 & ins) >> 8) as usize;
                let r2 = ((0x00F0 & ins) >> 4) as usize;

                if self.gp_registers[r1] != self.gp_registers[r2] {
                    self.increment_pc();
                }
            }
            0xA => {
                // SET IDX
                self.index_register = ins & (0x0FFF);
            }
            0xC => {
                // vx := random NN
                let random_number: u8 = self.random_number_generator.random_range(0..=255);
                let register = ((ins & 0x0F00) >> 8) as usize;
                self.gp_registers[register] = ((ins & 0x00FF) as u8) & random_number;
            }
            0xD => {
                // DRAW
                let x_reg = ((ins & 0x0F00) >> 8) as usize;
                let y_reg = ((ins & 0x00F0) >> 4) as usize;
                let offset = (ins & 0x000F) as u8;

                self.gp_registers[0xF] = 0;

                for i in 0..offset {
                    let sprite = self.ram[(self.index_register + i as u16) as usize];

                    for j in 0..8 {
                        let x = (self.gp_registers[x_reg] as usize + j as usize) % 64;
                        let y = (self.gp_registers[y_reg] as usize + i as usize) % 32;

                        let bit = (sprite >> (7 - j)) & 1;

                        if self.display[y][x] && bit == 1 {
                            self.gp_registers[0xF] = 1;
                        }

                        self.display[y][x] ^= bit != 0;
                    }
                }
            }
            0xE => {
                let register = ((ins & 0x0F00) >> 8) as usize;

                match ins & 0x000F {
                    0xE => {
                        if let Some(key) = Chip8::reverse_key_map(self.gp_registers[register]) {
                            if rl.is_key_down(key) {
                                self.increment_pc();
                            }
                        }
                    }
                    0x1 => {
                        if let Some(key) = Chip8::reverse_key_map(self.gp_registers[register]) {
                            if !rl.is_key_down(key) {
                                self.increment_pc();
                            }
                        }
                    }
                    _ => {}
                }
            }
            0xF => {
                let register = ((ins & 0x0F00) >> 8) as usize;

                match ins & 0x00FF {
                    0x7 => {
                        if self.last_delay_updated.elapsed() < Duration::from_millis(60) {
                            self.gp_registers[register] = std::cmp::max(
                                self.delay_timer as i16
                                    - (Duration::from_millis(60)
                                        - self.last_delay_updated.elapsed())
                                    .as_millis() as i16,
                                0,
                            ) as u8;
                        } else {
                            self.gp_registers[register] = 0;
                        }
                    }
                    0x15 => {
                        // delay := vx
                        self.delay_timer = self.gp_registers[register];
                        self.last_delay_updated = Instant::now();
                    }
                    0x18 => {
                        // buzzer := vx
                        self.sound_timer = self.gp_registers[register];
                    }
                    0x1E => {
                        // i += vx
                        let (sum, overflowed) = self
                            .index_register
                            .overflowing_add(self.gp_registers[register] as u16);

                        self.index_register = sum;

                        if overflowed {
                            self.gp_registers[0xF] = 1;
                        }
                    }
                    0x29 => {
                        // i := hex vx, Set i to a hex character
                        self.index_register =
                            (FONTS_START_ADDR as u16) + ((self.gp_registers[register] * 5) as u16);
                    }
                    0x33 => {
                        // bcd vx, Decode vx into binary-coded decimal
                        let mut number = self.gp_registers[register];

                        for i in (0..3).rev() {
                            self.ram[self.index_register as usize + i] = number % 10;
                            number /= 10;
                        }
                    }
                    0x55 => {
                        // save vx, Save v0-vx to i through (i+x)
                        for i in 0..=register {
                            self.ram[self.index_register as usize + i] = self.gp_registers[i];
                        }
                    }
                    0x65 => {
                        //load vx, Load v0-vx from i through (i+x)
                        for i in 0..=register {
                            self.gp_registers[i] =
                                self.ram[(self.index_register + i as u16) as usize];
                        }
                    }
                    0x0A => {
                        if let Some(pressed_key) = rl.get_key_pressed() {
                            while rl.is_key_released(pressed_key) {
                                if let Some(key) = Chip8::key_map(pressed_key) {
                                    self.gp_registers[register] = key;
                                    break;
                                }
                            }
                        } else {
                            self.pc -= 2;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        self.draw(rl);
    }

    fn draw(&self, d: &mut RaylibDrawHandle) {
        d.clear_background(Color::WHITE);

        for i in 0..32 {
            for j in 0..64 {
                if self.display[i][j] {
                    d.draw_rectangle(j as i32 * 10, i as i32 * 10, 10, 10, Color::BLACK);
                }
            }
        }
    }

    fn reverse_key_map(key: u8) -> Option<KeyboardKey> {
        match key {
            1 => Some(KeyboardKey::KEY_ONE),
            2 => Some(KeyboardKey::KEY_TWO),
            3 => Some(KeyboardKey::KEY_THREE),
            12 => Some(KeyboardKey::KEY_FOUR),

            4 => Some(KeyboardKey::KEY_Q),
            5 => Some(KeyboardKey::KEY_W),
            6 => Some(KeyboardKey::KEY_E),
            13 => Some(KeyboardKey::KEY_R),

            7 => Some(KeyboardKey::KEY_A),
            8 => Some(KeyboardKey::KEY_S),
            9 => Some(KeyboardKey::KEY_D),
            14 => Some(KeyboardKey::KEY_F),

            10 => Some(KeyboardKey::KEY_Z),
            0 => Some(KeyboardKey::KEY_X),
            11 => Some(KeyboardKey::KEY_C),
            15 => Some(KeyboardKey::KEY_V),

            _ => None,
        }
    }

    fn key_map(key: KeyboardKey) -> Option<u8> {
        return match key {
            KeyboardKey::KEY_ONE => Some(1),
            KeyboardKey::KEY_TWO => Some(2),
            KeyboardKey::KEY_THREE => Some(3),
            KeyboardKey::KEY_FOUR => Some(12),

            KeyboardKey::KEY_Q => Some(4),
            KeyboardKey::KEY_W => Some(5),
            KeyboardKey::KEY_E => Some(6),
            KeyboardKey::KEY_R => Some(13),

            KeyboardKey::KEY_A => Some(7),
            KeyboardKey::KEY_S => Some(8),
            KeyboardKey::KEY_D => Some(9),
            KeyboardKey::KEY_F => Some(14),

            KeyboardKey::KEY_Z => Some(10),
            KeyboardKey::KEY_X => Some(0),
            KeyboardKey::KEY_C => Some(11),
            KeyboardKey::KEY_V => Some(15),
            _ => None,
        };
    }
}

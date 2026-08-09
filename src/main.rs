use std::time::{Duration, Instant};
use std::thread;
use rand;
use rand::RngExt;
use raylib::prelude::*;
use std::fs;

const FONTS: [[u8; 5]; 16] = [
    [0xF0, 0x90, 0x90, 0x90, 0xF0], // 0
    [0x20, 0x60, 0x20, 0x20, 0x70], // 1
    [0xF0, 0x10, 0xF0, 0x80, 0xF0], // 2
    [0xF0, 0x10, 0xF0, 0x10, 0xF0], // 3
    [0x90, 0x90, 0xF0, 0x10, 0x10], // 4
    [0xF0, 0x80, 0xF0, 0x10, 0xF0], // 5
    [0xF0, 0x80, 0xF0, 0x90, 0xF0], // 6
    [0xF0, 0x10, 0x20, 0x40, 0x40], // 7
    [0xF0, 0x90, 0xF0, 0x90, 0xF0], // 8
    [0xF0, 0x90, 0xF0, 0x10, 0xF0], // 9
    [0xF0, 0x90, 0xF0, 0x90, 0x90], // A
    [0xE0, 0x90, 0xE0, 0x90, 0xE0], // B
    [0xF0, 0x80, 0x80, 0x80, 0xF0], // C
    [0xE0, 0x90, 0x90, 0x90, 0xE0], // D
    [0xF0, 0x80, 0xF0, 0x80, 0xF0], // E
    [0xF0, 0x80, 0xF0, 0x80, 0x80], // F
];

const FONT_ADDR: usize = 0x50;
const ROM_START_ADDR: usize = 0x200;

fn instruction_not_implemented(ins: u16) {
    panic!("Instruction {:#x} not implemented", ins);
}

fn clear_display(d: &mut RaylibDrawHandle) {
   d.clear_background(Color::WHITE);
}

fn draw(d: &mut RaylibDrawHandle, display: &Vec<Vec<bool>>) {
    d.clear_background(Color::WHITE);

    for i in 0..32 {
        for j in 0..64 {
            if display[i][j] {
                d.draw_rectangle((j as i32 * 10), (i as i32 * 10), 10, 10, Color::BLACK);
            }
        }
    }
}

fn chip_8() {
    let (mut rl, thread) = raylib::init()
    .size(640, 320)
    .title("CHIP-8")
    .build();

    let mut ram: Vec<u8> = vec![0; 0xfff]; // VM RAM
    let mut display: Vec<Vec<bool>> = vec![vec![false; 64]; 32]; // Display
    let mut gp_registers: Vec<u8> = vec![0; 16]; // General Purpose Registers
    let mut pc: u16 = ROM_START_ADDR as u16; // Program Counter
    let mut index_register: u16 = 0; // I Register
    let mut stack: Vec<u16> = vec![]; // Routine Stack
    let mut rng = rand::rng(); // Random Number generator
    let mut delay_timer: u8 = 0;
    let mut sound_timer: u8 = 0;

    const InstructionsPerSecond: u32 = 100;
    const NanoPerInstruction: u32 = 1_000_000_000 / InstructionsPerSecond;

    let target_duration = Duration::new(0, NanoPerInstruction);
    let mut last_time = Instant::now();

    let args: Vec<String> = std::env::args().collect();
    
    // Read the ROM from args
    let bytes = fs::read(&args[1]).unwrap();
    
    // Load the ROM into RAM
    for i in 0..bytes.len() {
        ram[ROM_START_ADDR + i] = bytes[i];
    }

    // load fonts into ram
    for i in 0..16 {
        for j in 0..5 {
            ram[FONT_ADDR + i + j] = FONTS[i][j];
        }
    }

    while !rl.window_should_close() {
        let mut draw_handle = rl.begin_drawing(&thread);
        let start_time = Instant::now();

        let first = ram[pc as usize];
        let second = ram[(pc + 1) as usize];

        pc += 2;

        let ins: u16 = ((first as u16) << 8) | second as u16;

        match (ins & 0xF000) >> 12 {
            0x0 => {
                match ins & 0x00FF {
                    0xE0 => {
                        // clear
                        clear_display(&mut draw_handle);
                    }
                    0xEE => {
                        // return
                        pc = stack.pop().unwrap();
                    }
                    _ => {
                    }
                }
            }
            0x1 => {
                // jump NNN
                pc = ins & 0x0FFF;
            }
            0x2 => {
                // Call subroutine at NNN
                stack.push(pc);
                pc = ins & 0x0FFF;
            }
            0x3 => {
                // if vx != NN then
                let register_value = gp_registers[((0x0F00 & ins) >> 8) as usize] as u16;
                let compare_value = 0x00FF & ins;

                if register_value == compare_value {
                    pc += 2;
                }
            }
            0x4 => {
                // if vx == NN then
                let register_value = gp_registers[((0x0F00 & ins) >> 8) as usize] as u16;
                let compare_value = 0x00FF & ins;

                if register_value != compare_value {
                    pc += 2;
                }
            }
            0x5 => {
                // if vx != vy then
                let r1 = ((0x0F00 & ins) >> 8) as usize;
                let r2 = ((0x00F0 & ins) >> 4) as usize;

                if gp_registers[r1] == gp_registers[r2] {
                    pc += 2;
                }
            }
            0x6 => {
                // SET GP
                let reg = (ins & 0x0F00) >> 8;
                gp_registers[reg as usize] = (ins & 0x00FF) as u8;
            }
            0x7 => {
                // ADD GP
                let reg = (ins & 0x0F00) >> 8;
                gp_registers[reg as usize] =
                    gp_registers[reg as usize].wrapping_add((ins & 0x00FF) as u8)
            }
            0x8 => {
                let x_reg = ((ins & 0x0F00) >> 8) as usize;
                let y_reg = ((ins & 0x00F0) >> 4) as usize;

                match ins & 0x000F {
                    0x0 => {
                        // vx := vy
                        gp_registers[x_reg] = gp_registers[y_reg];
                    }
                    0x1 => {
                        // vx |= vy
                        gp_registers[x_reg] |= gp_registers[y_reg];
                    }
                    0x2 => {
                        // vx &= vy
                        gp_registers[x_reg] &= gp_registers[y_reg];
                    }
                    0x3 => {
                        // vx ^= vy
                        gp_registers[x_reg] ^= gp_registers[y_reg];
                    }
                    0x4 => {
                        // vx += vy, vf = 1 on carry
                        let (sum, overflowed) =
                            gp_registers[x_reg].overflowing_add(gp_registers[y_reg]);

                        gp_registers[x_reg] = sum;

                        if overflowed {
                            gp_registers[0xF] = 1;
                        }
                    }
                    0x5 => {
                        // vx -= vy, vf = 0 on borrow
                        let (res, overflowed) =
                            gp_registers[x_reg].overflowing_sub(gp_registers[y_reg]);
                        gp_registers[x_reg] = res;

                        if overflowed {
                            gp_registers[0xF] = 1;
                        }
                    }
                    0x6 => {
                        // vx >>= vy, vf = old least significant bit 
                        let old_bit = gp_registers[y_reg] & 1;
                        gp_registers[x_reg] = gp_registers[y_reg] >> 1;
                        gp_registers[0xF] = old_bit;

                    }
                    0x7 => {
                        // vx =- vy, vf = 0 on borrow
                        let (res, overflowed) =
                            gp_registers[y_reg].overflowing_sub(gp_registers[x_reg]);
                        gp_registers[x_reg] = res;

                        if overflowed {
                            gp_registers[0xF] = 1;
                        }
                    }
                    0xE => {
                        // vx <<= vy, vf = old most significant bit
                        let old_bit = (gp_registers[y_reg] & 0x80) >> 6;
                        gp_registers[x_reg] = gp_registers[y_reg] << 1;
                        gp_registers[0xF] = old_bit;
                    }
                    _ => {
                        instruction_not_implemented(ins);
                    }
                }
            }
            0x9 => {
                // if vx == vy then
                let r1 = ((0x0F00 & ins) >> 8) as usize;
                let r2 = ((0x00F0 & ins) >> 4) as usize;

                if gp_registers[r1] != gp_registers[r2] {
                    pc += 2;
                }
            }
            0xA => {
                // SET IDX
                index_register = ins & (0x0FFF);
            }
            0xC => {
                // vx := random NN
                let random_number: u8 = rng.random_range(0..=255);
                let register = ((ins & 0x0F00) >> 8) as usize;
                gp_registers[register] = ((ins & 0x00FF) as u8) & random_number;
            }
            0xD => {
                // DRAW
                let x_reg = ((ins & 0x0F00) >> 8) as usize;
                let y_reg = ((ins & 0x00F0) >> 4) as usize;
                let offset = (ins & 0x000F) as u8;

                gp_registers[0xF] = 0;

                for i in 0..offset {
                    let sprite = ram[(index_register + i as u16) as usize];

                    for j in 0..8 {
                        let x = (gp_registers[x_reg] as usize + j as usize) % 64;
                        let y = (gp_registers[y_reg] as usize + i as usize) % 32;

                        let bit = (sprite >> (7 - j)) & 1;

                        if(display[y][x] && bit == 1) {
                            gp_registers[0xF] = 1;
                        }

                        display[y][x] ^= bit != 0;
                    }
                }
            }
            0xE => {
            }
            0xF => {
                let register = ((ins & 0x0F00) >> 8) as usize;

                match ins & 0x00FF {
                    0x7 => {
                        gp_registers[register] = delay_timer;
                    }
                    0x15 => {
                        // delay := vx
                        delay_timer = gp_registers[register];
                    }
                    0x18 => {
                        // buzzer := vx
                        sound_timer = gp_registers[register];
                    }
                    0x1E => {
                        // i += vx
                        let (sum, overflowed) =
                            index_register.overflowing_add(gp_registers[register] as u16);
                        
                        index_register = sum;

                        if overflowed {
                            gp_registers[0xF] = 1;
                        }
                    }
                    0x29 => {
                        // i := hex vx, Set i to a hex character
                        index_register = (FONT_ADDR as u16) + ((gp_registers[register] * 5) as u16);
                    }
                    0x33 => {
                        // bcd vx, Decode vx into binary-coded decimal
                        let mut number = gp_registers[register];

                        for i in (0..3).rev() {
                            ram[index_register as usize + i] = number % 10;
                            number /= 10;
                        }
                    }
                    0x55 => {
                        // save vx, Save v0-vx to i through (i+x)
                        for i in 0..=register {
                            ram[index_register as usize + i] = gp_registers[i];
                        }
                    }
                    0x65 => {
                        //load vx, Load v0-vx from i through (i+x) 
                        for i in 0..=register {
                            gp_registers[i] = ram[(index_register + i as u16) as usize]; 
                        }
                    }
                    0x0A => {
                    }
                    _ => {
                        instruction_not_implemented(ins);
                    }
                }
            }
            _ => {
                instruction_not_implemented(ins);
            }
        }

        draw(&mut draw_handle, &display);

        let elapsed = start_time.elapsed();

        if elapsed < target_duration {
            thread::sleep(target_duration - elapsed);
        }
    }
}

fn main() {
    chip_8();
}

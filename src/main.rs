mod chip8;
mod font;
use std::thread;
use std::time::{Duration, Instant};

use raylib::ffi::TraceLogLevel;

use crate::chip8::Chip8;

const INSTRUCTIONS_PER_SECOND: f64 = 700.0;

fn main() {
    raylib::core::logging::set_trace_log(TraceLogLevel::LOG_NONE);

    let (mut rl, thread) = raylib::init().size(640, 320).title("CHIP-8").build();

    let args: Vec<String> = std::env::args().collect();
    let clock_interval: Duration = Duration::from_secs_f64(1.0 / INSTRUCTIONS_PER_SECOND);

    let rom_path = args.get(1).expect("A ROM path is required.");
    let mut chip8 = Chip8::new(&rom_path); // Load ROM specified in cmd line args

    while !rl.window_should_close() {
        let now = Instant::now();

        let mut draw_handle = rl.begin_drawing(&thread);

        chip8.fde_cycle(&mut draw_handle);

        if now.elapsed() < clock_interval {
            thread::sleep(clock_interval - now.elapsed());
        }
    }
}

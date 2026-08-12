# CHIP-8 Emulator

A CHIP-8 virtual machine written in Rust using [raylib](https://www.raylib.com/) for graphics.

## Requirements

- Rust (edition 2024)
- raylib system dependencies (see [raylib-rs build docs](https://github.com/raysan5/raylib))

## Usage

```sh
cargo run -- <path-to-rom>
```

Example:

```sh
cargo run -- "roms/IBM Logo.ch8"
```

## Note

Audio is not implemented.

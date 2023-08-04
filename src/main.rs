pub mod bus;
pub mod cartridge;
pub mod cpu;
pub mod opcodes;
pub mod ppu;
pub mod render;
pub mod tile_viewer;
pub mod trace;
pub mod joypad;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate bitflags;

use std::collections::HashMap;

use bus::Bus;
use cartridge::Rom;
use cpu::CPU;
use joypad::{JoypadButton, Joypad};
use ppu::NesPPU;
use render::frame::Frame;
use sdl2::{event::Event, keyboard::Keycode, pixels::PixelFormatEnum};

fn keymap() -> HashMap<Keycode, JoypadButton> {
    let mut keymap = HashMap::new();
    keymap.insert(Keycode::W, joypad::JoypadButton::UP);
    keymap.insert(Keycode::A, joypad::JoypadButton::LEFT);
    keymap.insert(Keycode::S, joypad::JoypadButton::DOWN);
    keymap.insert(Keycode::D, joypad::JoypadButton::RIGHT);
    keymap.insert(Keycode::Space, joypad::JoypadButton::SELECT);
    keymap.insert(Keycode::Return, joypad::JoypadButton::START);
    keymap.insert(Keycode::Num1, joypad::JoypadButton::A);
    keymap.insert(Keycode::Num2, joypad::JoypadButton::B);
    keymap
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        run(&args[1]);
    } else {
        run("bins/pacman.nes");
    }
}
fn run(rom_path: &str) {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Tile Viewer", (256.0 * 3.0) as u32, (240.0 * 3.0) as u32)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(3.0, 3.0).unwrap();

    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(PixelFormatEnum::RGB24, 256, 240)
        .unwrap();

    // load snake.nes
    let raw_rom: Vec<u8> = std::fs::read(rom_path).expect("Failed to read ROM");
    let cartridge = Rom::new(&raw_rom).expect("Failed to load ROM");

    let mut frame = Frame::new();

    let bus = Bus::new(cartridge, move |ppu: &NesPPU, joypad: &mut Joypad| {
        render::render(ppu, &mut frame);
        texture.update(None, &frame.data, 256 * 3).unwrap();

        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    std::process::exit(0);
                }
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => {
                    if let Some(button) = keymap().get(&keycode) {
                        joypad.press(*button);
                    }
                }
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => {
                    if let Some(button) = keymap().get(&keycode) {
                        joypad.release(*button);
                    }
                }
                _ => {}
            }
        }
        let sleep_time = std::time::Duration::from_millis(10);
        std::thread::sleep(sleep_time);
    });
    let mut cpu = CPU::new(bus);
    cpu.reset();
    cpu.run();
}

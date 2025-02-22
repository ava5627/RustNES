use sdl2::{event::Event, keyboard::Keycode, pixels::PixelFormatEnum};

use crate::{
    cartridge::Rom,
    render::{frame::Frame, palette::SYSTEM_PALLETE},
};

pub fn show_tile(chr_rom: &[u8], bank: usize, tile_n: usize) -> Frame {
    assert!(bank <= 1);

    let mut frame = Frame::new();
    let bank = bank * 0x1000;

    let tile = &chr_rom[(bank + tile_n * 16)..=(bank + tile_n * 16 + 15)];

    for y in 0..=7 {
        let mut upper = tile[y];
        let mut lower = tile[y + 8];

        for x in (0..=7).rev() {
            let color = ((upper & 1) << 1) | (lower & 1);
            upper >>= 1;
            lower >>= 1;

            let rgb = match color {
                0b00 => SYSTEM_PALLETE[0x01],
                0b01 => SYSTEM_PALLETE[0x23],
                0b10 => SYSTEM_PALLETE[0x27],
                0b11 => SYSTEM_PALLETE[0x30],
                _ => panic!(
                    "Color can only be 0b00, 0b01, 0b10 or 0b11. Got 0b{:b}",
                    color
                ),
            };

            frame.set_pixel(x, y, rgb);
        }
    }

    frame
}

pub fn show_tile_bank(chr_rom: &[u8], bank: usize) -> Frame {
    assert!(bank <= 1);

    let mut frame = Frame::new();
    let mut tile_x = 0;
    let mut tile_y = 0;
    let bank = bank * 0x1000;

    for tile_n in 0..255 {
        if tile_n != 0 && tile_n % 20 == 0 {
            tile_y += 10;
            tile_x = 0;
        }

        let tile = &chr_rom[(bank + tile_n * 16)..=(bank + tile_n * 16 + 15)];

        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];

            for x in (0..=7).rev() {
                let color = ((upper & 1) << 1) | (lower & 1);
                upper >>= 1;
                lower >>= 1;

                let rgb = match color {
                    0b00 => SYSTEM_PALLETE[0x01],
                    0b01 => SYSTEM_PALLETE[0x23],
                    0b10 => SYSTEM_PALLETE[0x27],
                    0b11 => SYSTEM_PALLETE[0x30],
                    _ => unreachable!(),
                };

                frame.set_pixel(tile_x + x, tile_y + y, rgb);
            }
        }
        tile_x += 10;
    }

    frame
}

pub fn display_tile_bank(rom_path: &str, bank: usize) {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Tile Viewer", (256.0 * 3.0) as u32, (240.0 * 3.0) as u32)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    // canvas.set_scale(3.0, 3.0).unwrap();

    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(PixelFormatEnum::RGB24, 256, 240)
        .unwrap();

    // load snake.nes
    let raw_rom: Vec<u8> = std::fs::read(rom_path).expect("Failed to read ROM");
    let cartridge = Rom::new(&raw_rom).expect("Failed to load ROM");

    let tile_frame = show_tile_bank(&cartridge.chr_rom, bank);

    texture.update(None, &tile_frame.data, 256 * 3).unwrap();
    canvas.copy(&texture, None, None).unwrap();
    canvas.present();

    loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => std::process::exit(0),
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => std::process::exit(0),
                _ => {}
            }
        }
    }
}

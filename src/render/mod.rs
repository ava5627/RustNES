#![allow(clippy::needless_range_loop)]
use crate::{cartridge::Mirroring, ppu::NesPPU};

use frame::Frame;

use self::palette::SYSTEM_PALLETE;

pub mod frame;
pub mod palette;

fn bg_pallette(ppu: &NesPPU, attr_table: &[u8], tile_column: usize, tile_row: usize) -> [u8; 4] {
    let attr_table_idx = tile_row / 4 * 8 + tile_column / 4;
    let attr_byte = attr_table[attr_table_idx];

    let palette_idx = match (tile_column % 4 / 2, tile_row % 4 / 2) {
        (0, 0) => attr_byte & 0b11,
        (1, 0) => (attr_byte >> 2) & 0b11,
        (0, 1) => (attr_byte >> 4) & 0b11,
        (1, 1) => (attr_byte >> 6) & 0b11,
        _ => unreachable!(),
    };

    let palette_start = 1 + palette_idx as usize * 4;
    [
        ppu.palette_table[0],
        ppu.palette_table[palette_start],
        ppu.palette_table[palette_start + 1],
        ppu.palette_table[palette_start + 2],
    ]
}

fn sprite_pallette(ppu: &NesPPU, palette_idx: u8) -> [u8; 4] {
    let start = 0x11 + palette_idx as usize * 4;
    [
        0,
        ppu.palette_table[start],
        ppu.palette_table[start + 1],
        ppu.palette_table[start + 2],
    ]
}

struct Rect {
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
}

impl Rect {
    fn new(x1: usize, y1: usize, x2: usize, y2: usize) -> Self {
        Rect { x1, y1, x2, y2 }
    }
}

fn render_name_table(
    ppu: &NesPPU,
    frame: &mut Frame,
    name_table: &[u8],
    view_port: Rect,
    shift_x: isize,
    shift_y: isize,
) {
    let bank = ppu.ctrl.bknd_pattern_addr();

    let attr_table = &name_table[0x03c0..0x0400];

    for i in 0..0x03c0 {
        let tile_x = i % 32;
        let tile_y = i / 32;
        let tile_idx = name_table[i] as u16;
        let tile =
            &ppu.chr_rom[(bank + tile_idx * 16) as usize..=(bank + tile_idx * 16 + 15) as usize];
        let palette = bg_pallette(ppu, attr_table, tile_x, tile_y);

        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];

            for x in (0..=7).rev() {
                let color = (1 & lower) << 1 | (1 & upper);
                upper >>= 1;
                lower >>= 1;

                let rgb = match color {
                    0b00 => SYSTEM_PALLETE[ppu.palette_table[0] as usize],
                    0b01 => SYSTEM_PALLETE[palette[1] as usize],
                    0b10 => SYSTEM_PALLETE[palette[2] as usize],
                    0b11 => SYSTEM_PALLETE[palette[3] as usize],
                    _ => unreachable!(),
                };
                let pixel_x = tile_x * 8 + x;
                let pixel_y = tile_y * 8 + y;
                if pixel_x >= view_port.x1
                    && pixel_x < view_port.x2
                    && pixel_y >= view_port.y1
                    && pixel_y < view_port.y2
                {
                    frame.set_pixel(
                        (shift_x + pixel_x as isize) as usize,
                        (shift_y + pixel_y as isize) as usize,
                        rgb,
                    );
                }
            }
        }
    }
}

pub fn render(ppu: &NesPPU, frame: &mut Frame) {
    let scroll_x = ppu.scroll.scroll_x as usize;
    let scroll_y = ppu.scroll.scroll_y as usize;

    let (main_nametable, second_nametable) = match (&ppu.mirroring, ppu.ctrl.nametable_addr()) {
        (Mirroring::VERTICAL, 0x2000)
        | (Mirroring::VERTICAL, 0x2800)
        | (Mirroring::HORIZONTAL, 0x2000)
        | (Mirroring::HORIZONTAL, 0x2400) => (&ppu.vram[0..0x400], &ppu.vram[0x400..0x800]),
        (Mirroring::VERTICAL, 0x2400)
        | (Mirroring::VERTICAL, 0x2c00)
        | (Mirroring::HORIZONTAL, 0x2800)
        | (Mirroring::HORIZONTAL, 0x2c00) => (&ppu.vram[0x400..0x800], &ppu.vram[0..0x400]),
        _ => unreachable!(),
    };

    render_name_table(
        ppu,
        frame,
        main_nametable,
        Rect::new(scroll_x, scroll_y, 256, 240),
        -(scroll_x as isize),
        -(scroll_y as isize),
    );
    if scroll_x > 0 {
        render_name_table(
            ppu,
            frame,
            second_nametable,
            Rect::new(0, 0, scroll_x, 240),
            256 - (scroll_x as isize),
            0,
        );
    } else if scroll_y > 0 {
        render_name_table(
            ppu,
            frame,
            second_nametable,
            Rect::new(0, 0, 256, scroll_y),
            0,
            240 - (scroll_y as isize),
        );
    }
    for i in (0..ppu.oam_data.len()).step_by(4).rev() {
        let tile_idx = ppu.oam_data[i + 1] as u16;
        let tile_x = ppu.oam_data[i + 3] as usize;
        let tile_y = ppu.oam_data[i] as usize;

        let flip_v = ppu.oam_data[i + 2] >> 7 & 1 == 1;
        let flip_h = ppu.oam_data[i + 2] >> 6 & 1 == 1;

        let palette_idx = ppu.oam_data[i + 2] & 0b11;
        let sprite_pallete = sprite_pallette(ppu, palette_idx);
        let bank = ppu.ctrl.sprite_pattern_addr();

        let tile =
            &ppu.chr_rom[(bank + tile_idx * 16) as usize..=(bank + tile_idx * 16 + 15) as usize];

        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];
            'inner: for x in (0..=7).rev() {
                let value = ((lower & 1) << 1) | (upper & 1);
                upper >>= 1;
                lower >>= 1;
                let rgb = match value {
                    0 => continue 'inner,
                    1 => SYSTEM_PALLETE[sprite_pallete[1] as usize],
                    2 => SYSTEM_PALLETE[sprite_pallete[2] as usize],
                    3 => SYSTEM_PALLETE[sprite_pallete[3] as usize],
                    _ => unreachable!(),
                };
                match (flip_h, flip_v) {
                    (false, false) => frame.set_pixel(tile_x + x, tile_y + y, rgb),
                    (true, false) => frame.set_pixel(tile_x + 7 - x, tile_y + y, rgb),
                    (false, true) => frame.set_pixel(tile_x + x, tile_y + 7 - y, rgb),
                    (true, true) => frame.set_pixel(tile_x + 7 - x, tile_y + 7 - y, rgb),
                }
            }
        }
    }
}

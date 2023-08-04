use crate::{
    cartridge::Rom,
    cpu::Mem,
    ppu::{NesPPU, PPU}, joypad::Joypad,
};

const RAM: u16 = 0x0000;
const RAM_MIRRORS_END: u16 = 0x1FFF;

const PPU_CTRL: u16 = 0x2000;
const PPU_MASK: u16 = 0x2001;
const PPU_STATUS: u16 = 0x2002;
const PPU_OAM_ADDR: u16 = 0x2003;
const PPU_OAM_DATA: u16 = 0x2004;
const PPU_SCROLL: u16 = 0x2005;
const PPU_ADDR: u16 = 0x2006;
const PPU_DATA: u16 = 0x2007;

const PPU_REGISTERS_MIRRORS_START: u16 = 0x2008;
const PPU_REGISTERS_MIRRORS_END: u16 = 0x3FFF;

impl Mem for Bus<'_> {
    fn mem_read(&mut self, address: u16) -> u8 {
        match address {
            RAM..=RAM_MIRRORS_END => {
                let unmirrored_address = address & 0x07FF;
                self.cpu_vram[(unmirrored_address & 0x07FF) as usize]
            }
            PPU_CTRL | PPU_MASK | PPU_OAM_ADDR | PPU_SCROLL | PPU_ADDR | 0x4014 => {
                panic!("Cannot read from write-only PPU register")
            }
            PPU_STATUS => self.ppu.read_status(),
            PPU_OAM_DATA => self.ppu.read_oam_data(),
            PPU_DATA => self.ppu.read_data(),
            0x4000..=0x4015 => 0, // APU
            0x4016 => self.joypad1.read(),
            0x4017 => 0,          // joypad 2
            PPU_REGISTERS_MIRRORS_START..=PPU_REGISTERS_MIRRORS_END => {
                let miror_down_address = address & 0x2007;
                self.mem_read(miror_down_address)
            }
            0x8000..=0xFFFF => self.read_prg_rom(address),
            _ => {
                eprintln!("Invalid memory address: {:#X}", address);
                0
            }
        }
    }

    fn mem_write(&mut self, address: u16, value: u8) {
        match address {
            RAM..=RAM_MIRRORS_END => {
                self.cpu_vram[(address & 0x07FF) as usize] = value;
            }
            PPU_CTRL => self.ppu.write_to_ctrl(value),
            PPU_MASK => self.ppu.write_to_mask(value),
            PPU_STATUS => panic!("Cannot write to read-only PPU register"),
            PPU_OAM_ADDR => self.ppu.write_to_oam_addr(value),
            PPU_OAM_DATA => self.ppu.write_to_oam_data(value),
            PPU_SCROLL => self.ppu.write_to_scroll(value),
            PPU_ADDR => self.ppu.write_to_ppu_addr(value),
            PPU_DATA => self.ppu.write_to_data(value),
            0x4000..=0x4013 | 0x4015 => {} // APU
            0x4016 => self.joypad1.write(value),
            0x4017 => {}                   // joypad 2
            0x4014 => {
                let mut buffer: [u8; 256] = [0; 256];
                let hi: u16 = (value as u16) << 8;
                for i in 0..=255 {
                    buffer[i as usize] = self.mem_read(hi | i);
                }
                self.ppu.write_to_oam_dma(&buffer);
            }
            PPU_REGISTERS_MIRRORS_START..=PPU_REGISTERS_MIRRORS_END => {
                let miror_down_address = address & 0x2007;
                self.mem_write(miror_down_address, value);
            }
            0x8000..=0xFFFF => panic!("Cannot write to ROM"),
            _ => eprintln!("Invalid memory address: {:#X}", address),
        }
    }
}

pub struct Bus<'call> {
    cpu_vram: [u8; 2048],
    rom: Vec<u8>,
    ppu: NesPPU,

    cycles: usize,
    game_loop_callback: Box<dyn FnMut(&NesPPU, &mut Joypad) + 'call>,
    joypad1: Joypad,
}

impl<'a> Bus<'a> {
    pub fn new<'call, F>(rom: Rom, game_loop_callback: F) -> Bus<'call>
    where
        F: FnMut(&NesPPU, &mut Joypad) + 'call,
    {
        let ppu = NesPPU::new(rom.chr_rom, rom.mirroring);
        Bus {
            cpu_vram: [0; 2048],
            rom: rom.prg_rom,
            ppu,
            cycles: 0,
            game_loop_callback: Box::from(game_loop_callback),
            joypad1: Joypad::new(),
        }
    }

    fn read_prg_rom(&self, mut address: u16) -> u8 {
        address -= 0x8000;
        if self.rom.len() == 0x4000 {
            address %= 0x4000;
        }
        self.rom[address as usize]
    }

    pub fn tick(&mut self, cycles: u8) {
        self.cycles += cycles as usize;
        let new_frame = self.ppu.tick(cycles * 3);
        if new_frame {
            (self.game_loop_callback)(&self.ppu, &mut self.joypad1);
        }
    }

    pub fn poll_nmi_status(&mut self) -> Option<u8> {
        self.ppu.poll_nmi_interrupt()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cartridge::test;

    #[test]
    fn test_mem_read_write_to_ram() {
        let mut bus = Bus::new(test::test_rom(), |_ppu: &NesPPU, _joypad: &mut Joypad| {});
        bus.mem_write(0x01, 0x55);
        assert_eq!(bus.mem_read(0x01), 0x55);
    }

    #[test]
    fn test_mem_write_to_oam() {
        let mut bus = Bus::new(test::test_rom(), |_ppu: &NesPPU, _joypad: &mut Joypad| {});
        bus.mem_write(0x2003, 0x55);
        assert_eq!(bus.ppu.oam_addr, 0x55);
        bus.mem_write(0x2004, 0x66);
        assert_eq!(bus.ppu.oam_data[0x55], 0x66);
    }
}

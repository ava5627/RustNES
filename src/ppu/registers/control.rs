

bitflags! {
    pub struct ControlRegister: u8 {
        const NAMETABLE1          = 0b00000001;
        const NAMETABLE2          = 0b00000010;
        const VRAM_ADD_INC        = 0b00000100;
        const SPRITE_PATTERN_ADDR = 0b00001000;
        const BG_PATTERN_ADDR     = 0b00010000;
        const SPRITE_SIZE         = 0b00100000;
        const MASTER_SLAVE_SELECT = 0b01000000;
        const GENERATE_NMI        = 0b10000000;
    }
}

impl ControlRegister {
    pub fn new() -> Self {
        ControlRegister::from_bits_truncate(0)
    }

    pub fn vram_addr_increment(&self) -> u8 {
        if !self.contains(ControlRegister::VRAM_ADD_INC) {
            1
        } else {
            32
        }
    }

    pub fn update(&mut self, data: u8) {
        *self = ControlRegister::from_bits_truncate(data);
    }

    pub fn generate_nmi(&self) -> bool {
        self.contains(ControlRegister::GENERATE_NMI)
    }

    pub fn bknd_pattern_addr(&self) -> u16 {
        if !self.contains(ControlRegister::BG_PATTERN_ADDR) {
            0x0000
        } else {
            0x1000
        }
    }

    pub fn sprite_pattern_addr(&self) -> u16 {
        if !self.contains(ControlRegister::SPRITE_PATTERN_ADDR) {
            0x0000
        } else {
            0x1000
        }
    }

    pub fn sprite_size(&self) -> u8 {
        if !self.contains(ControlRegister::SPRITE_SIZE) {
            8
        } else {
            16
        }
    }

    pub fn nametable_addr(&self) -> u16 {
        match self.bits() & 0b00000011 {
            0b00 => 0x2000,
            0b01 => 0x2400,
            0b10 => 0x2800,
            0b11 => 0x2C00,
            _ => panic!("Invalid nametable address"),
        }
    }

    pub fn master_slave_select(&self) -> bool {
        self.contains(ControlRegister::MASTER_SLAVE_SELECT)
    }



}

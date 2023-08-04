bitflags! {
    pub struct StatusRegister: u8 {
        const NOT_USED_0          = 0b00000001;
        const NOT_USED_1          = 0b00000010;
        const NOT_USED_2          = 0b00000100;
        const NOT_USED_3          = 0b00001000;
        const NOT_USED_4          = 0b00010000;
        const SPRITE_OVERFLOW     = 0b00100000;
        const SPRITE_ZERO_HIT     = 0b01000000;
        const VERTICAL_BLANK      = 0b10000000;
    }
}

impl StatusRegister {
    pub fn new() -> Self {
        StatusRegister::from_bits_truncate(0)
    }

    pub fn set_sprite_overflow(&mut self, overflow: bool) {
        self.set(StatusRegister::SPRITE_OVERFLOW, overflow);
    }

    pub fn set_sprite_zero_hit(&mut self, hit: bool) {
        self.set(StatusRegister::SPRITE_ZERO_HIT, hit);
    }

    pub fn set_vertical_blank(&mut self, blank: bool) {
        self.set(StatusRegister::VERTICAL_BLANK, blank);
    }

    pub fn reset_vertical_blank(&mut self) {
        self.remove(StatusRegister::VERTICAL_BLANK);
    }

    pub fn is_in_sprite_overflow(&self) -> bool {
        self.contains(StatusRegister::SPRITE_OVERFLOW)
    }

    pub fn is_in_sprite_zero_hit(&self) -> bool {
        self.contains(StatusRegister::SPRITE_ZERO_HIT)
    }

    pub fn is_in_vertical_blank(&self) -> bool {
        self.contains(StatusRegister::VERTICAL_BLANK)
    }
}

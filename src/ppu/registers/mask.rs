bitflags! {
    pub struct MaskRegister: u8 {
        const GREYSCALE               = 0b00000001;
        const LEFTMOST_8PXL_BG        = 0b00000010;
        const LEFTMOST_8PXL_SPRITE     = 0b00000100;
        const SHOW_BACKGROUND         = 0b00001000;
        const SHOW_SPRITES            = 0b00010000;
        const EMPHASISE_RED           = 0b00100000;
        const EMPHASISE_GREEN         = 0b01000000;
        const EMPHASISE_BLUE          = 0b10000000;
    }
}

pub enum Color {
    Red,
    Green,
    Blue,
}

impl MaskRegister {
    pub fn new() -> Self {
        MaskRegister::from_bits_truncate(0)
    }

    pub fn is_greyscale(&self) -> bool {
        self.contains(MaskRegister::GREYSCALE)
    }

    pub fn leftmost_8pxl_bg(&self) -> bool {
        self.contains(MaskRegister::LEFTMOST_8PXL_BG)
    }

    pub fn leftmost_8pxl_sprite(&self) -> bool {
        self.contains(MaskRegister::LEFTMOST_8PXL_SPRITE)
    }

    pub fn show_background(&self) -> bool {
        self.contains(MaskRegister::SHOW_BACKGROUND)
    }

    pub fn show_sprites(&self) -> bool {
        self.contains(MaskRegister::SHOW_SPRITES)
    }

    pub fn emphasise(&self) -> Vec<Color> {
        let mut result = Vec::new();
        if self.contains(MaskRegister::EMPHASISE_RED) {
            result.push(Color::Red);
        }
        if self.contains(MaskRegister::EMPHASISE_GREEN) {
            result.push(Color::Green);
        }
        if self.contains(MaskRegister::EMPHASISE_BLUE) {
            result.push(Color::Blue);
        }
        result
    }

    pub fn update(&mut self, data: u8) {
        *self = MaskRegister::from_bits_truncate(data);
    }
}

impl Default for MaskRegister {
    fn default() -> Self {
        Self::new()
    }
}

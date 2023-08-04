bitflags! {
    #[derive(Clone, Copy, Default)]
    pub struct JoypadButton: u8 {
        const A      = 0b00000001;
        const B      = 0b00000010;
        const SELECT = 0b00000100;
        const START  = 0b00001000;
        const UP     = 0b00010000;
        const DOWN   = 0b00100000;
        const LEFT   = 0b01000000;
        const RIGHT  = 0b10000000;
    }
}

pub struct Joypad {
    strobe: bool,
    button_index: u8,
    button_status: JoypadButton,
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            strobe: false,
            button_index: 0,
            button_status: JoypadButton::empty(),
        }
    }

    pub fn write(&mut self, value: u8) {
        self.strobe = value & 0x01 == 0x01;
        if self.strobe {
            self.button_index = 0;
        }
    }

    pub fn read(&mut self) -> u8 {
        if self.button_index > 7 {
            return 1;
        }
        let response = (self.button_status.bits() & (1 << self.button_index)) >> self.button_index;
        if !self.strobe {
            self.button_index += 1;
        }
        response
    }

    pub fn press(&mut self, button: JoypadButton) {
        self.button_status.insert(button);
    }

    pub fn release(&mut self, button: JoypadButton) {
        self.button_status.remove(button);
    }
}

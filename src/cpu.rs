#![allow(clippy::upper_case_acronyms)]
use std::{collections::HashMap, fmt::Display};

use nes_macro::{match_all, opcode};

use crate::{bus::Bus, opcodes};

const STACK: u16 = 0x0100;
const STACK_START: u8 = 0xFD;

const PROGRAM_START: u16 = 0x0600;
// const PROGRAM_START: u16 = 0x8000;

bitflags! {
    #[derive(Clone)]
    pub struct StatusFlags: u8 {
        const CARRY    = 0b0000_0001;
        const ZERO     = 0b0000_0010;
        const INTERRUPT_DISABLE      = 0b0000_0100;
        const DECIMAL  = 0b0000_1000;
        const BREAK    = 0b0001_0000;
        const BREAK2   = 0b0010_0000;
        const OVERFLOW = 0b0100_0000;
        const NEGATIVE = 0b1000_0000;
    }
}

pub enum AddressingMode {
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    IndirectX,
    IndirectY,
    NoneAddressing,
}

impl Display for AddressingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddressingMode::Accumulator => write!(f, "ac"),
            AddressingMode::Immediate => write!(f, "im"),
            AddressingMode::ZeroPage => write!(f, "zp"),
            AddressingMode::ZeroPageX => write!(f, "zx"),
            AddressingMode::ZeroPageY => write!(f, "zy"),
            AddressingMode::Absolute => write!(f, "ab"),
            AddressingMode::AbsoluteX => write!(f, "ax"),
            AddressingMode::AbsoluteY => write!(f, "ay"),
            AddressingMode::IndirectX => write!(f, "ix"),
            AddressingMode::IndirectY => write!(f, "iy"),
            AddressingMode::NoneAddressing => write!(f, "na"),
        }
    }
}

pub trait Mem {
    fn mem_read(&mut self, address: u16) -> u8;
    fn mem_write(&mut self, address: u16, value: u8);

    fn u16_mem_read(&mut self, address: u16) -> u16 {
        let low = self.mem_read(address) as u16;
        let high = self.mem_read(address + 1) as u16;
        (high << 8) | low
    }

    fn u16_mem_write(&mut self, address: u16, value: u16) {
        let low = value as u8;
        let high = (value >> 8) as u8;
        self.mem_write(address, low);
        self.mem_write(address + 1, high);
    }
}

impl Mem for CPU<'_> {
    fn mem_read(&mut self, address: u16) -> u8 {
        self.bus.mem_read(address)
    }

    fn mem_write(&mut self, address: u16, value: u8) {
        self.bus.mem_write(address, value);
    }

    fn u16_mem_read(&mut self, address: u16) -> u16 {
        self.bus.u16_mem_read(address)
    }

    fn u16_mem_write(&mut self, address: u16, value: u16) {
        self.bus.u16_mem_write(address, value);
    }
}

fn page_crossed(addr1: u16, addr2: u16) -> bool {
    addr1 & 0xFF00 != addr2 & 0xFF00
}

mod interrupt {
    #[derive(PartialEq, Eq)]
    pub enum InterruptType {
        NMI,
    }

    #[derive(PartialEq, Eq)]
    pub(super) struct Interrupt {
        pub(super) itype: InterruptType,
        pub(super) vector_addr: u16,
        pub(super) b_flag_mask: u8,
        pub(super) cpu_cycles: u8,
    }

    pub(super) const NMI: Interrupt = Interrupt {
        itype: InterruptType::NMI,
        vector_addr: 0xFFFA,
        b_flag_mask: 0b0010_0000,
        cpu_cycles: 2,
    };
}

pub struct CPU<'a> {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: StatusFlags,
    pub stack_pointer: u8,
    pub program_counter: u16,
    pub bus: Bus<'a>,
}

impl CPU<'_> {
    pub fn new(bus: Bus<'_>) -> CPU<'_> {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status: StatusFlags::from_bits_truncate(0b100100),
            stack_pointer: 0xFD,
            program_counter: 0,
            bus,
        }
    }

    fn stack_push_u16(&mut self, value: u16) {
        let lo = (value & 0x00FF) as u8;
        let hi = ((value & 0xFF00) >> 8) as u8;
        self.stack_push_u8(hi);
        self.stack_push_u8(lo);
    }

    fn stack_push_u8(&mut self, value: u8) {
        self.mem_write(STACK + self.stack_pointer as u16, value);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
    }

    fn stack_pop_u16(&mut self) -> u16 {
        let lo = self.stack_pop_u8() as u16;
        let hi = self.stack_pop_u8() as u16;
        (hi << 8) | lo
    }

    fn stack_pop_u8(&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.mem_read(STACK + self.stack_pointer as u16)
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.status = StatusFlags::from_bits_truncate(0b100100);
        self.stack_pointer = STACK_START;
        self.program_counter = self.u16_mem_read(0xFFFC);
    }

    pub fn load(&mut self, program: Vec<u8>) {
        for (i, byte) in program.iter().enumerate() {
            self.mem_write(PROGRAM_START + i as u16, *byte);
        }
        self.u16_mem_write(0xFFFC, PROGRAM_START);
    }

    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run();
    }

    // ignore dead code warning
    #[allow(dead_code)]
    fn load_and_run_no_reset(&mut self, program: Vec<u8>) {
        self.load(program);
        self.program_counter = PROGRAM_START;
        self.run();
    }

    fn add_to_reg_a(&mut self, value: u8) {
        let sum: u16 =
            self.register_a as u16 + value as u16 + self.status.contains(StatusFlags::CARRY) as u16;

        self.status.set(StatusFlags::CARRY, sum > 0xFF);
        let result = sum as u8;

        if (value ^ result) & (result ^ self.register_a) & 0x80 != 0 {
            self.status.insert(StatusFlags::OVERFLOW);
        } else {
            self.status.remove(StatusFlags::OVERFLOW);
        }

        self.register_a = result;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn sub_from_reg_a(&mut self, value: u8) {
        self.add_to_reg_a(((value as i8).wrapping_neg().wrapping_sub(1)) as u8);
    }

    #[opcode(codes = [0x69, 0x65, 0x75, 0x6D, 0x7D, 0x79, 0x61, 0x71], name = "ADC", addr_mode)]
    fn adc(&mut self, mode: &AddressingMode) {
        let (address, pc) = self.get_operand_address(mode);
        let value = self.mem_read(address);
        self.add_to_reg_a(value);
        self.update_zero_and_negative_flags(self.register_a);
        if pc {
            self.bus.tick(1);
        }
    }

    #[opcode(codes = [0x29, 0x25, 0x35, 0x2D, 0x3D, 0x39, 0x21, 0x31], name = "AND", addr_mode)]
    fn and(&mut self, mode: &AddressingMode) {
        let (address, pc) = self.get_operand_address(mode);
        let value = self.mem_read(address);
        self.register_a &= value;
        self.update_zero_and_negative_flags(self.register_a);
        if pc {
            self.bus.tick(1);
        }
    }

    #[opcode(codes = [0x0A, 0x06, 0x16, 0x0E, 0x1E], name = "ASL", addr_mode)]
    fn asl(&mut self, mode: &AddressingMode) {
        if let AddressingMode::Accumulator = mode {
            self.asl_accumulator();
        } else {
            let (addr, _pc) = self.get_operand_address(mode);
            self.asl_memory(addr);
        };
    }

    fn asl_accumulator(&mut self) {
        let value = self.register_a;
        self.status.set(StatusFlags::CARRY, value & 0x80 != 0);
        self.register_a = value << 1;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn asl_memory(&mut self, address: u16) {
        let value = self.mem_read(address);
        self.status.set(StatusFlags::CARRY, value & 0x80 != 0);
        let result = value << 1;
        self.mem_write(address, result);
        self.update_zero_and_negative_flags(result);
    }

    fn branch(&mut self, condition: bool) {
        if condition {
            self.bus.tick(1);

            let offset = self.mem_read(self.program_counter) as i8;
            // add 1 to the offset to account for the length of the new address... I think
            let jump_addr = self
                .program_counter
                .wrapping_add(1)
                .wrapping_add(offset as u16);
            if self.program_counter.wrapping_add(1) & 0xFF00 != jump_addr & 0xFF00 {
                self.bus.tick(1);
            }
            self.program_counter = jump_addr;
        }
    }

    #[opcode(codes = [0x90], name = "BCC")]
    fn bcc(&mut self) {
        self.branch(!self.status.contains(StatusFlags::CARRY))
    }

    #[opcode(codes = [0xB0], name = "BCS")]
    fn bcs(&mut self) {
        self.branch(self.status.contains(StatusFlags::CARRY))
    }

    #[opcode(codes = [0xF0], name = "BEQ")]
    fn beq(&mut self) {
        self.branch(self.status.contains(StatusFlags::ZERO))
    }

    #[opcode(codes = [0x24, 0x2C], name = "BIT", addr_mode)]
    fn bit(&mut self, mode: &AddressingMode) {
        let (address, _pc) = self.get_operand_address(mode);
        let value = self.mem_read(address);
        let result = self.register_a & value;
        self.status.set(StatusFlags::ZERO, result == 0);
        self.status.set(StatusFlags::OVERFLOW, value & 0x40 > 0);
        self.status.set(StatusFlags::NEGATIVE, value & 0x80 > 0);
    }

    #[opcode(codes = [0x30], name = "BMI")]
    fn bmi(&mut self) {
        self.branch(self.status.contains(StatusFlags::NEGATIVE))
    }

    #[opcode(codes = [0xD0], name = "BNE")]
    fn bne(&mut self) {
        self.branch(!self.status.contains(StatusFlags::ZERO))
    }

    #[opcode(codes = [0x10], name = "BPL")]
    fn bpl(&mut self) {
        self.branch(!self.status.contains(StatusFlags::NEGATIVE))
    }

    #[opcode(codes = [0x00], name = "BRK")]
    fn brk(&mut self) {
        self.status.insert(StatusFlags::BREAK);
    }

    #[opcode(codes = [0x50], name = "BVC")]
    fn bvc(&mut self) {
        self.branch(!self.status.contains(StatusFlags::OVERFLOW))
    }

    #[opcode(codes = [0x70], name = "BVS")]
    fn bvs(&mut self) {
        self.branch(self.status.contains(StatusFlags::OVERFLOW))
    }

    #[opcode(codes = [0x18], name = "CLC")]
    fn clc(&mut self) {
        self.status.remove(StatusFlags::CARRY);
    }

    #[opcode(codes = [0xD8], name = "CLD")]
    fn cld(&mut self) {
        self.status.remove(StatusFlags::DECIMAL);
    }

    #[opcode(codes = [0x58], name = "CLI")]
    fn cli(&mut self) {
        self.status.remove(StatusFlags::INTERRUPT_DISABLE);
    }

    #[opcode(codes = [0xB8], name = "CLV")]
    fn clv(&mut self) {
        self.status.remove(StatusFlags::OVERFLOW);
    }

    #[opcode(codes = [0xC9, 0xC5, 0xD5, 0xCD, 0xDD, 0xD9, 0xC1, 0xD1], name = "CMP", addr_mode)]
    fn cmp(&mut self, mode: &AddressingMode) {
        let (address, pc) = self.get_operand_address(mode);
        let value = self.mem_read(address);
        let result = self.register_a.wrapping_sub(value);
        self.status
            .set(StatusFlags::CARRY, self.register_a >= value);
        self.update_zero_and_negative_flags(result);
        if pc {
            self.bus.tick(1);
        }
    }

    #[opcode(codes = [0xE0, 0xE4, 0xEC], name = "CPX", addr_mode)]
    fn cpx(&mut self, mode: &AddressingMode) {
        let (address, pc) = self.get_operand_address(mode);
        let value = self.mem_read(address);
        let result = self.register_x.wrapping_sub(value);
        self.status
            .set(StatusFlags::CARRY, self.register_x >= value);
        self.update_zero_and_negative_flags(result);
        if pc {
            self.bus.tick(1);
        }
    }

    #[opcode(codes = [0xC0, 0xC4, 0xCC], name = "CPY", addr_mode)]
    fn cpy(&mut self, mode: &AddressingMode) {
        let (address, pc) = self.get_operand_address(mode);
        let value = self.mem_read(address);
        let result = self.register_y.wrapping_sub(value);
        self.status
            .set(StatusFlags::CARRY, self.register_y >= value);
        self.update_zero_and_negative_flags(result);
        if pc {
            self.bus.tick(1);
        }
    }

    #[opcode(codes = [0xC6, 0xD6, 0xCE, 0xDE], name = "DEC", addr_mode)]
    fn dec(&mut self, mode: &AddressingMode) {
        let (address, _pc) = self.get_operand_address(mode);
        let value = self.mem_read(address).wrapping_sub(1);
        self.mem_write(address, value);
        self.update_zero_and_negative_flags(value);
    }

    #[opcode(codes = [0xCA], name = "DEX")]
    fn dex(&mut self) {
        self.register_x = self.register_x.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_x);
    }

    #[opcode(codes = [0x88], name = "DEY")]
    fn dey(&mut self) {
        self.register_y = self.register_y.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_y);
    }

    #[opcode(codes = [0x49, 0x45, 0x55, 0x4D, 0x5D, 0x59, 0x41, 0x51], name = "EOR", addr_mode)]
    fn eor(&mut self, mode: &AddressingMode) {
        let (address, pc) = self.get_operand_address(mode);
        let value = self.mem_read(address);
        self.register_a ^= value;
        self.update_zero_and_negative_flags(self.register_a);
        if pc {
            self.bus.tick(1);
        }
    }

    #[opcode(codes = [0xE6, 0xF6, 0xEE, 0xFE], name = "INC", addr_mode)]
    fn inc(&mut self, mode: &AddressingMode) {
        let (address, _pc) = self.get_operand_address(mode);
        let value = self.mem_read(address).wrapping_add(1);
        self.mem_write(address, value);
        self.update_zero_and_negative_flags(value);
    }

    #[opcode(codes = [0xE8], name = "INX")]
    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_x);
    }

    #[opcode(codes = [0xC8], name = "INY")]
    fn iny(&mut self) {
        self.register_y = self.register_y.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_y);
    }

    #[opcode(codes = [0x4C, 0x6C], name = "JMP", addr_mode)]
    fn jmp(&mut self, mode: &AddressingMode) {
        let address = self.u16_mem_read(self.program_counter);
        if let AddressingMode::Absolute = mode {
            self.program_counter = address;
            return;
        }

        let indirect_ref = if address & 0x00FF == 0x00FF {
            // Simulate page boundary hardware bug
            let lo = self.mem_read(address);
            let hi = self.mem_read(address & 0xFF00);
            (hi as u16) << 8 | (lo as u16)
        } else {
            self.u16_mem_read(address)
        };

        self.program_counter = indirect_ref;
    }

    #[opcode(codes = [0x20], name = "JSR")]
    fn jsr(&mut self) {
        let address = self.u16_mem_read(self.program_counter);
        let return_address = self.program_counter + 2 - 1; // +2 for the operand, -1 for the PC increment
        self.stack_push_u16(return_address);
        self.program_counter = address;
    }

    #[opcode(codes = [0xA9, 0xA5, 0xB5, 0xAD, 0xBD, 0xB9, 0xA1, 0xB1], name = "LDA", addr_mode)]
    fn lda(&mut self, mode: &AddressingMode) {
        if let AddressingMode::Immediate = mode {
            self.register_a = self.mem_read(self.program_counter);
            self.program_counter += 1;
            self.update_zero_and_negative_flags(self.register_a);
            return;
        }
        let (address, pc) = self.get_operand_address(mode);
        let value = self.mem_read(address);
        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
        if pc {
            self.bus.tick(1);
        }
    }

    #[opcode(codes = [0xA2, 0xA6, 0xB6, 0xAE, 0xBE], name = "LDX", addr_mode)]
    fn ldx(&mut self, mode: &AddressingMode) {
        let (address, pc) = self.get_operand_address(mode);
        let value = self.mem_read(address);
        self.register_x = value;
        self.update_zero_and_negative_flags(self.register_x);
        if pc {
            self.bus.tick(1);
        }
    }

    #[opcode(codes = [0xA0, 0xA4, 0xB4, 0xAC, 0xBC], name = "LDY", addr_mode)]
    fn ldy(&mut self, mode: &AddressingMode) {
        let (address, pc) = self.get_operand_address(mode);
        let value = self.mem_read(address);
        self.register_y = value;
        self.update_zero_and_negative_flags(self.register_y);
        if pc {
            self.bus.tick(1);
        }
    }

    #[opcode(codes = [0x4A, 0x46, 0x56, 0x4E, 0x5E], name = "LSR", addr_mode)]
    fn lsr(&mut self, mode: &AddressingMode) {
        if let AddressingMode::Accumulator = mode {
            let value = self.register_a;
            self.status.set(StatusFlags::CARRY, value & 0x01 == 0x01);
            let value = value >> 1;
            self.update_zero_and_negative_flags(value);
            self.register_a = value;
            return;
        }
        let (address, _) = self.get_operand_address(mode);
        let mut value = self.mem_read(address);
        self.status.set(StatusFlags::CARRY, value & 0x01 == 0x01);
        value >>= 1;
        self.update_zero_and_negative_flags(value);
        self.mem_write(address, value);
    }

    #[opcode(codes = [0xEA], name = "NOP")]
    #[opcode(codes = [0x80, 0x82, 0x89, 0xC2, 0xE2], name = "*NOP")]
    #[opcode(codes = [0x02, 0x12, 0x22, 0x32, 0x42, 0x52, 0x62, 0x72, 0x92, 0xB2, 0xD2, 0xF2], name = "*NOP")]
    #[opcode(codes = [0x1A, 0x3A, 0x5A, 0x7A, 0xDA, 0xFA], name = "*NOP")]
    fn nop(&mut self) {}

    #[opcode(codes = [0x04, 0x44, 0x64, 0x14, 0x34, 0x54, 0x74, 0xD4, 0xF4], name = "*NOP", addr_mode)]
    #[opcode(codes = [0x0C, 0x1C, 0x3C, 0x5C, 0x7C, 0xDC, 0xFC], name = "*NOP", addr_mode)]
    fn nop_read(&mut self, mode: &AddressingMode) {
        let (_address, pc) = self.get_operand_address(mode);
        if pc {
            self.bus.tick(1);
        }
    }

    #[opcode(codes = [0x09, 0x05, 0x15, 0x0D, 0x1D, 0x19, 0x01, 0x11], name = "ORA", addr_mode)]
    fn ora(&mut self, mode: &AddressingMode) {
        let (address, pc) = self.get_operand_address(mode);
        let value = self.mem_read(address);
        self.register_a |= value;
        self.update_zero_and_negative_flags(self.register_a);
        if pc {
            self.bus.tick(1);
        }
    }

    #[opcode(codes = [0x48], name = "PHA")]
    fn pha(&mut self) {
        self.stack_push_u8(self.register_a);
    }

    #[opcode(codes = [0x08], name = "PHP")]
    fn php(&mut self) {
        let mut flag = self.status.clone();
        flag.insert(StatusFlags::BREAK);
        flag.insert(StatusFlags::BREAK2);
        self.stack_push_u8(flag.bits());
    }

    #[opcode(codes = [0x68], name = "PLA")]
    fn pla(&mut self) {
        self.register_a = self.stack_pop_u8();
        self.update_zero_and_negative_flags(self.register_a);
    }

    #[opcode(codes = [0x28], name = "PLP")]
    fn plp(&mut self) {
        self.status = StatusFlags::from_bits_truncate(self.stack_pop_u8());
        self.status.remove(StatusFlags::BREAK);
        self.status.insert(StatusFlags::BREAK2);
    }

    #[opcode(codes = [0x2A, 0x26, 0x36, 0x2E, 0x3E], name = "ROL", addr_mode)]
    fn rol(&mut self, mode: &AddressingMode) {
        if let AddressingMode::Accumulator = mode {
            self.rol_accumulator();
            return;
        }
        let (address, _pc) = self.get_operand_address(mode);
        let mut value = self.mem_read(address);
        let carry = self.status.contains(StatusFlags::CARRY);
        self.status.set(StatusFlags::CARRY, value & 0x80 == 0x80);
        value <<= 1;
        value |= carry as u8;
        self.update_zero_and_negative_flags(value);
        self.mem_write(address, value);
    }

    fn rol_accumulator(&mut self) {
        let mut value = self.register_a;
        let carry = self.status.contains(StatusFlags::CARRY);
        self.status.set(StatusFlags::CARRY, value & 0x80 == 0x80);
        value <<= 1;
        value |= carry as u8;
        self.update_zero_and_negative_flags(value);
        self.register_a = value;
    }

    #[opcode(codes = [0x6A, 0x66, 0x76, 0x6E, 0x7E], name = "ROR", addr_mode)]
    fn ror(&mut self, mode: &AddressingMode) {
        if let AddressingMode::Accumulator = mode {
            self.ror_accumulator();
            return;
        }
        let (address, _pc) = self.get_operand_address(mode);
        let mut value = self.mem_read(address);
        let carry = self.status.contains(StatusFlags::CARRY);
        self.status.set(StatusFlags::CARRY, value & 0x01 == 0x01);
        value >>= 1;
        value |= (carry as u8) << 7;
        self.update_zero_and_negative_flags(value);
        self.mem_write(address, value);
    }

    fn ror_accumulator(&mut self) {
        let mut value = self.register_a;
        let carry = self.status.contains(StatusFlags::CARRY);
        self.status.set(StatusFlags::CARRY, value & 0x01 == 0x01);
        value >>= 1;
        value |= (carry as u8) << 7;
        self.update_zero_and_negative_flags(value);
        self.register_a = value;
    }

    #[opcode(codes = [0x40], name = "RTI")]
    fn rti(&mut self) {
        self.status = StatusFlags::from_bits_truncate(self.stack_pop_u8());
        self.status.remove(StatusFlags::BREAK);
        self.status.insert(StatusFlags::BREAK2);
        self.program_counter = self.stack_pop_u16();
    }

    #[opcode(codes = [0x60], name = "RTS")]
    fn rts(&mut self) {
        self.program_counter = self.stack_pop_u16() + 1;
    }

    #[opcode(codes = [0xE9, 0xE5, 0xF5, 0xED, 0xFD, 0xF9, 0xE1, 0xF1], name = "SBC", addr_mode)]
    #[opcode(codes = [0xEB], name = "SBC", addr_mode)]
    fn sbc(&mut self, mode: &AddressingMode) {
        let (address, pc) = self.get_operand_address(mode);
        let value = self.mem_read(address);
        self.sub_from_reg_a(value);
        if pc {
            self.bus.tick(1);
        }
    }

    #[opcode(codes = [0x38], name = "SEC")]
    fn sec(&mut self) {
        self.status.insert(StatusFlags::CARRY);
    }

    #[opcode(codes = [0xF8], name = "SED")]
    fn sed(&mut self) {
        self.status.insert(StatusFlags::DECIMAL);
    }

    #[opcode(codes = [0x78], name = "SEI")]
    fn sei(&mut self) {
        self.status.insert(StatusFlags::INTERRUPT_DISABLE);
    }

    #[opcode(codes = [0x85, 0x95, 0x8D, 0x9D, 0x99, 0x81, 0x91], name = "STA", addr_mode)]
    fn sta(&mut self, mode: &AddressingMode) {
        let (address, _pc) = self.get_operand_address(mode);
        self.mem_write(address, self.register_a);
    }

    #[opcode(codes = [0x86, 0x96, 0x8E], name = "STX", addr_mode)]
    fn stx(&mut self, mode: &AddressingMode) {
        let (address, _pc) = self.get_operand_address(mode);
        self.mem_write(address, self.register_x);
    }

    #[opcode(codes = [0x84, 0x94, 0x8C], name = "STY", addr_mode)]
    fn sty(&mut self, mode: &AddressingMode) {
        let (address, _pc) = self.get_operand_address(mode);
        self.mem_write(address, self.register_y);
    }

    #[opcode(codes = [0xAA], name = "TAX")]
    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_and_negative_flags(self.register_x);
    }

    #[opcode(codes = [0xA8], name = "TAY")]
    fn tay(&mut self) {
        self.register_y = self.register_a;
        self.update_zero_and_negative_flags(self.register_y);
    }

    #[opcode(codes = [0xBA], name = "TSX")]
    fn tsx(&mut self) {
        self.register_x = self.stack_pointer;
        self.update_zero_and_negative_flags(self.register_x);
    }

    #[opcode(codes = [0x8A], name = "TXA")]
    fn txa(&mut self) {
        self.register_a = self.register_x;
        self.update_zero_and_negative_flags(self.register_a);
    }

    #[opcode(codes = [0x9A], name = "TXS")]
    fn txs(&mut self) {
        self.stack_pointer = self.register_x;
    }

    #[opcode(codes = [0x98], name = "TYA")]
    fn tya(&mut self) {
        self.register_a = self.register_y;
        self.update_zero_and_negative_flags(self.register_a);
    }

    // Unofficial opcodes

    #[opcode(codes = [0x0B, 0x2B], name = "ANC", addr_mode)]
    fn anc(&mut self, mode: &AddressingMode) {
        let (address, _pc) = self.get_operand_address(mode);
        let value = self.mem_read(address);
        self.register_a &= value;
        self.status
            .set(StatusFlags::CARRY, self.register_a & 0x80 == 0x80);
        self.update_zero_and_negative_flags(self.register_a);
    }

    #[opcode(codes = [0x87, 0x97, 0x8F, 0x83], name = "SAX", addr_mode)]
    fn sax(&mut self, mode: &AddressingMode) {
        let (address, _pc) = self.get_operand_address(mode);
        let value = self.register_a & self.register_x;
        self.mem_write(address, value);
        // self.update_zero_and_negative_flags(value);
    }

    #[opcode(codes = [0x6B], name = "ARR", addr_mode)]
    fn arr(&mut self, mode: &AddressingMode) {
        let (address, _pc) = self.get_operand_address(mode);
        let value = self.mem_read(address);
        let result = (self.register_a & value) >> 1;
        self.register_a = result;
        self.update_zero_and_negative_flags(self.register_a);
        // xx11xxxx ->  c !v
        // xx00xxxx -> !c !v
        // xx01xxxx -> !c  v
        // xx10xxxx ->  c  v
        self.status
            .set(StatusFlags::CARRY, self.register_a & 0x40 == 0x40);
        self.status.set(
            StatusFlags::OVERFLOW,
            (self.register_a & 0x40 == 0x40) ^ (self.register_a & 0x20 == 0x20),
        );
    }

    #[opcode(codes = [0x4B], name = "ALR", addr_mode)]
    fn alr(&mut self, mode: &AddressingMode) {
        let (address, _pc) = self.get_operand_address(mode);
        let value = self.mem_read(address);
        self.status
            .set(StatusFlags::CARRY, self.register_a & 0x01 == 0x01);
        self.register_a &= value;
        self.register_a >>= 1;
        self.update_zero_and_negative_flags(self.register_a);
    }

    #[opcode(codes = [0xAB], name = "LXA", addr_mode)]
    fn lxa(&mut self, mode: &AddressingMode) {
        let (address, _pc) = self.get_operand_address(mode);
        let value = self.mem_read(address);
        self.register_a = value;
        self.register_x = value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    #[opcode(codes = [0x93, 0x9f], name = "AHX", addr_mode)]
    fn ahx(&mut self, mode: &AddressingMode) {
        let (address, _pc) = self.get_operand_address(mode);
        let value = self.register_a & self.register_x & (address >> 8) as u8;
        self.mem_write(address, value);
    }

    #[opcode(codes = [0xCB], name = "AXS", addr_mode)]
    fn axs(&mut self, mode: &AddressingMode) {
        let (address, _pc) = self.get_operand_address(mode);
        let value = self.mem_read(address);
        let result = (self.register_a & self.register_x).wrapping_sub(value);
        self.register_x = result;
        self.update_zero_and_negative_flags(self.register_x);
        self.status
            .set(StatusFlags::CARRY, self.register_x & 0x80 == 0x80);
    }

    #[opcode(codes = [0xC7, 0xD7, 0xCF, 0xDF, 0xDB, 0xC3, 0xD3], name = "DCP", addr_mode)]
    fn dcp(&mut self, mode: &AddressingMode) {
        let (address, _pc) = self.get_operand_address(mode);
        let value = self.mem_read(address);
        let result = value.wrapping_sub(1);
        self.mem_write(address, result);
        self.update_zero_and_negative_flags(self.register_a.wrapping_sub(result));
        self.status
            .set(StatusFlags::CARRY, self.register_a >= result);
    }

    #[opcode(codes = [0xE7, 0xF7, 0xEF, 0xFF, 0xFB, 0xE3, 0xF3], name = "ISB", addr_mode)]
    fn isb(&mut self, mode: &AddressingMode) {
        let (address, _pc) = self.get_operand_address(mode);
        let value = self.mem_read(address);
        let result = value.wrapping_add(1);
        self.mem_write(address, result);
        self.update_zero_and_negative_flags(result);
        self.status
            .set(StatusFlags::CARRY, self.register_a >= result);
        self.sbc(mode);
    }

    #[opcode(codes = [0xBB], name = "LAS", addr_mode)]
    fn las(&mut self, mode: &AddressingMode) {
        let (address, _pc) = self.get_operand_address(mode);
        let value = self.mem_read(address);
        self.register_a = self.stack_pointer & value;
        self.register_x = self.register_a;
        self.stack_pointer = self.register_a;
        self.update_zero_and_negative_flags(self.register_a);
    }

    #[opcode(codes = [0xA7, 0xB7, 0xAF, 0xBF, 0xA3, 0xB3], name = "LAX", addr_mode)]
    fn lax(&mut self, mode: &AddressingMode) {
        self.lda(mode);
        self.tax();
    }

    #[opcode(codes = [0x27, 0x37, 0x2F, 0x3F, 0x3B, 0x23, 0x33], name = "RLA", addr_mode)]
    fn rla(&mut self, mode: &AddressingMode) {
        self.rol(mode);
        self.and(mode);
    }

    #[opcode(codes = [0x67, 0x77, 0x6F, 0x7F, 0x7B, 0x63, 0x73], name = "RRA", addr_mode)]
    fn rra(&mut self, mode: &AddressingMode) {
        self.ror(mode);
        self.adc(mode);
    }

    #[opcode(codes = [0x07, 0x17, 0x0F, 0x1F, 0x1B, 0x03, 0x13], name = "SLO", addr_mode)]
    fn slo(&mut self, mode: &AddressingMode) {
        self.asl(mode);
        self.ora(mode);
    }

    #[opcode(codes = [0x47, 0x57, 0x4F, 0x5F, 0x5B, 0x43, 0x53], name = "SRE", addr_mode)]
    fn sre(&mut self, mode: &AddressingMode) {
        self.lsr(mode);
        self.eor(mode);
    }

    #[opcode(codes = [0x9E, 0x9C], name = "SHX", addr_mode)]
    fn shx(&mut self, mode: &AddressingMode) {
        let (address, _pc) = self.get_operand_address(mode);
        let value = self.register_x & ((address >> 8) as u8 + 1);
        self.mem_write(address, value);
    }

    #[opcode(codes = [0x9C], name = "SHY", addr_mode)]
    fn shy(&mut self, mode: &AddressingMode) {
        let (address, _pc) = self.get_operand_address(mode);
        let value = self.register_y & ((address >> 8) as u8 + 1);
        self.mem_write(address, value);
    }

    #[opcode(codes = [0x8B], name = "XAA", addr_mode)]
    fn xaa(&mut self, _mode: &AddressingMode) {
        panic!("XAA is highly unstable and should not be used");
    }

    #[opcode(codes = [0x9B], name = "TAS", addr_mode)]
    fn tas(&mut self, mode: &AddressingMode) {
        let (address, _pc) = self.get_operand_address(mode);
        let value = self.register_a & self.register_x;
        self.stack_pointer = value;
        let result = value & ((address >> 8) as u8 + 1);
        self.mem_write(address, result);
    }

    fn update_zero_and_negative_flags(&mut self, register_value: u8) {
        if register_value == 0 {
            self.status.insert(StatusFlags::ZERO);
        } else {
            self.status.remove(StatusFlags::ZERO);
        }

        if register_value & 0b1000_0000 != 0 {
            self.status.insert(StatusFlags::NEGATIVE);
        } else {
            self.status.remove(StatusFlags::NEGATIVE);
        }
    }

    fn interrupt(&mut self, interrupt: interrupt::Interrupt) {
        self.stack_push_u16(self.program_counter);
        let mut flag = self.status.clone();
        flag.set(StatusFlags::BREAK, interrupt.b_flag_mask & 0b0010000 != 0);
        flag.set(StatusFlags::BREAK2, interrupt.b_flag_mask & 0b100000 != 0);

        self.stack_push_u8(flag.bits());
        self.status.insert(StatusFlags::INTERRUPT_DISABLE);

        self.bus.tick(interrupt.cpu_cycles);
        self.program_counter = self.u16_mem_read(interrupt.vector_addr);
    }

    pub fn get_actual_address(&mut self, mode: &AddressingMode, addr: u16) -> (u16, bool) {
        match mode {
            AddressingMode::ZeroPage => (self.mem_read(addr) as u16, false),
            AddressingMode::Absolute => (self.u16_mem_read(addr), false),
            AddressingMode::ZeroPageX => {
                let zero_page_address = self.mem_read(addr);
                let wrapping_add = zero_page_address.wrapping_add(self.register_x) as u16;
                (wrapping_add, false)
            }
            AddressingMode::ZeroPageY => {
                let zero_page_address = self.mem_read(addr);
                let wrapping_add = zero_page_address.wrapping_add(self.register_y) as u16;
                (wrapping_add, false)
            }
            AddressingMode::AbsoluteX => {
                let absolute_address = self.u16_mem_read(addr);
                let addr = absolute_address.wrapping_add(self.register_x as u16);
                (addr, page_crossed(absolute_address, addr))
            }
            AddressingMode::AbsoluteY => {
                let absolute_address = self.u16_mem_read(addr);
                let addr = absolute_address.wrapping_add(self.register_y as u16);
                (addr, page_crossed(absolute_address, addr))
            }
            AddressingMode::IndirectX => {
                let base = self.mem_read(addr);
                let ptr = base.wrapping_add(self.register_x);
                let lo = self.mem_read(ptr as u16) as u16;
                let hi = self.mem_read(ptr.wrapping_add(1) as u16) as u16;
                ((hi << 8) | lo, false)
            }
            AddressingMode::IndirectY => {
                let base = self.mem_read(addr);
                let lo = self.mem_read(base as u16) as u16;
                let hi = self.mem_read(base.wrapping_add(1) as u16) as u16;
                let deref_base = (hi << 8) | lo;
                let addr = deref_base.wrapping_add(self.register_y as u16);
                (addr, page_crossed(deref_base, addr))
            }
            AddressingMode::Accumulator => panic!("Accumulator should be handled separately"),
            _ => panic!("Invalid Addressing Mode"),
        }
    }

    pub fn get_operand_address(&mut self, mode: &AddressingMode) -> (u16, bool) {
        match mode {
            AddressingMode::Immediate => (self.program_counter, false),
            _ => self.get_actual_address(mode, self.program_counter),
        }
    }

    pub fn run(&mut self) {
        self.run_with_callback(|_| {});
    }

    pub fn run_with_callback<F>(&mut self, mut callback: F)
    where
        F: FnMut(&mut CPU),
    {
        let opcode_map: &HashMap<u8, &opcodes::OpCode> = &opcodes::CPU_OPS_CODES_MAP;
        loop {
            if let Some(_nmi) = self.bus.poll_nmi_status() {
                self.interrupt(interrupt::NMI);
            }

            callback(self);
            let code = self.mem_read(self.program_counter);
            self.program_counter += 1;
            let original_pc = self.program_counter;

            let opcode = opcode_map
                .get(&code)
                .unwrap_or_else(|| panic!("opcode not found: {}", code));

            match_all!(code);

            if self.status.contains(StatusFlags::BREAK) {
                break;
            }

            self.bus.tick(opcode.cycles);

            if original_pc == self.program_counter {
                self.program_counter += opcode.bytes as u16 - 1;
            }
        }
    }
}

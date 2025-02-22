use crate::{
    cpu::{AddressingMode, Mem, CPU},
    opcodes::CPU_OPS_CODES_MAP,
};

pub fn trace(cpu: &mut CPU) -> String {
    // C000  4C F5 C5 JMP $C5F5                         A:00 X:00 Y:00 P:24 SP:FB PPU:  0,  0 CYC:  0
    let opcodes = &CPU_OPS_CODES_MAP;

    let code = cpu.mem_read(cpu.program_counter);
    let opcode = opcodes.get(&code).unwrap_or_else(|| panic!("Unknown opcode: {:02X}", code));

    let begin = cpu.program_counter;
    let mut dump = vec![];
    dump.push(code);

    let (mem_addr, value) = match opcode.addr_mode {
        AddressingMode::Immediate
        | AddressingMode::NoneAddressing
        | AddressingMode::Accumulator => (0, 0),
        _ => {
            let (addr, _) = cpu.get_actual_address(&opcode.addr_mode, begin + 1);
            (addr, cpu.mem_read(addr))
        }
    };

    let tmp = match opcode.bytes {
        1 => match opcode.addr_mode {
            AddressingMode::Accumulator => "A ".to_string(),
            _ => String::new(),
        },
        2 => {
            let address = cpu.mem_read(begin + 1);
            dump.push(address);

            match opcode.addr_mode {
                AddressingMode::Immediate => format!("#${:02X}", address),
                AddressingMode::ZeroPage => format!("${:02X} = {:02X}", address, value),
                AddressingMode::ZeroPageX => {
                    format!("${:02X},X @ {:02X} = {:02X}", address, mem_addr, value)
                }
                AddressingMode::ZeroPageY => {
                    format!("${:02X},Y @ {:02X} = {:02X}", address, mem_addr, value)
                }
                AddressingMode::IndirectX => format!(
                    "(${:02X},X) @ {:02X} = {:04X} = {:02X}",
                    address,
                    address.wrapping_add(cpu.register_x),
                    mem_addr,
                    value
                ),
                AddressingMode::IndirectY => format!(
                    "(${:02X}),Y = {:04X} @ {:04X} = {:02X}",
                    address,
                    mem_addr.wrapping_sub(cpu.register_y as u16),
                    mem_addr,
                    value
                ),
                AddressingMode::NoneAddressing => {
                    let address = (begin as usize + 2).wrapping_add((address as i8) as usize);
                    format!("${:04X}", address)
                }
                _ => String::new(),
            }
        }
        3 => {
            let lo = cpu.mem_read(begin + 1);
            let hi = cpu.mem_read(begin + 2);
            dump.push(lo);
            dump.push(hi);

            let address = cpu.u16_mem_read(begin + 1);
            match opcode.addr_mode {
                AddressingMode::NoneAddressing => {
                    if opcode.name == "JMP" {
                        let jmp_addr = if address & 0x00ff == 0x00FF {
                            let lo = cpu.mem_read(address);
                            let hi = cpu.mem_read(address & 0xff00);
                            (hi as u16) << 8 | (lo as u16)
                        } else {
                            cpu.u16_mem_read(address)
                        };
                        format!("(${:04X}) = {:04X}", address, jmp_addr)
                    } else {
                        format!("${:04X}", address)
                    }
                }
                AddressingMode::Absolute => {
                    if opcode.name == "JMP" {
                        format!("${:04X}", address)
                    } else {
                        format!("${:04X} = {:02X}", address, value)
                    }
                }
                AddressingMode::AbsoluteX => {
                    format!("${:04X},X @ {:04X} = {:02X}", address, mem_addr, value)
                }
                AddressingMode::AbsoluteY => {
                    format!("${:04X},Y @ {:04X} = {:02X}", address, mem_addr, value)
                }
                _ => panic!("Invalid addressing mode"),
            }
        }
        _ => String::new(),
    };

    let hex_str = dump
        .iter()
        .map(|z| format!("{:02x}", z))
        .collect::<Vec<String>>()
        .join(" ");
    let asm_str = format!("{:04x}  {:8} {: >4} {}", begin, hex_str, opcode.name, tmp)
        .trim()
        .to_string();
    format!(
        "{:47} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X}",
        asm_str, cpu.register_a, cpu.register_x, cpu.register_y, cpu.status, cpu.stack_pointer
    ).to_ascii_uppercase()
}

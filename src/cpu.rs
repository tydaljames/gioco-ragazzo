use crate::mmu::Mmu;

pub struct Registers {
    pub a: u8, //Accumulator
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub f: u8, //Stores CPU flags; Special register
    pub h: u8,
    pub l: u8,
}

impl Registers {
    pub fn new() -> Self {
        Self {
            a: 0x01,
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            f: 0xB0,
            h: 0x01,
            l: 0x4D,
        }
    }

    // Big-endian-like access for combined registers
    // If bc = 0xABCD, then b = AB, c = CD
    pub fn get_bc(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }

    pub fn set_bc(&mut self, value: u16) {
        self.b = ((value & 0xFF00) >> 8) as u8;
        self.c = (value & 0x00FF) as u8;
    }

    pub fn get_de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }

    pub fn set_de(&mut self, value: u16) {
        self.d = ((value & 0xFF00) >> 8) as u8;
        self.e = (value & 0x00FF) as u8;
    }

    pub fn get_hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }

    pub fn set_hl(&mut self, value: u16) {
        self.h = ((value & 0xFF00) >> 8) as u8;
        self.l = (value & 0x00FF) as u8;
    }

    pub fn get_af(&self) -> u16 {
        ((self.a as u16) << 8) | (self.f as u16)
    }

    pub fn set_af(&mut self, value: u16) {
        self.a = ((value & 0xFF00) >> 8) as u8;

        // Bottom 4 bits of F register are permanently wired to 0.
        // Mask with 0x00F0 so these bottom 4 bits are always 0.
        self.f = (value & 0x00F0) as u8;
    }

    pub fn get_zero_flag(&self) -> bool {
        // Check if bit 7 is 1
        (self.f & 0b1000_0000) != 0
    }
    pub fn set_zero_flag(&mut self, value: bool) {
        if value {
            self.f |= 0b1000_0000;
        }
        else {
            self.f &= 0b0111_1111;
        }
    }

    pub fn get_sub_flag(&self) -> bool {
        // Check if bit 6 is 1
        (self.f & 0b0100_0000) != 0
    }
    pub fn set_sub_flag(&mut self, value: bool) {
        if value {
            self.f |= 0b0100_0000;
        }
        else {
            self.f &= 0b1011_1111;
        }
    }

    pub fn get_half_carry_flag(&self) -> bool {
        // Check if bit 5 is 1
        (self.f & 0b0010_0000) != 0
    }
    pub fn set_half_carry_flag(&mut self, value: bool) {
        if value {
            self.f |= 0b0010_0000;
        }
        else {
            self.f &= 0b1101_1111;
        }
    }

    pub fn get_carry_flag(&self) -> bool {
        // Check if bit 4 is 1
        (self.f & 0b0001_0000) != 0
    }
    pub fn set_carry_flag(&mut self, value: bool) {
        if value {
            self.f |= 0b0001_0000;
        }
        else {
            self.f &= 0b1110_1111;
        }
    }
}

pub struct Cpu {
    pub registers: Registers,
    pub pc: u16,
    pub sp: u16,
    pub mmu: Mmu,
}

impl Cpu {
    pub fn new(mmu: Mmu) -> Self {
        Self {
            registers: Registers::new(),
            pc: 0x0100,
            sp: 0xFFFE,
            mmu,
        }
    }

    pub fn step(&mut self) -> u8 {
        // 1. FETCH: Read byte at PC
        let opcode = self.mmu.read_byte(self.pc);

        // Advance PC to next byte
        self.pc = self.pc.wrapping_add(1);

        // 2. DECODE and EXECUTE
        let cycles = self.execute(opcode);

        // 3. RETURN T-CYCLES (used to later sync graphics and timers)
        cycles
    }

    fn execute(&mut self, opcode: u8) -> u8 {
        match opcode {
            // NOP (0x00): No Operation
            0x00 => 4, //Takes 4 T-cycles, does nothing

            // LD B, d8 (LD into register from memory)
            0x06 => {
                let value = self.mmu.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                self.registers.b = value;
                8
            }

            // LD A, B (LD into register from register)
            0x78 => {
                self.registers.a = self.registers.b;
                4
            }

            // Pattern C: Arithmetic & Flag Updates (INC A)
            // Example: 0x3C — INC A (Increment register A by 1)
            // When performing math, you must update the flags register (F):
            //     Zero Flag (Z): Set if result becomes 0.
            //     Subtract Flag (N): Cleared (false) because this was addition/increment.
            //     Half-Carry Flag (H): Set if lower 4 bits overflow ((a & 0x0F) == 0x0F).
            0x3C => {
                let old_val = self.registers.a;
                let new_val = old_val.wrapping_add(1);
                self.registers.a = new_val;

                // Set/clear flags
                self.registers.set_zero_flag(new_val == 0);
                self.registers.set_sub_flag(false);
                self.registers.set_half_carry_flag((old_val & 0x0F) == 0x0F);

                4
            }

            // JP a16 (Jump to 16-bit address)
            0xC3 => {
                let target_addr = self.mmu.read_word(self.pc);
                self.pc = target_addr;

                16 // 16 T-cycles (4 fetch opcode, 8 read 16-bit target, 4 internal CPU step)
            }

            // Prefix byte. Read from secondary 256 set list of opcodes when given this opcode.
            0xCB => {
                // Read the secondary opcode byte
                let cb_opcode = self.mmu.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);

                self.execute_cb(cb_opcode)
            }


            // Add opcode cases here later

            // Crash handler for unwritten opcodes
            _ => panic!(
                "Unimplemented opcode 0x{:02X} at address 0x{:04X}",
                opcode,
                self.pc.wrapping_sub(1)
            )
        }
    }

    // Handles cb opcodes.
    fn execute_cb(&mut self, opcode: u8) -> u8 {
        match opcode {
            // Crash handler for unwritten opcodes
            _ => panic!(
                "Unimplemented opcode 0x{:02X} at address 0x{:04X}",
                opcode,
                self.pc.wrapping_sub(1)
            )
        }
    }

    pub fn debug_print_state(&self) {
        // Read upcoming opcode without advancing the PC
        let opcode = self.mmu.read_byte(self.pc);

        // Print the PC, Opcode, and all Registers in a clean format
        println!(
            "A: {:02X} F: {:02X} B: {:02X} C: {:02X} D: {:02X} E: {:02X} H: {:02X} L: {:02X} SP: {:04X} PC: 00:{:04X} ({:02X})",
            self.registers.a, self.registers.f,
            self.registers.b, self.registers.c,
            self.registers.d, self.registers.e,
            self.registers.h, self.registers.l,
            self.sp, self.pc, opcode
        );
    }
}


// af = 0xABCD
//
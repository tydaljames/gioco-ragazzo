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

    pub fn get_hc_flag(&self) -> bool {
        // Check if bit 5 is 1
        (self.f & 0b0010_0000) != 0
    }
    pub fn set_hc_flag(&mut self, value: bool) {
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
        }
    }
}


// af = 0xABCD
//
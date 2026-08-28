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
    pub halted: bool,
    pub ime: bool,
}

impl Cpu {
    pub fn new(mmu: Mmu) -> Self {
        Self {
            registers: Registers::new(),
            pc: 0x0100,
            sp: 0xFFFE,
            mmu,
            halted: false,
            ime: false,
        }
    }

    /// Pushes a 16-bit value onto the Stack
    fn stack_push(&mut self, val: u16) {
        // High byte goes first on the Game Boy stack
        self.sp = self.sp.wrapping_sub(1);
        self.mmu.write_byte(self.sp, ((val & 0xFF00) >> 8) as u8);

        self.sp = self.sp.wrapping_sub(1);
        self.mmu.write_byte(self.sp, (val & 0x00FF) as u8);
    }

    /// Pops a 16-bit value from the Stack
    fn stack_pop(&mut self) -> u16 {
        let low = self.mmu.read_byte(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);

        let high = self.mmu.read_byte(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);

        (high << 8) | low
    }

    pub fn step(&mut self) -> u8 {

        // if self.halted {
        //     return 4
        // }

        // 1. FETCH: Read byte at PC
        let opcode = self.mmu.read_byte(self.pc);

        // Advance PC to next byte
        self.pc = self.pc.wrapping_add(1);

        // 2. DECODE and EXECUTE
        let cycles = self.execute(opcode);

        // 3. RETURN T-CYCLES (used to later sync graphics and timers)
        cycles
    }

    // Every Game Boy opcode is structured into octal parts: xxyyyzzz.
    // x = (opcode >> 6) & 0b11 (Determines the main family of instructions)
    // y = (opcode >> 3) & 0b111 (Determines the destination register or condition)
    // z = opcode & 0b111 (Determines the source register or minor operation)
    fn execute(&mut self, opcode: u8) -> u8 {
        let x = OpcodeDecoder::get_x(opcode);
        let y = OpcodeDecoder::get_y(opcode);
        let z = OpcodeDecoder::get_z(opcode);

        match x {
            // x = 00: Miscellaneous / Control / 16-bit Block
            0 => {
                self.opcodes_x1(y, z)
            }

            // x == 01: LD r1, r2 (Load into r1 from r2)
            1 => {
                // SPECIAL: Halt instruction. Has a known bug in actual hardware. May need to review later!
                if y == 6 && z == 6 {
                    self.halted = true;
                    return 4;
                }

                let value = self.read_reg_8bit(z);
                self.write_reg_8bit(y, value);

                if y == 6 || z == 6 {8} else {4}
            }

            // x == 10: ALU instructions rA, r2
            2 => {
                let operand = self.read_reg_8bit(z);
                self.execute_alu_operation(y, operand);

                if z == 6 {8} else {4}
            }

            // x = 11: Jumps, Calls, Returns, Stack, and Restarts
            3 => {
                self.opcodes_x3(y, z)
            }

            // Crash handler for unwritten opcodes
            _ => panic!(
                "Unimplemented CB opcode 0x{:02X} at address 0x{:04X}",
                opcode,
                self.pc.wrapping_sub(1)
            )
        }
    }

    fn opcodes_x1(&mut self, y: u8, z: u8) -> u8 {
        match z {
            0 => {
                // z = 0: NOP, STOP, and Relative Jumps
                match y {
                    0 => 4, // NOP
                    1 => { // LD (a16), SP
                        let addr = self.mmu.read_word(self.pc);
                        self.pc = self.pc.wrapping_add(2);
                        self.mmu.write_word(addr, self.sp);
                        20
                    }
                    2 => { // STOP
                        self.pc = self.pc.wrapping_add(1);
                        4
                    }
                    3 => { // JR d8 (Unconditional Relative Jump)
                        let offset = self.mmu.read_byte(self.pc) as i8;
                        self.pc = self.pc.wrapping_add(1);
                        self.pc = (self.pc as i32 + offset as i32) as u16;
                        12
                    }
                    4..=7 => { // JR cc, d8 (Conditional Relative Jumps)
                        let offset = self.mmu.read_byte(self.pc) as i8;
                        self.pc = self.pc.wrapping_add(1);

                        if self.check_condition(y - 4) {
                            self.pc = (self.pc as i32 + offset as i32) as u16;
                            12 // Branch taken takes longer
                        } else {
                            8  // Branch not taken
                        }
                    }
                    _ => unreachable!(),
                }
            }

            1 => {
                // z = 1: 16-bit Load Immediate and ADD HL, r16
                let p = y >> 1; // Divide by 2 to get the register index (0-3)
                let q = y % 2;  // Odd or even?

                if q == 0 {
                    // q = 0: LD r16, d16
                    let val = self.mmu.read_word(self.pc);
                    self.pc = self.pc.wrapping_add(2);
                    self.write_reg_16bit(p, val);
                    12
                } else {
                    // q = 1: ADD HL, r16
                    let hl = self.registers.get_hl();
                    let val = self.read_reg_16bit(p);
                    let result = hl.wrapping_add(val);
                    self.registers.set_hl(result);

                    self.registers.set_sub_flag(false);
                    // Half carry for 16-bit addition occurs at bit 11, not bit 3!
                    self.registers.set_half_carry_flag((hl & 0x0FFF) + (val & 0x0FFF) > 0x0FFF);
                    self.registers.set_carry_flag((hl as u32) + (val as u32) > 0xFFFF);
                    8
                }
            }

            2 => {
                // z = 2: Indirect Loads (LD (r16), A and LD A, (r16))
                let p = y >> 1;
                let q = y % 2;

                // Determine the memory address based on `p`
                let addr = match p {
                    0 => self.registers.get_bc(),
                    1 => self.registers.get_de(),
                    2 => { // HL+, read HL then increment it
                        let hl = self.registers.get_hl();
                        self.registers.set_hl(hl.wrapping_add(1));
                        hl
                    }
                    3 => { // HL-, read HL then decrement it
                        let hl = self.registers.get_hl();
                        self.registers.set_hl(hl.wrapping_sub(1));
                        hl
                    }
                    _ => unreachable!(),
                };

                if q == 0 { // LD (r16), A
                    self.mmu.write_byte(addr, self.registers.a);
                } else { // LD A, (r16)
                    self.registers.a = self.mmu.read_byte(addr);
                }
                8
            }

            3 => {
                // z = 3: 16-bit Increment and Decrement
                let p = y >> 1;
                let q = y % 2;

                let val = self.read_reg_16bit(p);
                if q == 0 { // INC r16
                    self.write_reg_16bit(p, val.wrapping_add(1));
                } else { // DEC r16
                    self.write_reg_16bit(p, val.wrapping_sub(1));
                }
                // Note: 16-bit INC/DEC do NOT affect any flags on the Game Boy!
                8
            }

            4 => {
                // z = 4: INC r8
                let val = self.read_reg_8bit(y);
                let result = val.wrapping_add(1);
                self.write_reg_8bit(y, result);

                self.registers.set_zero_flag(result == 0);
                self.registers.set_sub_flag(false);
                self.registers.set_half_carry_flag((val & 0x0F) == 0x0F);

                if y == 6 { 12 } else { 4 }
            }

            5 => {
                // z = 5: DEC r8
                let val = self.read_reg_8bit(y);
                let result = val.wrapping_sub(1);
                self.write_reg_8bit(y, result);

                self.registers.set_zero_flag(result == 0);
                self.registers.set_sub_flag(true);
                self.registers.set_half_carry_flag((val & 0x0F) == 0);

                if y == 6 { 12 } else { 4 }
            }

            6 => {
                // z = 6: LD r8, d8
                let immediate_val = self.mmu.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                self.write_reg_8bit(y, immediate_val);
                if y == 6 { 12 } else { 8 }
            }

            7 => {
                // z = 7: Rotate, Shift, and Flag operations on A
                match y {
                    0 => { // RLCA (Rotate A Left)
                        let a = self.registers.a;
                        let carry = (a & 0x80) >> 7;
                        self.registers.a = (a << 1) | carry;

                        self.registers.set_zero_flag(false); // Hardware quirk: these clear the Z flag!
                        self.registers.set_sub_flag(false);
                        self.registers.set_half_carry_flag(false);
                        self.registers.set_carry_flag(carry == 1);
                        4
                    }
                    1 => { // RRCA (Rotate A Right)
                        let a = self.registers.a;
                        let carry = a & 0x01;
                        self.registers.a = (a >> 1) | (carry << 7);

                        self.registers.set_zero_flag(false);
                        self.registers.set_sub_flag(false);
                        self.registers.set_half_carry_flag(false);
                        self.registers.set_carry_flag(carry == 1);
                        4
                    }
                    2 => { // RLA (Rotate A Left through Carry)
                        let a = self.registers.a;
                        let old_carry = if self.registers.get_carry_flag() { 1 } else { 0 };
                        let new_carry = (a & 0x80) >> 7;
                        self.registers.a = (a << 1) | old_carry;

                        self.registers.set_zero_flag(false);
                        self.registers.set_sub_flag(false);
                        self.registers.set_half_carry_flag(false);
                        self.registers.set_carry_flag(new_carry == 1);
                        4
                    }
                    3 => { // RRA (Rotate A Right through Carry)
                        let a = self.registers.a;
                        let old_carry = if self.registers.get_carry_flag() { 1 } else { 0 };
                        let new_carry = a & 0x01;
                        self.registers.a = (a >> 1) | (old_carry << 7);

                        self.registers.set_zero_flag(false);
                        self.registers.set_sub_flag(false);
                        self.registers.set_half_carry_flag(false);
                        self.registers.set_carry_flag(new_carry == 1);
                        4
                    }
                    4 => {
                        // DAA (Decimal Adjust Accumulator)
                        // Transforms hex addition (e.g., 0x09 + 0x01 = 0x0A) into
                        // Binary Coded Decimal (0x09 + 0x01 = 0x10).
                        let mut a = self.registers.a;
                        let mut adjust = 0;
                        let mut carry = false;

                        if self.registers.get_sub_flag() { // After a subtraction
                            if self.registers.get_carry_flag() {
                                adjust -= 0x60;
                                carry = true;
                            }
                            if self.registers.get_half_carry_flag() { adjust -= 0x06; }
                        } else { // After an addition
                            if self.registers.get_carry_flag() || a > 0x99 {
                                adjust += 0x60;
                                carry = true;
                            }
                            if self.registers.get_half_carry_flag() || (a & 0x0F) > 0x09 { adjust += 0x06; }
                        }

                        a = a.wrapping_add(adjust as u8);
                        self.registers.a = a;

                        self.registers.set_zero_flag(a == 0);
                        self.registers.set_half_carry_flag(false); // DAA always clears H
                        self.registers.set_carry_flag(carry);
                        4
                    }
                    5 => { // CPL (Complement A)
                        self.registers.a = !self.registers.a;
                        self.registers.set_sub_flag(true);
                        self.registers.set_half_carry_flag(true);
                        4
                    }
                    6 => { // SCF (Set Carry Flag)
                        self.registers.set_sub_flag(false);
                        self.registers.set_half_carry_flag(false);
                        self.registers.set_carry_flag(true);
                        4
                    }
                    7 => { // CCF (Complement Carry Flag)
                        self.registers.set_sub_flag(false);
                        self.registers.set_half_carry_flag(false);
                        self.registers.set_carry_flag(!self.registers.get_carry_flag());
                        4
                    }
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
    }

    fn opcodes_x3(&mut self, y: u8, z: u8) -> u8 {
        match z {
            0 => {
                // z = 0: Conditional Returns (RET cc)
                // y specifies the condition: 0=NZ, 1=Z, 2=NC, 3=C
                if self.check_condition(y) {
                    let target = self.stack_pop();
                    self.pc = target;
                    20 // Return taken is slow
                } else {
                    8  // Return not taken is fast
                }
            }

            1 => {
                // z = 1: Pop Register Pair, RET, RETI, or JP HL
                let p = y >> 1;
                let q = y % 2;

                if q == 0 {
                    // POP r16 (Note: p=3 pops AF, where lower 4 bits of F are masked)
                    let val = self.stack_pop();
                    if p == 3 {
                        self.registers.set_af(val);
                    } else {
                        // Map p to BC, DE, HL
                        match p {
                            0 => self.registers.set_bc(val),
                            1 => self.registers.set_de(val),
                            2 => self.registers.set_hl(val),
                            _ => unreachable!(),
                        }
                    }
                    12
                } else {
                    match p {
                        0 => { // RET (Unconditional Return)
                            self.pc = self.stack_pop();
                            16
                        }
                        1 => { // RETI (Return and Enable Interrupts)
                            self.pc = self.stack_pop();
                            self.ime = true; // Instantly re-enables interrupts
                            16
                        }
                        2 => { // JP (HL) - Jump directly to address in HL
                            self.pc = self.registers.get_hl();
                            4
                        }
                        3 => { // LD SP, HL (Load HL into Stack Pointer)
                            self.sp = self.registers.get_hl();
                            8
                        }
                        _ => unreachable!(),
                    }
                }
            }

            2 => {
                // z = 2: Conditional Jumps (JP cc, a16)
                let addr = self.mmu.read_word(self.pc);
                self.pc = self.pc.wrapping_add(2);

                if self.check_condition(y) {
                    self.pc = addr;
                    16 // Jump taken
                } else {
                    12 // Jump not taken
                }
            }

            3 => {
                // z = 3: Absolute Jumps, CB prefix, and Illegal instructions
                match y {
                    0 => { // JP a16 (Unconditional Absolute Jump)
                        let addr = self.mmu.read_word(self.pc);
                        self.pc = addr;
                        16
                    }
                    1 => {
                        // CB PREFIX! (This delegates to the 0xCB bit-manipulation engine we built earlier)
                        let cb_opcode = self.mmu.read_byte(self.pc);
                        self.pc = self.pc.wrapping_add(1);
                        self.execute_cb(cb_opcode)
                    }
                    6 => {
                        // DI (Disable Interrupts)
                        self.ime = false;
                        4
                    }
                    7 => {
                        // EI (Enable Interrupts)
                        self.ime = true;
                        4
                    }
                    // y = 2 through 7 are illegal opcodes on the Game Boy
                    _ => panic!("Illegal opcode encountered at 0x{:04X}. x = {}, y = {}, z = {}", self.pc.wrapping_sub(1), 3, y, z)
                }
            }

            4 => {
                if y >= 4 {
                    panic!("Illegal opcode encountered at 0x{:04X}. x = {}, y = {}, z = {}", self.pc.wrapping_sub(1), 3, y, z)
                }

                // z = 4: Conditional Calls (CALL cc, a16)
                let addr = self.mmu.read_word(self.pc);
                self.pc = self.pc.wrapping_add(2);

                if self.check_condition(y) {
                    self.stack_push(self.pc);
                    self.pc = addr;
                    24 // Call taken
                } else {
                    12 // Call not taken
                }
            }

            5 => {
                if y == 3 || y == 5 || y == 7 {
                    panic!("Illegal opcode encountered at 0x{:04X}. x = {}, y = {}, z = {}", self.pc.wrapping_sub(1), 3, y, z)
                }

                // z = 5: Push Register Pair or Unconditional CALL
                let p = y >> 1;
                let q = y % 2;

                if q == 0 {
                    // PUSH r16
                    let val = match p {
                        0 => self.registers.get_bc(),
                        1 => self.registers.get_de(),
                        2 => self.registers.get_hl(),
                        3 => self.registers.get_af(),
                        _ => unreachable!(),
                    };
                    self.stack_push(val);
                    16
                } else {
                    // CALL a16 (Unconditional Call)
                    let addr = self.mmu.read_word(self.pc);
                    self.pc = self.pc.wrapping_add(2);

                    self.stack_push(self.pc);
                    self.pc = addr;
                    24
                }
            }

            6 => {
                // z = 6: ALU Operations with Immediate values (e.g., ADD A, d8, SUB d8)
                // Notice this reuses our execute_alu_operation helper from block x = 10!
                let operand = self.mmu.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);

                self.execute_alu_operation(y, operand);
                8
            }

            7 => {
                // z = 7: RST (Restart - Fast internal function calls to 0x0000, 0x0008, etc.)
                // y specifies which vector (0, 8, 16, 24, 32, 40, 48, 56)
                let target_addr = (y as u16) * 8;
                self.stack_push(self.pc);
                self.pc = target_addr;
                16
            }

            _ => unreachable!(),
        }
    }

    // Handles cb opcodes.
    fn execute_cb(&mut self, opcode: u8) -> u8 {
        let x = OpcodeDecoder::get_x(opcode);
        let y = OpcodeDecoder::get_y(opcode);
        let z = OpcodeDecoder::get_z(opcode);

        let mut value = self.read_reg_8bit(z);

        match x {
            // xx == 00: Rotations and shifts. Use "y" to determine which specific shift.
            0 => {


                // Come back later!

                // self.registers.set_zero_flag(value == 0);
                // self.registers.set_sub_flag(false);
                // self.registers.set_half_carry_flag(false);
                //
                // // Unset if SWAP
                // self.registers.set_carry_flag(value == 0);



                if z == 6 {16} else {8}
            }

            // xx == 01: BIT b, r (test bit 'y' of register 'z')
            1 => {
                let bit_mask = 1 << y;

                self.registers.set_zero_flag((value & bit_mask) == 0);
                self.registers.set_sub_flag(false);
                self.registers.set_half_carry_flag(true);

                if z == 6 {12} else {8}
            }

            // xx == 10: RES b, r (Reset bit 'y' of register 'z' to 0)
            2 => {
                let bit_mask = !(1 << y);
                value &= bit_mask;
                self.write_reg_8bit(z, value);

                if z == 6 {16} else {8}
            }

            // xx == 11: SET b, r (Set bit 'y' of register 'z' to 1)
            3 => {
                let bit_mask = 1 << y;
                value |= bit_mask;
                self.write_reg_8bit(z, value);

                if z == 6 {16} else {8}
            }

            // Crash handler for unwritten opcodes
            _ => panic!(
                "Unimplemented CB opcode 0x{:02X} at address 0x{:04X}",
                opcode,
                self.pc.wrapping_sub(1)
            )
        }
    }

    fn read_reg_8bit(&self, index: u8) -> u8 {
        match index {
            0 => self.registers.b,
            1 => self.registers.c,
            2 => self.registers.d,
            3 => self.registers.e,
            4 => self.registers.h,
            5 => self.registers.l,
            6 => self.mmu.read_byte(self.registers.get_hl()), // Read from memory at (HL)
            7 => self.registers.a,
            _ => unreachable!("Register index must be 0-7"),
        }
    }

    /// Writes a value based on a 3-bit register index.
    fn write_reg_8bit(&mut self, index: u8, value: u8) {
        match index {
            0 => self.registers.b = value,
            1 => self.registers.c = value,
            2 => self.registers.d = value,
            3 => self.registers.e = value,
            4 => self.registers.h = value,
            5 => self.registers.l = value,
            6 => self.mmu.write_byte(self.registers.get_hl(), value), // Write to memory at (HL)
            7 => self.registers.a = value,
            _ => unreachable!("Register index must be 0-7"),
        }
    }

    /// Decodes a 2-bit index (p) into a 16-bit register value
    fn read_reg_16bit(&self, p: u8) -> u16 {
        match p {
            0 => self.registers.get_bc(),
            1 => self.registers.get_de(),
            2 => self.registers.get_hl(),
            3 => self.sp,
            _ => unreachable!("16-bit register index must be 0-3"),
        }
    }

    /// Writes a value to a 16-bit register based on a 2-bit index (p)
    fn write_reg_16bit(&mut self, p: u8, val: u16) {
        match p {
            0 => self.registers.set_bc(val),
            1 => self.registers.set_de(val),
            2 => self.registers.set_hl(val),
            3 => self.sp = val,
            _ => unreachable!("16-bit register index must be 0-3"),
        }
    }

    /// Evaluates jump conditions: 0 = NZ, 1 = Z, 2 = NC, 3 = C
    fn check_condition(&self, cc: u8) -> bool {
        match cc {
            0 => !self.registers.get_zero_flag(),   // NZ (Not Zero)
            1 => self.registers.get_zero_flag(),    // Z (Zero)
            2 => !self.registers.get_carry_flag(),  // NC (Not Carry)
            3 => self.registers.get_carry_flag(),   // C (Carry)
            _ => unreachable!("Condition code must be 0-3"),
        }
    }

    fn execute_alu_operation(&mut self, operation: u8, operand: u8) {
        let a = self.registers.a;

        match operation {
            0 => { // ADD A, n
                let result = a.wrapping_add(operand);
                self.registers.a = result;
                self.registers.set_zero_flag(result == 0);
                self.registers.set_sub_flag(false);
                // Half-carry happens if bits 0-3 overflow (sum > 15)
                self.registers.set_half_carry_flag((a & 0x0F) + (operand & 0x0F) > 0x0F);
                // Carry happens if the full 8 bits overflow (sum > 255)
                self.registers.set_carry_flag(a as u16 + operand as u16 > 0xFF);
            }
            1 => { // ADC A, n (Add with Carry)
                let carry = if self.registers.get_carry_flag() { 1 } else { 0 };
                let result = a.wrapping_add(operand).wrapping_add(carry);
                self.registers.a = result;

                self.registers.set_zero_flag(result == 0);
                self.registers.set_sub_flag(false);
                self.registers.set_half_carry_flag((a & 0x0F) + (operand & 0x0F) + carry > 0x0F);
                self.registers.set_carry_flag(a as u16 + operand as u16 + carry as u16 > 0xFF);
            }
            2 => { // SUB A, n
                let result = a.wrapping_sub(operand);
                self.registers.a = result;

                self.registers.set_zero_flag(result == 0);
                self.registers.set_sub_flag(true);
                // Half-carry for subtraction: if lower nibble borrow is needed
                self.registers.set_half_carry_flag((a & 0x0F) < (operand & 0x0F));
                self.registers.set_carry_flag(a < operand);
            }
            3 => { // SBC A, n (Subtract with Carry)
                let carry = if self.registers.get_carry_flag() { 1 } else { 0 };
                let result = a.wrapping_sub(operand).wrapping_sub(carry);
                self.registers.a = result;

                self.registers.set_zero_flag(result == 0);
                self.registers.set_sub_flag(true);
                self.registers.set_half_carry_flag((a & 0x0F) < (operand & 0x0F) + carry);
                self.registers.set_carry_flag((a as u16) < (operand as u16) + (carry as u16));
            }
            4 => { // AND A, n
                let result = a & operand;
                self.registers.a = result;

                self.registers.set_zero_flag(result == 0);
                self.registers.set_sub_flag(false);
                self.registers.set_half_carry_flag(true); // AND always sets H to 1
                self.registers.set_carry_flag(false);     // AND always clears C
            }
            5 => { // XOR A, n
                let result = a ^ operand;
                self.registers.a = result;

                self.registers.set_zero_flag(result == 0);
                self.registers.set_sub_flag(false);
                self.registers.set_half_carry_flag(false);
                self.registers.set_carry_flag(false);
            }
            6 => { // OR A, n
                let result = a | operand;
                self.registers.a = result;

                self.registers.set_zero_flag(result == 0);
                self.registers.set_sub_flag(false);
                self.registers.set_half_carry_flag(false);
                self.registers.set_carry_flag(false);
            }
            7 => { // CP A, n (Compare)
                // Compare is exactly like SUB, but it ONLY updates flags (doesn't save to A)
                let result = a.wrapping_sub(operand);

                self.registers.set_zero_flag(result == 0);
                self.registers.set_sub_flag(true);
                self.registers.set_half_carry_flag((a & 0x0F) < (operand & 0x0F));
                self.registers.set_carry_flag(a < operand);
            }
            _ => unreachable!(),
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

pub struct OpcodeDecoder;

impl OpcodeDecoder {
    // Extract the "xx" bits (6-7) from an opcode.
    pub fn get_x(opcode: u8) -> u8 {
        (opcode >> 6) & 0b11
    }

    // Extract the "yyy" bits (5-3) from an opcode.
    pub fn get_y(opcode: u8) -> u8 {
        (opcode >> 3) & 0b111
    }

    // Extract the "zzz" bits (0-2) from an opcode.
    pub fn get_z(opcode: u8) -> u8 {
        opcode & 0b111
    }
}


// af = 0xABCD
//
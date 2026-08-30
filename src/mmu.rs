use std::fs;
use std::path::Path;

pub struct Cartridge {
    pub rom: Vec<u8>
}

impl Cartridge {
    // This function will read a file from the computer
    // and return a Result. If successful, gives us a Cartridge
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let rom = fs::read(path)?;
        Ok(Self { rom })
    }
}

pub struct Mmu {
    cartridge: Cartridge,
    vram: [u8; 0x2000], // 8,196 bytes/ 8 KB video RAM
    wram: [u8; 0x2000], // 8 KB working RAM
    hram: [u8; 0x7F], // 127 bytes of high-ram (fast CPU operations)
    if_register: u8,
    ie_register: u8, // Single byte for interrupts
    serial_data: u8,
    serial_control: u8,
    print_buffer: String,
    pub div_counter: u16,
    pub tima: u8,
    pub tma: u8,
    pub tac: u8,
    pub tima_counter: i32,
}

impl Mmu {
    // Constructor to create new MMU and initialize arrays to 0
    pub fn new(cartridge: Cartridge) -> Self {
        Self {
            cartridge,
            vram: [0; 0x2000],
            wram: [0; 0x2000],
            hram: [0; 0x7F],
            if_register: 0,
            ie_register: 0,
            serial_data: 0,
            serial_control: 0,
            print_buffer: String::new(),
            div_counter: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            tima_counter: 0,
        }
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            // Ignore write attempts to ROM
            0x0000..=0x7FFF => self.cartridge.rom[addr as usize],

            // Video RAM (VRAM)
            0x8000..=0x9FFF => {
                let index = (addr - 0x8000) as usize;
                self.vram[index]
            }

            // Work RAM (WRAM)
            0xC000..=0xDFFF => {
                let index = (addr - 0xC000) as usize;
                self.wram[index]
            }

            0xFF01 => self.serial_data,
            0xFF02 => self.serial_control,

            0xFF04 => (self.div_counter >> 8) as u8, // DIV is the upper 8 bits of the 16-bit counter
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.tac | 0xF8, // Unused upper 5 bits always return 1

            0xFF0F => self.if_register | 0xE0, // Top 3 bits permanently set to 1
            0xFFFF => self.ie_register,

            // High RAM (HRAM)
            0xFF80..=0xFFFE => {
                let index = (addr - 0xFF80) as usize;
                self.hram[index]
            }

            // Interrupt Enable Register (IE)
            0xFFFF => self.ie_register,

            _ => 0xFF
        }
    }

    pub fn write_byte(&mut self, addr: u16, val: u8) {
        match addr {
            // Ignore write attempts to ROM
            0x0000..=0x7FFF => {},

            // Video RAM (VRAM)
            0x8000..=0x9FFF => {
                let index = (addr - 0x8000) as usize;
                self.vram[index] = val;
            }

            // Work RAM (WRAM)
            0xC000..=0xDFFF => {
                let index = (addr - 0xC000) as usize;
                self.wram[index] = val;
            }

            // Intercept Serial Output for Blargg's Tests
            0xFF01 => {
                self.serial_data = val;
            }
            0xFF02 => {
                if val == 0x81 {
                    let ch = self.serial_data as char;
                    self.print_buffer.push(ch);
                    // print!("{}", self.serial_data as char);

                    if ch == '\n' {
                        print!("{}", self.print_buffer);
                        // Flush stdout so the text appears in your terminal immediately
                        use std::io::Write;
                        std::io::stdout().flush().unwrap();
                    }
                }
            }

            0xFF04 => self.div_counter = 0, // Writing anything to DIV resets it to 0
            0xFF05 => self.tima = val,
            0xFF06 => self.tma = val,
            0xFF07 => self.tac = val & 0x07, // Only bottom 3 bits are writeable

            0xFF0F => self.if_register = val,
            0xFFFF => self.ie_register = val,

            // High RAM (HRAM)
            0xFF80..=0xFFFE => {
                let index = (addr - 0xFF80) as usize;
                self.hram[index] = val;
            }

            // Interrupt Enable Register (IE)
            0xFFFF => self.ie_register = val,

            // Do nothing for all other unimplemented memory regions (I/O, Echo RAM, OAM, etc.)
            _ => {},
        }
    }

    // Little Endian.
    // 0xABCD, low_byte = CD, high_byte = AB
    pub fn read_word(&self, addr: u16) -> u16 {
        let low_byte = self.read_byte(addr) as u16;
        let high_byte = self.read_byte(addr + 1) as u16;

        (high_byte << 8) | low_byte
    }

    pub fn write_word(&mut self, addr: u16, val: u16) {
        let low_byte = (0x00FF & val) as u8;
        let high_byte = ((0xFF00 & val) >> 8) as u8;

        self.write_byte(addr, low_byte);
        self.write_byte(addr + 1, high_byte)
    }

    pub fn tick(&mut self, cycles: u8) {
        // 1. Update 16-bit DIV counter (increments every T-cycle)
        let old_div = self.div_counter;
        self.div_counter = self.div_counter.wrapping_add(cycles as u16);

        // 2. Check if TIMA is enabled (Bit 2 of TAC)
        let timer_enabled = (self.tac & 0x04) != 0;
        if timer_enabled {
            // Determine threshold based on TAC bits 0-1
            let threshold = match self.tac & 0x03 {
                0 => 1024, // CPU Clock / 1024 (4096 Hz)
                1 => 16,   // CPU Clock / 16 (262144 Hz)
                2 => 64,   // CPU Clock / 64 (65536 Hz)
                3 => 256,  // CPU Clock / 256 (16384 Hz)
                _ => unreachable!(),
            };

            self.tima_counter += cycles as i32;
            while self.tima_counter >= threshold {
                self.tima_counter -= threshold;

                // Increment TIMA, handle overflow
                let (new_tima, overflow) = self.tima.overflowing_add(1);
                if overflow {
                    self.tima = self.tma; // Reload with TMA
                    // Request Timer Interrupt (Bit 2 of IF register, 0xFF0F)
                    self.if_register |= 0x04;
                } else {
                    self.tima = new_tima;
                }
            }
        }
    }
}

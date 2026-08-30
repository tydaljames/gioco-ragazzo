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
                    print!("{}", self.serial_data as char);

                    // Flush stdout so the text appears in your terminal immediately
                    use std::io::Write;
                    std::io::stdout().flush().unwrap();
                }
            }

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
}

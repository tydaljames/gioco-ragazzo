use std::fs;
use std::path::Path;
use crate::ppu::Ppu;

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
    pub ppu: Ppu,
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

    // MBC1 State
    pub rom_bank: usize,         // Current active ROM bank (defaults to 1)
    pub ram_bank: usize,         // Current active external RAM bank
    pub banking_mode: u8,        // 0 = ROM mode, 1 = RAM mode
    pub ram_enabled: bool,       // Is external RAM enabled?
}

impl Mmu {
    // Constructor to create new MMU and initialize arrays to 0
    pub fn new(cartridge: Cartridge) -> Self {
        Self {
            cartridge,
            ppu: Ppu::new(),
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

            // MBC1
            rom_bank: 1, // Bank 0 is fixed at the start, Bank 1 is the default switchable bank
            ram_bank: 0,
            banking_mode: 0,
            ram_enabled: false,
        }
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => {
                // ROM Bank 0 (Fixed to bank 0, unless in advanced RAM banking mode)
                let bank = if self.banking_mode == 1 {
                    (self.rom_bank & 0x60) // In mode 1, upper bits can affect bank 0
                } else {
                    0
                };
                let mapped_addr = (bank * 0x4000) + (addr as usize);
                self.cartridge.rom.get(mapped_addr).copied().unwrap_or(0xFF)
            }
            0x4000..=0x7FFF => {
                // Switchable ROM Bank
                let mapped_addr = (self.rom_bank * 0x4000) + (addr as usize - 0x4000);
                self.cartridge.rom.get(mapped_addr).copied().unwrap_or(0xFF)
            }

            // PPU memory (Graphics)
            0x8000..=0x9FFF => self.ppu.vram[addr as usize - 0x8000],
            0xFE00..=0xFE9F => self.ppu.oam[addr as usize - 0xFE00],
            0xFF40 => self.ppu.lcdc,
            0xFF41 => self.ppu.stat | 0x80, // Top bit of STAT is always 1
            0xFF42 => self.ppu.scy,
            0xFF43 => self.ppu.scx,
            0xFF44 => self.ppu.ly,
            0xFF45 => self.ppu.lyc,
            0xFF46 => self.ppu.dma,
            0xFF47 => self.ppu.bgp,
            0xFF48 => self.ppu.obp0,
            0xFF49 => self.ppu.obp1,
            0xFF4A => self.ppu.wy,
            0xFF4B => self.ppu.wx,

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
            0x0000..=0x1FFF => {
                // RAM Enable: Writing 0x0A to this range enables external RAM
                self.ram_enabled = (val & 0x0F) == 0x0A;
            }
            0x2000..=0x3FFF => {
                // ROM Bank Number (Lower 5 bits)
                let mut bank = (val & 0x1F) as usize;
                if bank == 0 {
                    bank = 1; // Bank 0 cannot be selected here; it maps to Bank 1
                }
                // Keep the upper bits intact, update the lower 5 bits
                self.rom_bank = (self.rom_bank & 0x60) | bank;
            }
            0x4000..=0x5FFF => {
                // Upper 2 bits of ROM Bank Number (or RAM Bank Number depending on mode)
                let bits = (val & 0x03) as usize;
                if self.banking_mode == 0 {
                    // ROM Mode: bits are the upper 2 bits of the ROM bank
                    self.rom_bank = (self.rom_bank & 0x1F) | (bits << 5);
                } else {
                    // RAM Mode: bits select the external RAM bank
                    self.ram_bank = bits;
                }
            }
            0x6000..=0x7FFF => {
                // Banking Mode Select (0 = ROM banking mode, 1 = RAM banking mode)
                self.banking_mode = val & 0x01;
            }

            // PPU memory (Graphics)
            0x8000..=0x9FFF => self.ppu.vram[addr as usize - 0x8000] = val,
            0xFE00..=0xFE9F => self.ppu.oam[addr as usize - 0xFE00] = val,
            0xFF40 => self.ppu.lcdc = val,
            0xFF41 => self.ppu.stat = (self.ppu.stat & 0x07) | (val & 0xF8), // Lower 3 bits are read-only
            0xFF42 => self.ppu.scy = val,
            0xFF43 => self.ppu.scx = val,
            0xFF44 => {}, // LY is read-only, ignore writes
            0xFF45 => self.ppu.lyc = val,
            0xFF46 => {
                self.ppu.dma = val;
                let source_base = (val as u16) << 8;
                for i in 0..160 {
                    let b = self.read_byte(source_base + i);
                    self.ppu.oam[i as usize] = b;
                }
            }            0xFF47 => self.ppu.bgp = val,
            0xFF48 => self.ppu.obp0 = val,
            0xFF49 => self.ppu.obp1 = val,
            0xFF4A => self.ppu.wy = val,
            0xFF4B => self.ppu.wx = val,

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
        // Tick the PPU and capture any requested interrupts
        let ppu_interrupts = self.ppu.tick(cycles);

        // If the PPU requested a VBlank (bit 0) or STAT (bit 1) interrupt, apply it to IF
        self.if_register |= ppu_interrupts;
    }
}

use std::fs;
use std::path::Path;
use crate::mmu;
use crate::ppu::Ppu;

pub struct Cartridge {
    pub rom: Vec<u8>,
    pub sram: Vec<[u8; 0x2000]>, // Up to 4 SRAM banks (8KB each)

    // MBC State
    pub mbc_type: MbcType,
    pub rom_bank: usize,
    pub sram_bank: usize,
    pub sram_enabled: bool,

    // MBC3 RTC Registers (Seconds, Minutes, Hours, Days Low, Days High)
    pub rtc_registers: [u8; 5],
    pub rtc_selected_reg: u8,
    pub rtc_latch_register: u8,
}

impl Cartridge {
    // This function will read a file from the computer
    // and return a Result. If successful, gives us a Cartridge
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let rom = fs::read(path)?;

        if rom.len() < 0x0150 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "ROM file is too small to contain a valid header",
            ));
        }

        // 1. Determine MBC Types from header byte 0x0147
        let cartridge_type = rom[0x0147];
        let mbc_type = match cartridge_type {
            0x01 | 0x02 | 0x03 => MbcType::Mbc1,
            0x0F | 0x10 | 0x11 | 0x12 | 0x13 => MbcType::Mbc3,
            _ => MbcType::RomOnly,
        };

        // 2. Determine SRAM size from header byte 0x0149
        // 0x03 is used by Pokemon Red/ Blue, with 32KB (4 banks of 8KB) SRAM
        let ram_size_byte = rom[0x0149];
        let sram_bank_count = match ram_size_byte {
            0x02 => 1, // 8KB (1 bank)
            0x03 => 4, // 32KB (4 banks)
            0x04 => 16, // 128KB (16 banks)
            _ => 0
        };

        // Initialize SRAM storage
        let sram = vec![[0; 0x2000]; sram_bank_count];

        Ok(Self {
            rom,
            sram,
            mbc_type,
            rom_bank: 1, // Bank 0 is fixed, 1 is switchable
            sram_bank: 0,
            sram_enabled: false,
            rtc_registers: [0; 5],
            rtc_selected_reg: 0xFF,
            rtc_latch_register: 0xFF,
        })
    }
}

#[derive(Debug, PartialEq)]
pub enum MbcType {
    RomOnly,
    Mbc1,
    Mbc3,
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

    pub joypad_select: u8,
    pub joypad_state: u8,
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

            joypad_select: 0x30,
            joypad_state: 0xFF, // All buttons unpressed by default
        }
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => {
                let bank = if self.banking_mode == 1 { self.rom_bank & 0x60 } else { 0 };
                let mapped_addr = (bank * 0x4000) + (addr as usize);
                self.cartridge.rom.get(mapped_addr).copied().unwrap_or(0xFF)
            }
            0x4000..=0x7FFF => {
                let active_bank = if self.cartridge.mbc_type == crate::mmu::MbcType::Mbc3 && self.rom_bank == 0 {
                    1
                } else {
                    self.rom_bank
                };
                let mapped_addr = (active_bank * 0x4000) + (addr as usize - 0x4000);
                self.cartridge.rom.get(mapped_addr).copied().unwrap_or(0xFF)
            }

            // External SRAM & RTC Registers ($A000 - $BFFF)
            0xA000..=0xBFFF => {
                if !(self.cartridge.sram_enabled || self.ram_enabled) { return 0xFF; }

                if self.cartridge.mbc_type == crate::mmu::MbcType::Mbc3
                    && (0x08..=0x0C).contains(&self.cartridge.rtc_selected_reg)
                {
                    return self.cartridge.rtc_registers[(self.cartridge.rtc_selected_reg - 0x08) as usize];
                }

                let bank = if self.cartridge.mbc_type == crate::mmu::MbcType::Mbc3 { self.cartridge.sram_bank } else { self.ram_bank };
                self.cartridge.sram.get(bank).map_or(0xFF, |s| s[(addr - 0xA000) as usize])
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

            0xFF00 => {
                // Bits 6-7 are always 1. Bits 4-5 reflect whatever the game wrote to select lines.
                let mut selected_buttons = 0x0F; // Default to all unpressed (1s)

                let select_directions = (self.joypad_select & 0x10) == 0;
                let select_actions    = (self.joypad_select & 0x20) == 0;

                // If direction keys are selected, filter by lower nibble
                if select_directions {
                    selected_buttons &= self.joypad_state & 0x0F;
                }
                // If action keys are selected, filter by upper nibble (shifted down)
                if select_actions {
                    selected_buttons &= (self.joypad_state >> 4) & 0x0F;
                }

                // Combine: Upper 2 bits (11) + Selection bits (bits 4-5) + Button state (bits 0-3)
                0xC0 | self.joypad_select | selected_buttons
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
            0x0000..=0x7FFF => {
                if self.cartridge.mbc_type == MbcType::Mbc3 {
                    match addr {
                        0x0000..=0x1FFF => self.cartridge.sram_enabled = (val & 0x0F) == 0x0A,
                        0x2000..=0x3FFF => {
                            let mut bank = (val & 0x7F) as usize;
                            if bank == 0 { bank = 1; }
                            self.rom_bank = bank;
                        }
                        0x4000..=0x5FFF => {
                            if val <= 0x03 {
                                self.cartridge.sram_bank = val as usize;
                                self.cartridge.rtc_selected_reg = 0xFF;
                            } else if (0x08..=0x0C).contains(&val) {
                                self.cartridge.rtc_selected_reg = val;
                            }
                        }
                        0x6000..=0x7FFF => self.cartridge.rtc_latch_register = val,
                        _ => {}
                    }
                } else {
                    // Original MBC1 logic preserved cleanly
                    match addr {
                        0x0000..=0x1FFF => self.ram_enabled = (val & 0x0F) == 0x0A,
                        0x2000..=0x3FFF => {
                            let mut bank = (val & 0x1F) as usize;
                            if bank == 0 { bank = 1; }
                            self.rom_bank = (self.rom_bank & 0x60) | bank;
                        }
                        0x4000..=0x5FFF => {
                            let bits = (val & 0x03) as usize;
                            if self.banking_mode == 0 {
                                self.rom_bank = (self.rom_bank & 0x1F) | (bits << 5);
                            } else {
                                self.ram_bank = bits;
                            }
                        }
                        0x6000..=0x7FFF => self.banking_mode = val & 0x01,
                        _ => {}
                    }
                }
            }

            0xA000..=0xBFFF => {
                if !(self.cartridge.sram_enabled || self.ram_enabled) { return; }

                if self.cartridge.mbc_type == crate::mmu::MbcType::Mbc3
                    && (0x08..=0x0C).contains(&self.cartridge.rtc_selected_reg)
                {
                    self.cartridge.rtc_registers[(self.cartridge.rtc_selected_reg - 0x08) as usize] = val;
                    return;
                }

                let bank = if self.cartridge.mbc_type == crate::mmu::MbcType::Mbc3 { self.cartridge.sram_bank } else { self.ram_bank };
                if bank < self.cartridge.sram.len() {
                    self.cartridge.sram[bank][(addr - 0xA000) as usize] = val;
                }
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

            0xFF00 => {
                // The game writes to bits 4 and 5 to select which button group to read
                self.joypad_select = val & 0x30;
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

    pub fn update_joypad(&mut self, state: u8) {
        self.joypad_state = state;
    }
}

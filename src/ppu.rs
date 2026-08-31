pub struct Ppu {
    pub vram: [u8; 0x2000], // 0x8000 - 0x9FFF
    pub oam: [u8; 0xA0], // 0xFE00 - 0xFE9F

    pub lcdc: u8, // 0xFF40
    pub stat: u8, // 0xFF41
    pub scy: u8, // 0xFF42
    pub scx: u8, // 0xFF43
    pub ly: u8, // 0xFF44
    pub lyc: u8, // 0xFF45
    pub dma: u8, // 0xFF46
    pub bgp: u8, // 0xFF47
    pub obp0: u8, // 0xFF48
    pub obp1: u8, // 0xFF49
    pub wy: u8,   // 0xFF4A - Window Y
    pub wx: u8,   // 0xFF4B - Window X

    // Internal timing
    dots: u32
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            vram: [0; 0x2000],
            oam: [0; 0xA0],
            lcdc: 0x91, // Default LCDC boot state
            stat: 0x85, // Default STAT boot state
            scy: 0, scx: 0, ly: 0, lyc: 0, dma: 0,
            bgp: 0xFC, obp0: 0xFF, obp1: 0xFF,
            wy: 0, wx: 0,
            dots: 0,
        }
    }
}

impl Ppu {
    // ... new() function ...

    // Returns a u8 representing requested interrupts (Bit 0 for VBlank, Bit 1 for STAT)
    pub fn tick(&mut self, cycles: u8) -> u8 {
        let mut interrupts = 0;

        // If the LCD is turned off (LCDC Bit 7 is 0), the PPU does not run
        if (self.lcdc & 0x80) == 0 {
            self.dots = 0;
            self.ly = 0;
            self.stat &= 0xFC; // Set mode to 0 (HBlank)
            return 0;
        }

        self.dots += cycles as u32;

        // A single scanline takes 456 dots
        if self.dots >= 456 {
            self.dots -= 456;
            self.ly += 1;

            if self.ly == 154 {
                self.ly = 0; // Wrap back to the top of the screen
            }

            // Check LYC=LY coincidence
            if self.ly == self.lyc {
                self.stat |= 0x04; // Set coincidence bit
                if (self.stat & 0x40) != 0 {
                    interrupts |= 0x02; // Request STAT interrupt
                }
            } else {
                self.stat &= !0x04; // Clear coincidence bit
            }
        }

        // Determine the current LCD Mode based on LY and dots
        let current_mode = self.stat & 0x03;
        let mut new_mode = current_mode;

        if self.ly >= 144 {
            new_mode = 1; // Mode 1: VBlank
        } else if self.dots < 80 {
            new_mode = 2; // Mode 2: OAM Search
        } else if self.dots < 252 {
            new_mode = 3; // Mode 3: Pixel Transfer
        } else {
            new_mode = 0; // Mode 0: HBlank
        }

        // If the mode just changed, update STAT and trigger hardware interrupts
        if current_mode != new_mode {
            self.stat = (self.stat & 0xFC) | new_mode;

            if new_mode == 1 {
                interrupts |= 0x01; // Request VBlank Interrupt

                if (self.stat & 0x10) != 0 {
                    interrupts |= 0x02; // STAT interrupt for VBlank
                }
            } else if new_mode == 2 && (self.stat & 0x20) != 0 {
                interrupts |= 0x02; // STAT interrupt for OAM Search
            } else if new_mode == 0 && (self.stat & 0x08) != 0 {
                interrupts |= 0x02; // STAT interrupt for HBlank
            }
        }

        interrupts
    }
}
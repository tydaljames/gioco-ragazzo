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
    dots: u32,

    pub framebuffer: [u32; 160 * 144],
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

            framebuffer: [0; 160 * 144],
        }
    }
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

            // ADD THIS: Render the scanline right as Pixel Transfer (Mode 3) begins
            if new_mode == 3 && self.ly < 144 {
                self.render_scanline();
            }

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
    pub fn render_scanline(&mut self) {
        let y = self.ly as usize;
        if y >= 144 { return; }

        // Determine Tile Map base address (LCDC Bit 3)
        let tile_map_base = if (self.lcdc & 0x08) != 0 { 0x1C00 } else { 0x1800 };

        // Determine Tile Data addressing mode (LCDC Bit 4) using a local boolean
        let unsigned_tiles = (self.lcdc & 0x10) != 0;

        for x in 0..160_usize {
            let map_x = (x.wrapping_add(self.scx as usize)) & 255;
            let map_y = (y.wrapping_add(self.scy as usize)) & 255;

            // Find tile coordinates (0-31)
            let tile_x = map_x / 8;
            let tile_y = map_y / 8;
            let tile_offset = tile_map_base + tile_y * 32 + tile_x;
            let tile_index = self.vram[tile_offset];

            // Find tile data address in VRAM based on the addressing mode
            let tile_data_addr = if unsigned_tiles {
                // Unsigned: 0x8000 base (vram index 0x0000)
                (tile_index as usize) * 16
            } else {
                // Signed: 0x8800 base (vram index 0x0800, index is treated as i8)
                let signed_index = tile_index as i8 as i32;
                (0x0800 + signed_index * 16) as usize
            };

            // Find row within the tile (0-7)
            let row = map_y % 8;
            let byte1 = self.vram[tile_data_addr + row * 2];
            let byte2 = self.vram[tile_data_addr + row * 2 + 1];

            // Extract bit for this pixel (pixels are ordered bit 7 down to bit 0)
            let bit = 7 - (map_x % 8);
            let color_bit1 = (byte1 >> bit) & 1;
            let color_bit2 = (byte2 >> bit) & 1;
            let color_id = (color_bit2 << 1) | color_bit1;

            // Map color ID (0-3) using BGP palette register
            let palette_color = (self.bgp >> (color_id * 2)) & 0x03;

            // Convert Game Boy shade to an RGB color
            let rgb = match palette_color {
                0 => 0xFF9BBC0F, // Lightest green
                1 => 0xFF8BAC0F,
                2 => 0xFF306230,
                _ => 0xFF0F380F, // Darkest green
            };

            // Store in framebuffer
            self.framebuffer[y * 160 + x] = rgb;
        }
    }
}

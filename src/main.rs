use minifb::{Key, Window, WindowOptions};
use crate::cpu::Cpu;
use crate::mmu::{Cartridge, Mmu};

mod mmu;
mod cpu;
mod ppu;

fn main() {
    // println!("Game Boy emulator booting up!");
    println!("Loading ROM...");

    // 1. Load the ROM file
    // Test ROMs
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/cgb_sound/cgb_sound.gb").expect("Failed to load ROM!");
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/cpu_instrs/cpu_instrs.gb").expect("Failed to load ROM!");
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/dmg_sound/dmg_sound.gb").expect("Failed to load ROM!");
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/instr_timing/instr_timing.gb").expect("Failed to load ROM!");
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/interrupt_time/interrupt_time.gb").expect("Failed to load ROM!");
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/mem_timing/mem_timing.gb").expect("Failed to load ROM!");
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/mem_timing-2/mem_timing.gb").expect("Failed to load ROM!");
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/oam_bug/oam_bug.gb").expect("Failed to load ROM!");

    // Real ROMs
    // let cartridge = Cartridge::load("roms/Pokemon Red.gb").expect("Failed to load ROM!");
    let cartridge = Cartridge::load("roms/Tetris (JUE) (V1.1) [!].gb").expect("Failed to load ROM!");

    // 2. Initialize the hardware
    let mmu = Mmu::new(cartridge);
    let mut cpu = Cpu::new(mmu);



    // 3. Initialize the Display Window (Scaled 4x for a comfortable viewing size)
    let mut window = Window::new(
        "Rust Game Boy Emulator - Tetris",
        160,
        144,
        WindowOptions {
            scale: minifb::Scale::X4,
            ..WindowOptions::default()
        },
    )
        .unwrap();

    // Limit to ~60 FPS (approx 16.6ms per frame)
    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    // 4. The Visual Execution Loop
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let mut cycles_this_frame = 0;

        // Run the CPU/PPU until a full frame (~70,224 T-cycles) has passed
        while cycles_this_frame < 70224 {
            let cycles = cpu.step(); // Assumes cpu.step() returns the number of cycles taken (e.g., u8)

            // NOTE: If your cpu.step() doesn't automatically call mmu.tick(cycles)
            // inside its execution, make sure to add it here:
            // cpu.mmu.tick(cycles);

            cycles_this_frame += cycles as u32;
        }

        // 5. Push the PPU framebuffer to the window screen buffer
        window
            .update_with_buffer(
                &cpu.mmu.ppu.framebuffer,
                160,
                144,
            )
            .unwrap();
    }




    // // 3. The Execution Loop
    // loop {
    //     cpu.step();
    // }
}

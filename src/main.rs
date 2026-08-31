use crate::cpu::Cpu;
use crate::mmu::{Cartridge, Mmu};

mod mmu;
mod cpu;
mod ppu;

fn main() {
    // println!("Game Boy emulator booting up!");
    println!("Loading Blargg's Test ROM...");

    // 1. Load the ROM file
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/cgb_sound/cgb_sound.gb").expect("Failed to load ROM!");
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/cpu_instrs/cpu_instrs.gb").expect("Failed to load ROM!");
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/dmg_sound/dmg_sound.gb").expect("Failed to load ROM!");
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/instr_timing/instr_timing.gb").expect("Failed to load ROM!");
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/interrupt_time/interrupt_time.gb").expect("Failed to load ROM!");
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/mem_timing/mem_timing.gb").expect("Failed to load ROM!");
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/mem_timing-2/mem_timing.gb").expect("Failed to load ROM!");
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/oam_bug/oam_bug.gb").expect("Failed to load ROM!");






    // 2. Initialize the hardware
    let mmu = Mmu::new(cartridge);
    let mut cpu = Cpu::new(mmu);

    // 3. The Execution Loop
    loop {
        cpu.step();
    }
}

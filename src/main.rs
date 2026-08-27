use crate::cpu::Cpu;
use crate::mmu::{Cartridge, Mmu};

mod mmu;
mod cpu;

fn main() {
    // println!("Game Boy emulator booting up!");
    println!("Loading Blargg's Test ROM...");

    // 1. Load the ROM file
    let cartridge = Cartridge::load("roms/gb-test-roms-master/cpu_instrs/cpu_instrs.gb").expect("Failed to load ROM!");

    // 2. Initialize the hardware
    let mmu = Mmu::new(cartridge);
    let mut cpu = Cpu::new(mmu);

    // 3. The Execution Loop
    loop {
        cpu.step();
    }
}

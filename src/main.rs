use crate::cpu::Cpu;
use crate::mmu::{Cartridge, Mmu};

mod mmu;
mod cpu;

fn main() {
    // println!("Game Boy emulator booting up!");
    println!("Loading Blargg's Test ROM...");

    // 1. Load the ROM file
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/cpu_instrs/individual/01-special.gb").expect("Failed to load ROM!");
    let cartridge = Cartridge::load("roms/gb-test-roms-master/cpu_instrs/individual/02-interrupts.gb").expect("Failed to load ROM!");
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/cpu_instrs/individual/03-op sp,hl.gb").expect("Failed to load ROM!");
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/cpu_instrs/individual/04-op r,imm.gb").expect("Failed to load ROM!");
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/cpu_instrs/individual/05-op rp.gb").expect("Failed to load ROM!");
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/cpu_instrs/individual/06-ld r,r.gb").expect("Failed to load ROM!");
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/cpu_instrs/individual/07-jr,jp,call,ret,rst.gb").expect("Failed to load ROM!");
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/cpu_instrs/individual/08-misc instrs.gb").expect("Failed to load ROM!");
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/cpu_instrs/individual/09-op r,r.gb").expect("Failed to load ROM!");
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/cpu_instrs/individual/10-bit ops.gb").expect("Failed to load ROM!");
    // let cartridge = Cartridge::load("roms/gb-test-roms-master/cpu_instrs/individual/11-op a,(hl).gb").expect("Failed to load ROM!");

    // 2. Initialize the hardware
    let mmu = Mmu::new(cartridge);
    let mut cpu = Cpu::new(mmu);

    // 3. The Execution Loop
    loop {
        cpu.step();
    }
}

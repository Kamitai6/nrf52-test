#![no_std]
#![no_main]

use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

use core::{cell::RefCell, sync::atomic::{AtomicU32, Ordering}};

use cortex_m_rt::entry;
use cortex_m::{asm, interrupt::{Mutex}};

use h7lib;

#[entry]
fn main() -> ! {
    rtt_init_print!();
    // let mut cp = cortex_m::Peripherals::take().unwrap();
    // let dp = pac::Peripherals::take().unwrap();

    loop {
    }
}

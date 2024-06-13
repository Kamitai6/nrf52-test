#![no_std]
#![no_main]

use cortex_m::{self, delay};
use cortex_m_rt::entry;
use embedded_alloc::Heap;
use panic_halt as _;
use stm32h5::stm32h562;
mod as5048;
mod stm32;

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[entry]
fn main() -> ! {
    use core::mem::MaybeUninit;
    const HEAP_SIZE: usize = 1024;
    static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
    unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }

    let dp = stm32h562::Peripherals::take().unwrap();
    let cp = cortex_m::peripheral::Peripherals::take().unwrap();
    let mut delay = delay::Delay::new(cp.SYST, 240000000_u32);

    stm32::rcc::clock_init(&dp);
    stm32::gpio::gpio_a12_init(&dp);
    stm32::gpio::gpio_c10_init(&dp);
    stm32::gpio::gpio_c11_init(&dp);
    stm32::gpio::gpio_c12_init(&dp);
    // let spi = stm32::spi::SPI::new(&dp);
    // let delay_func = move |ms: u32| delay.delay_ms(ms);
    // let mut encoder = as5048::AS5048::new(&spi, delay_func);
    // spi.spi3_init();

    stm32::gpio::gpio_a12_toggle(&dp, true);
    stm32::gpio::gpio_c10_toggle(&dp, true);
    stm32::gpio::gpio_c11_toggle(&dp, true);
    stm32::gpio::gpio_c12_toggle(&dp, true);

    loop {
        delay.delay_ms(100)
        // let mut value = encoder.getState();
    }
}

#![no_std]
#![no_main]

// pa12 led blinky

use cortex_m_rt::entry;
use panic_halt as _;
use stm32h5::stm32h562;

#[entry]
fn main() -> ! {
    let peripherals = stm32h562::Peripherals::take().unwrap();
    peripherals
        .RCC
        .ahb2enr()
        .modify(|_, w| w.gpioaen().enabled());

    let gpioa = &peripherals.GPIOA;
    gpioa.moder().modify(|_, w| w.mode12().output());
    gpioa.odr().modify(|_, w| w.od12().high());
    gpioa.odr().modify(|_, w| w.od12().low());

    loop {}
}

#![no_std]
#![no_main]

use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

use cortex_m::asm;
use cortex_m_rt::entry;
use stm32h7xx_hal::{pac, prelude::*};


#[entry]
fn main() -> ! {
    rtt_init_print!();
    let dp = pac::Peripherals::take().expect("Cannot take peripherals");

    // Constrain and Freeze power
    rprintln!("Setup PWR...                  ");
    let pwr = dp.PWR.constrain();
    let pwrcfg = pwr.freeze();

    // Constrain and Freeze clock
    rprintln!("Setup RCC...                  ");
    let rcc = dp.RCC.constrain();
    let ccdr = rcc.sys_ck(8.MHz()).freeze(pwrcfg, &dp.SYSCFG);

    // Acquire the GPIOA and GPIOB peripherals. This also enables the clocks for
    // these peripherals in the RCC register.
    let gpioa = dp.GPIOA.split(ccdr.peripheral.GPIOA);
    let gpiob = dp.GPIOB.split(ccdr.peripheral.GPIOB);
    let gpioe = dp.GPIOE.split(ccdr.peripheral.GPIOE);

    // Select PWM output pins
    // let pins = (
    //     gpioa.pa8.into_alternate(),
    //     gpioa.pa9.into_alternate(),
    //     gpioa.pa10.into_alternate(),
    // );

    rprintln!("");
    rprintln!("stm32h7xx-hal example - PWM");
    rprintln!("");

    // // Configure PWM at 10kHz
    // let (mut pwm, ..) =
    //     dp.TIM1
    //         .pwm(pins, 10.kHz(), ccdr.peripheral.TIM1, &ccdr.clocks);

    // // Output PWM on PA8
    // let max = pwm.get_max_duty();
    // pwm.set_duty(max / 2);

    // rprintln!("50%");
    // pwm.enable();
    // asm::bkpt();

    // rprintln!("25%");
    // pwm.set_duty(max / 4);
    // asm::bkpt();

    // rprintln!("12.5%");
    // pwm.set_duty(max / 8);
    // asm::bkpt();

    // rprintln!("100%");
    // pwm.set_duty(max);
    // asm::bkpt();

    let mut pwm = dp.TIM1.pwm(
        gpioe.pe9.into_alternate(),
        10.kHz(),
        ccdr.peripheral.TIM1,
        &ccdr.clocks,
    );

    // Output PWM on PB14
    let max = pwm.get_max_duty();
    pwm.set_duty(max / 1);
    pwm.enable();

    loop {
        cortex_m::asm::nop()
    }
}
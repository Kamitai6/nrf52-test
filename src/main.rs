#![no_std]
#![no_main]

use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

use cortex_m_rt::entry;
use stm32h7xx_hal::{pac, prelude::*, spi};

pub struct Drv8343Reg {
    pub fault_status: u8,
    pub diag_status: [u8; 3],
    pub control: [u8; 14],
}

#[entry]
fn main() -> ! {
    rtt_init_print!();
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    // Constrain and Freeze power
    rprintln!("Setup PWR...                  ");
    let pwr = dp.PWR.constrain();
    let pwrcfg = pwr.freeze();

    // Constrain and Freeze clock
    rprintln!("Setup RCC...                  ");
    let rcc = dp.RCC.constrain();
    let ccdr = rcc
        .sys_ck(100.MHz())
        .pll1_q_ck(100.MHz())
        .freeze(pwrcfg, &dp.SYSCFG);

    let gpioa = dp.GPIOA.split(ccdr.peripheral.GPIOA);
    let gpiob = dp.GPIOB.split(ccdr.peripheral.GPIOB);
    let gpioc = dp.GPIOC.split(ccdr.peripheral.GPIOC);
    let gpiod = dp.GPIOD.split(ccdr.peripheral.GPIOD);

    let sck = gpioc.pc10.into_alternate();
    let miso = gpioc.pc11.into_alternate();
    let mosi = gpioc.pc12.into_alternate();
    let mut nss = gpioa.pa15.into_push_pull_output();

    let mut en_gate = gpioa.pa4.into_push_pull_output();
    en_gate.set_high();

    let mut led = gpiod.pd0.into_push_pull_output();
    let mut delay = cp.SYST.delay(ccdr.clocks);

    let mut spi: spi::Spi<_, _, u16> = dp.SPI3.spi(
        (sck, miso, mosi),
        spi::MODE_2, // or MODE_1?
        500.kHz(),
        ccdr.peripheral.SPI3,
        &ccdr.clocks,
    );
    nss.set_high();
    led.set_high();

    delay.delay_us(100_u16);

    let mut spi_buffer: [u16; 1] = [(0b1 << 15) | (0b0000101 << 8) | 0b00000000]; //fault statusを読みたい

    nss.set_low();
    led.set_low();
    let result = spi.transfer(&mut spi_buffer);
    match result {
        Ok(values) => {
            for (i, &value) in values.iter().enumerate() {
                rprintln!("Received data {}: {:#018b}", i, value);
            }
        }
        Err(e) => rprintln!("Error: {:?}", e),
    }
    nss.set_high();
    led.set_high();

    loop {
        
        // delay.delay_us(800_u16);

        // spi_buffer = [(0b1 << 15) | (0b0000000 << 8) | 0b00000000];
        // nss.set_low();
        // led.set_low();
        // let result2 = spi.transfer(&mut spi_buffer);
        // match result2 {
        //     Ok(values) => {
        //         for (i, &value) in values.iter().enumerate() {
        //             rprintln!("Received data {}: {}", i, value);
        //         }
        //     }
        //     Err(e) => rprintln!("Error: {:?}", e),
        // }
        // nss.set_high();
        // led.set_high();
    }
}
#![no_std]
#![no_main]

use cortex_m_rt::entry;
use nb::block;
use panic_halt as _;
use rtt_target::{rprintln, rtt_init_print};
use stm32h7xx_hal::{pac, prelude::*, spi};

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

    let sck = gpioa.pa12.into_alternate();
    let miso = gpiob.pb14.into_alternate();
    let mosi = gpiob.pb15.into_alternate();
    let mut nss = gpioa.pa11.into_push_pull_output();

    let mut led = gpiod.pd0.into_push_pull_output();
    let mut delay = cp.SYST.delay(ccdr.clocks);

    let mut spi: spi::Spi<_, _, u8> = dp.SPI2.spi(
        (sck, miso, mosi),
        spi::MODE_1,
        2.MHz(),
        ccdr.peripheral.SPI2,
        &ccdr.clocks,
    );
    nss.set_high();
    led.set_high();

    delay.delay_us(100_u16);

    let mut spi_buffer = [0x00, 0x00, 0xAA, 0xAA, 0x00, 0x00, 0xD0, 0xAB];

    nss.set_low();
    led.set_low();
    let result = spi.transfer(&mut spi_buffer);
    match result {
        Ok(values) => {
            for (i, &value) in values.iter().enumerate() {
                rprintln!("Received data {}: {}", i, value);
            }
        }
        Err(e) => rprintln!("Error: {:?}", e),
    }
    nss.set_high();
    led.set_high();

    loop {
        
        delay.delay_us(800_u16);

        spi_buffer = [0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x13, 0xEA];
        nss.set_low();
        led.set_low();
        let result2 = spi.transfer(&mut spi_buffer);
        match result2 {
            Ok(values) => {
                for (i, &value) in values.iter().enumerate() {
                    rprintln!("Received data {}: {}", i, value);
                }
                let angle_lsb = ((values[1] & 0x3F) as u16) << 8 | (values[0] as u16);
                rprintln!("angle {}", angle_lsb);
                let error_lsb = (values[1] as u16) >> 6;
                rprintln!("error {}", error_lsb);
                let crc_lsb = (values[7] as u16);
                rprintln!("crc {}", crc_lsb);
                let vgain_lsb = (values[4] as u16);
                rprintln!("vgain {}", vgain_lsb);
                let rollcnt_lsb = (values[6] as u16) & 0x3F;
                rprintln!("rollcnt {}", rollcnt_lsb);
            }
            Err(e) => rprintln!("Error: {:?}", e),
        }
        nss.set_high();
        led.set_high();
    }
}

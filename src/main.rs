#![no_std]
#![no_main]

use cortex_m::{self, delay};
use cortex_m_rt::entry;
use embassy_stm32 as pac;

use embassy_stm32::mode::Async;
use embassy_stm32::spi;
use embassy_stm32::time::mhz;
use embassy_time::Timer;

use embassy_executor::Executor;

use core::fmt::Write;
use core::str::from_utf8;
use heapless::String;
use static_cell::StaticCell;
// use embassy_time::Timer;
use panic_halt as _;
// use stm32h5::stm32h562 as pac;
// use stm32h5::stm32h562::interrupt;
// mod as5048;
// mod stm32;

#[embassy_executor::task]
async fn main_task(mut spi: spi::Spi<'static, Async>) {
    for n in 0u32.. {
        let mut write: String<128> = String::new();
        let mut read = [0; 128];
        core::write!(&mut write, "Hello DMA World {}!\r\n", n).unwrap();
        // transfer will slice the &mut read down to &write's actual length.
        spi.transfer(&mut read, write.as_bytes()).await.ok();
        // info!("read via spi+dma: {}", from_utf8(&read).unwrap());
    }
}

static EXECUTOR: StaticCell<Executor> = StaticCell::new();

#[entry]
fn main() -> ! {
    // info!("Hello World!");
    let mut config = pac::Config::default();
    {
        use pac::rcc::*;
        config.rcc.ls = LsConfig::off();
        config.rcc.hsi = Some(HSIPrescaler::DIV1);
        config.rcc.csi = false;
        config.rcc.pll1 = Some(Pll {
            source: PllSource::HSI,
            prediv: PllPreDiv::DIV4, // pllm
            mul: PllMul::MUL30,      //plln
            divp: Some(PllDiv::DIV2),
            divq: Some(PllDiv::DIV2),
            divr: Some(PllDiv::DIV2),
        });
        config.rcc.sys = Sysclk::PLL1_P;
        config.rcc.ahb_pre = AHBPrescaler::DIV1;
        config.rcc.apb1_pre = APBPrescaler::DIV1;
        config.rcc.apb2_pre = APBPrescaler::DIV1;
        config.rcc.apb3_pre = APBPrescaler::DIV1;
        config.rcc.voltage_scale = VoltageScale::Scale0;
    }
    let p = pac::init(config);

    let mut spi_config = spi::Config::default();
    spi_config.frequency = mhz(1);

    let spi = spi::Spi::new(
        p.SPI3,
        p.PB3,
        p.PB5,
        p.PB4,
        p.GPDMA1_CH3,
        p.GPDMA1_CH4,
        spi_config,
    );

    let executor = EXECUTOR.init(Executor::new());

    executor.run(|spawner| {
        spawner.spawn(main_task(spi));
    })
}
// #[embassy_executor::main]
// async fn main(_spawner: Spawner) {
//     let mut config = pac::Config::default();
//     {
//         use pac::rcc::*;
//         config.rcc.ls = LsConfig::off();
//         config.rcc.hsi = Some(HSIPrescaler::DIV1);
//         config.rcc.csi = false;
//         config.rcc.pll1 = Some(Pll {
//             source: PllSource::HSI,
//             prediv: PllPreDiv::DIV4, // pllm
//             mul: PllMul::MUL30,      //plln
//             divp: Some(PllDiv::DIV2),
//             divq: Some(PllDiv::DIV2),
//             divr: Some(PllDiv::DIV2),
//         });
//         config.rcc.sys = Sysclk::PLL1_P;
//         config.rcc.ahb_pre = AHBPrescaler::DIV1;
//         config.rcc.apb1_pre = APBPrescaler::DIV1;
//         config.rcc.apb2_pre = APBPrescaler::DIV1;
//         config.rcc.apb3_pre = APBPrescaler::DIV1;
//         config.rcc.voltage_scale = VoltageScale::Scale0;
//     }
//     let p = pac::init(config);

//     // let mut led = pac::gpio::Output::new(p.PA12, pac::gpio::Level::High, pac::gpio::Speed::Low);

//     let mut spi_config = spi::Config::default();
//     //     spi_config.frequency = mhz(1);
//     //     let spi = spi::Spi::new(
//     //         p.SPI3,
//     //         p.PB3,
//     //         p.PB5,
//     //         p.PB4,
//     //         p.GPDMA1_CH3,
//     //         p.GPDMA1_CH4,
//     //         spi_config,
//     //     );

//     loop {
//         led.set_high();
//         Timer::after_millis(500).await;

//         led.set_low();
//         Timer::after_millis(500).await;
//     }
// }

// #[entry]
// fn main() -> ! {
//     let mut config = pac::Config::default();
//     {
//         use pac::rcc::*;
//         config.rcc.ls = LsConfig::off();
//         config.rcc.hsi = Some(HSIPrescaler::DIV1);
//         config.rcc.csi = false;
//         config.rcc.pll1 = Some(Pll {
//             source: PllSource::HSI,
//             prediv: PllPreDiv::DIV4, // pllm
//             mul: PllMul::MUL30,      //plln
//             divp: Some(PllDiv::DIV2),
//             divq: Some(PllDiv::DIV2),
//             divr: Some(PllDiv::DIV2),
//         });
//         config.rcc.sys = Sysclk::PLL1_P;
//         config.rcc.ahb_pre = AHBPrescaler::DIV1;
//         config.rcc.apb1_pre = APBPrescaler::DIV1;
//         config.rcc.apb2_pre = APBPrescaler::DIV1;
//         config.rcc.apb3_pre = APBPrescaler::DIV1;
//         config.rcc.voltage_scale = VoltageScale::Scale0;
//     }
//     let p = pac::init(config);
//     // let mut led = pac::gpio::Output::new(p.PA12, pac::gpio::Level::High, pac::gpio::Speed::Low);
//     let mut spi_config = spi::Config::default();
//     spi_config.frequency = mhz(1);
//     let spi = spi::Spi::new(
//         p.SPI3,
//         p.PB3,
//         p.PB5,
//         p.PB4,
//         p.GPDMA1_CH3,
//         p.GPDMA1_CH4,
//         spi_config,
//     );

//     // let dp = pac::Peripherals::take().unwrap();
//     // let mut cp = cortex_m::peripheral::Peripherals::take().unwrap();
//     // let mut delay = delay::Delay::new(cp.SYST, 240000000_u32);

//     // let spi = stm32::spi::SPI::new(&dp);
//     // let delay_func = move |ms: u32| delay.delay_ms(ms);
//     // let mut encoder = as5048::AS5048::new(&spi, delay_func);
//     // spi.spi3_init();

//     loop {
//         // for n in 0u32.. {
//         let mut write: String<128> = String::new();
//         let mut read = [0; 128];
//         // core::write!(&mut write, "Hello DMA World {}!\r\n", n).unwrap();
//         // transfer will slice the &mut read down to &write's actual length.
//         spi.transfer(&mut read, write.as_bytes());
//         // info!("read via spi+dma: {}", from_utf8(&read).unwrap());
//         // }

//         // led.set_high();
//         // Timer::after_millis(500);

//         // led.set_low();
//         // Timer::after_millis(500);
//     }
// }

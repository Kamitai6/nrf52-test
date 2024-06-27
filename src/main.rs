#![no_std]
#![no_main]

use cortex_m::{self, delay};
use cortex_m_rt::entry;
use embassy_stm32 as pac;
// use embassy_time::Timer;
use panic_halt as _;
// use stm32h5::stm32h562 as pac;
// use stm32h5::stm32h562::interrupt;
// mod as5048;
// mod stm32;

#[entry]
fn main() -> ! {
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
    let mut led = pac::gpio::Output::new(p.PA12, pac::gpio::Level::High, pac::gpio::Speed::Low);
    // let dp = pac::Peripherals::take().unwrap();
    let mut cp = cortex_m::peripheral::Peripherals::take().unwrap();
    let mut delay = delay::Delay::new(cp.SYST, 240000000_u32);
    // unsafe {
    //     pac::NVIC::set_priority(&mut cp.NVIC, pac::interrupt::TIM2, 1); // (1)TIM2割り込み優先度設定
    //     pac::NVIC::unmask(pac::interrupt::TIM2); // (2)TIM2 NVIC割り込み許可
    // }
    // stm32::rcc::clock_init(&dp);
    // let spi = stm32::spi::SPI::new(&dp);
    // let delay_func = move |ms: u32| delay.delay_ms(ms);
    // let mut encoder = as5048::AS5048::new(&spi, delay_func);
    // spi.spi3_init();

    loop {
        led.set_high();
        delay.delay_ms(1000);

        led.set_low();
        delay.delay_ms(1000);
    }
}

// #[interrupt] // (21)割り込みの指定
// fn TIM2() {
//     // (22)TIM2割り込みハンドラ
//     unsafe {
//         let dp = stm32f401::Peripherals::steal(); // (23)Peripheralsの取得
//         dp.TIM2.sr.modify(|_, w| w.uif().clear()); // (24)更新フラグのクリア
//         dp.GPIOA.odr.modify(|r, w| w.odr5().bit(r.odr5().is_low())); // (25)GPIOA5の出力を反転する
//     }
// }

#![no_std]
#![no_main]

use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};
use cortex_m_rt::entry;

use h7lib::periph::{pwr, rcc, gpio, adc};

#[entry]
fn main() -> ! {
    rtt_init_print!();

    // let mut adc3 = adc::Adc::adc3(
    //         dp.ADC3,
    //         4.MHz(),
    //         &mut delay,
    //         ccdr.peripheral.ADC3,
    //         &ccdr.clocks,
    //     )
    //     .enable();
    //     adc3.set_resolution(adc::Resolution::SixteenBit); //16bit
    
    // let mut channel = gpioa.pa6.into_analog();
    // let mut channel = gpioc.pc2.into_analog();

    // let sck = gpioc.pc10.into_alternate();
    // let miso = gpioc.pc11.into_alternate();
    // let mosi = gpioc.pc12.into_alternate();
    // let mut nss = gpioa.pa15.into_push_pull_output();

    // let mut en_gate = gpioa.pa4.into_push_pull_output(); //default low
    // let mut led = gpiod.pd0.into_push_pull_output();

    // let (mut t1control, (t1c1, t1c2, t1c3)) = dp.TIM1.pwm_advanced(
    //     (
    //         gpioe.pe9.into_alternate(),
    //         gpioe.pe11.into_alternate(),
    //         gpioe.pe13.into_alternate(),
    //     ),
    //     ccdr.peripheral.TIM1,
    //     &ccdr.clocks,
    // )
    let pwr_config = pwr::PwrConfig {
        ..Default::default()
    };
    let pwr = pwr::Power::init(pwr_config);
    let rcc_config = rcc::Config {
        ..Default::default()
    };
    let clock = rcc::Rcc::init(pwr, rcc_config);

    let pb1 = gpio::PB::<1>::init(gpio::PinMode::Analog, &clock);
    let adc1_cfg = adc::Config {
        ..Default::default()
    };
    let mut adc1 = adc::Adc::<1>::init(pb1, adc1_cfg);

    loop {
        rprintln!("adc1: {}", adc1.read(adc::Channel::C5));
    }
}

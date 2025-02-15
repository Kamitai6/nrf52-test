#![no_std]
#![no_main]

use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

use cortex_m_rt::entry;
use stm32h7xx_hal::{adc, delay::Delay, pac, prelude::*, rcc::rec::AdcClkSel};

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

    // We need to configure a clock for adc_ker_ck_input. The default
    // adc_ker_ck_input is pll2_p_ck, but we will use per_ck. per_ck is sourced
    // from the 64MHz HSI
    //
    // adc_ker_ck_input is then divided by the ADC prescaler to give f_adc. The
    // maximum f_adc is 50MHz
    let mut ccdr = rcc.sys_ck(100.MHz()).freeze(pwrcfg, &dp.SYSCFG);

    // Switch adc_ker_ck_input multiplexer to per_ck
    ccdr.peripheral.kernel_adc_clk_mux(AdcClkSel::Per);

    rprintln!("");
    rprintln!("stm32h7xx-hal example - ADC");
    rprintln!("");

    let mut delay = Delay::new(cp.SYST, ccdr.clocks);

    // Setup ADC
    let mut adc1 = adc::Adc::adc1(
        dp.ADC1,
        4.MHz(),
        &mut delay,
        ccdr.peripheral.ADC12,
        &ccdr.clocks,
    )
    .enable();
    adc1.set_resolution(adc::Resolution::SixteenBit);

    // We can't use ADC2 here because ccdr.peripheral.ADC12 has been
    // consumed. See examples/adc12.rs

    // Setup GPIOC
    let gpiob = dp.GPIOB.split(ccdr.peripheral.GPIOB);

    // Configure pc0 as an analog input
    let mut channel = gpiob.pb1.into_analog(); // ANALOG IN 10

    loop {
        let data: u32 = adc1.read(&mut channel).unwrap();
        // voltage = reading * (vref/resolution)
        rprintln!(
            "ADC reading: {}, voltage for nucleo: {}",
            data,
            data as f32 * (3.3 / adc1.slope() as f32)
        );
    }
}
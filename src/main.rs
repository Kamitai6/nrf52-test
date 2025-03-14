#![no_std]
#![no_main]

use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};
use cortex_m_rt::entry;

use h7lib::*;
use periph::{pwr, rcc, gpio, adc, spi};


fn transfer_spi<const NSS_PORT: char, const NSS_PIN: u8, 
    const SPI_N: u8
>(
    rw: bool, nss: &mut gpio::Gpio<NSS_PORT, NSS_PIN>, 
    spi: &mut spi::Spi<SPI_N>, spi_buffer: &mut [u8])
{
    nss.set_low();
    if rw {
        let result = spi.transfer(spi_buffer);
        match result {
            Ok(values) => {
                for (i, &value) in values.iter().enumerate() {
                    rprintln!("Received data {}: {:#010b}", i, value);
                    // rprintln!("Received data {}: {:#018b}", i, value);
                }
            }
            Err(e) => rprintln!("Error {:?}", e),
        }
    }
    else {
        let _ = spi.write(spi_buffer);
    }
    
    nss.set_high();
}

#[entry]
fn main() -> ! {
    rtt_init_print!();

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
        sys_ck: Some(100.MHz()),
        pll1: rcc::PllConfig {
            q_ck: Some(100.MHz()),
            ..Default::default()
        },
        ..Default::default()
    };
    let clock = rcc::Rcc::init(pwr, rcc_config);

    let mut led = gpio::PD::<0>::init(gpio::PinMode::Output(gpio::OutputType::PushPull), &clock);
    led.set_high();

    let sck = gpio::PC::<10>::init(gpio::PinMode::AltFn(6, gpio::OutputType::PushPull), &clock);
    let miso = gpio::PC::<11>::init(gpio::PinMode::AltFn(6, gpio::OutputType::PushPull), &clock);
    let mosi = gpio::PC::<12>::init(gpio::PinMode::AltFn(6, gpio::OutputType::PushPull), &clock);
    let mut nss = gpio::PA::<15>::init(gpio::PinMode::Output(gpio::OutputType::PushPull), &clock);
    nss.set_high();

    let mut en_gate = gpio::PA::<4>::init(gpio::PinMode::Output(gpio::OutputType::PushPull), &clock);
    en_gate.set_high();
    delay_us(&clock, 10);

    let spi3_config = spi::SpiConfig {
        mode: spi::SpiMode::mode1(),
        data_size: spi::DataSize::D8,
        ..Default::default()
    };

    let mut spi3 = spi::Spi::<3>::init(sck, miso, mosi, spi3_config);

    // let sck = gpio::PA::<12>::init(gpio::PinMode::AltFn(5, gpio::OutputType::PushPull), &clock);
    // let miso = gpio::PB::<14>::init(gpio::PinMode::AltFn(5, gpio::OutputType::PushPull), &clock);
    // let mosi = gpio::PB::<15>::init(gpio::PinMode::AltFn(5, gpio::OutputType::PushPull), &clock);
    // let mut nss = gpio::PA::<11>::init(gpio::PinMode::Output(gpio::OutputType::PushPull), &clock);
    // nss.set_high();

    // let spi2_config = spi::SpiConfig {
    //     mode: spi::SpiMode::mode1(),
    //     data_size: spi::DataSize::D8,
    //     ..Default::default()
    // };

    // let mut spi2 = spi::Spi::<2>::init(sck, miso, mosi, spi2_config);

    let mut spi_buffer: [u8; 2] = [0; 2];

    spi_buffer = [0b10000000, 0b00000000]; //read fault
    transfer_spi(true, &mut nss, &mut spi3, &mut spi_buffer);
    delay_us(&clock, 10);

    let rccregs = unsafe {&*pac::RCC::ptr()};
    rprintln!("apb1henr {:#010x}", rccregs.apb1henr.read().bits());
    rprintln!("apb1lenr {:#010x}", rccregs.apb1lenr.read().bits());
    rprintln!("apb2enr {:#010x}", rccregs.apb2enr.read().bits());
    let spiregs = unsafe {&*pac::SPI3::ptr()};
    rprintln!("cr1: {:#010x}", spiregs.cr1.read().bits());
    rprintln!("cr2: {:#010x}", spiregs.cr2.read().bits());
    rprintln!("cfg1: {:#010x}", spiregs.cfg1.read().bits());
    rprintln!("cfg2: {:#010x}", spiregs.cfg2.read().bits());
    rprintln!("sr: {:#010x}", spiregs.sr.read().bits());

    spi_buffer = [0b00001101, 0b01100000]; // write lock free
    transfer_spi(false, &mut nss, &mut spi3, &mut spi_buffer);
    delay_us(&clock, 10);
    spi_buffer = [0b00000100, 0b10000000]; // write clear fault
    transfer_spi(false, &mut nss, &mut spi3, &mut spi_buffer);
    delay_us(&clock, 10);
    spi_buffer = [0b00001110, 0b00000010]; // write ocp mode
    transfer_spi(false, &mut nss, &mut spi3, &mut spi_buffer);
    delay_us(&clock, 10);
    spi_buffer = [0b10001110, 0b00000000]; //read ic11
    transfer_spi(true, &mut nss, &mut spi3, &mut spi_buffer);
    delay_us(&clock, 10);
    spi_buffer = [0b10000000, 0b00000000]; //read fault
    transfer_spi(true, &mut nss, &mut spi3, &mut spi_buffer);
    delay_us(&clock, 10);
    spi_buffer = [0b10000111, 0b00000000]; //read communication check
    transfer_spi(true, &mut nss, &mut spi3, &mut spi_buffer);
    delay_us(&clock, 10);
    
    // let pb1 = gpio::PB::<1>::init(gpio::PinMode::Analog, &clock);
    // let adc1_cfg = adc::Config {
    //     ..Default::default()
    // };
    // let mut adc1 = adc::Adc::<1>::init(pb1, adc1_cfg, &clock);

    loop {
        delay_ms(&clock, 1000);
        led.set_low();
        delay_ms(&clock, 1000);
        led.set_high();
        // rprintln!("adc1: {}", adc1.read(adc::Channel::C5));
    }
}

#![no_std]
#![no_main]

use core::{
    cell::RefCell,
    cell::UnsafeCell,
    sync::atomic::{AtomicU32, Ordering},
};
use cortex_m::{asm, interrupt::free, interrupt::{Mutex}};

use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};
use cortex_m_rt::entry;
use pac::interrupt;

use h7lib::*;
use periph::{pwr, rcc, gpio, adc, spi, timer, dma};
use plugin::pwm;

// static OVERFLOWS: AtomicU32 = AtomicU32::new(0);

// static TIMER: Mutex<RefCell<Option<timer::Timer<2>>>> =
//     Mutex::new(RefCell::new(None));

// fn transfer_spi<const NSS_PORT: char, const NSS_PIN: u8, 
//     const SPI_N: u8
// >(
//     rw: bool, nss: &mut gpio::Gpio<NSS_PORT, NSS_PIN>, 
//     spi: &mut spi::Spi<SPI_N>, spi_buffer: &mut [u8])
// {
//     nss.set_low();
//     if rw {
//         let result = spi.transfer(spi_buffer);
//         match result {
//             Ok(values) => {
//                 for (i, &value) in values.iter().enumerate() {
//                     rprintln!("Received data {}: {:#010b}", i, value);
//                     // rprintln!("Received data {}: {:#018b}", i, value);
//                 }
//             }
//             Err(e) => rprintln!("Error {:?}", e),
//         }
//     }
//     else {
//         let _ = spi.write(spi_buffer);
//     }
    
//     nss.set_high();
// }

#[link_section = ".sram3"]
static mut SPI_READ_BUF: [u8; 8] = [0; 8];

#[link_section = ".sram3"]
static mut SPI_WRITE_BUF: [u8; 8] = [0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x13, 0xEA];

static DMA: Mutex<RefCell<Option<dma::Dma<1>>>> =
    Mutex::new(RefCell::new(None));
static SPI: Mutex<RefCell<Option<spi::Spi<2>>>> =
    Mutex::new(RefCell::new(None));
static NSS: Mutex<RefCell<Option<gpio::PA<11>>>> =
    Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    rtt_init_print!();
    
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

    // let sck = gpio::PC::<10>::init(gpio::PinMode::AltFn(6, gpio::OutputType::PushPull), &clock);
    // let miso = gpio::PC::<11>::init(gpio::PinMode::AltFn(6, gpio::OutputType::PushPull), &clock);
    // let mosi = gpio::PC::<12>::init(gpio::PinMode::AltFn(6, gpio::OutputType::PushPull), &clock);
    // let mut nss = gpio::PA::<15>::init(gpio::PinMode::Output(gpio::OutputType::PushPull), &clock);
    // nss.set_high();

    // let mut en_gate = gpio::PA::<4>::init(gpio::PinMode::Output(gpio::OutputType::PushPull), &clock);
    // en_gate.set_high();
    // delay_us(&clock, 10);

    // let spi3_config = spi::SpiConfig {
    //     mode: spi::SpiMode::mode1(),
    //     data_size: spi::DataSize::D8,
    //     ..Default::default()
    // };

    // let mut spi3 = spi::Spi::<3>::init(sck, miso, mosi, spi3_config);

    let sck = gpio::PA::<12>::init(gpio::PinMode::AltFn(5, gpio::OutputType::PushPull), &clock);
    let miso = gpio::PB::<14>::init(gpio::PinMode::AltFn(5, gpio::OutputType::PushPull), &clock);
    let mosi = gpio::PB::<15>::init(gpio::PinMode::AltFn(5, gpio::OutputType::PushPull), &clock);
    let mut nss = gpio::PA::<11>::init(gpio::PinMode::Output(gpio::OutputType::PushPull), &clock);
    nss.set_high();

    let spi2_config = spi::SpiConfig {
        mode: spi::SpiMode::mode1(),
        data_size: spi::DataSize::D8,
        ..Default::default()
    };

    let mut spi2 = spi::Spi::<2>::init(sck, miso, mosi, spi2_config);

    let mut dma = dma::Dma::<1>::init();
    dma.mux1(dma::DmaChannel::C1, dma::DmaInput::Spi2Tx);
    dma.mux1(dma::DmaChannel::C2, dma::DmaInput::Spi2Rx);

    let mut spi_buf: [u8; 8] = [0x00, 0x00, 0xAA, 0xAA, 0x00, 0x00, 0xD0, 0xAB];
    nss.set_low();
    spi2.transfer(&mut spi_buf).ok();
    let values = spi_buf;
    for (i, &value) in values.iter().enumerate() {
        rprintln!("Received data 1 {}: {:#010x}", i, value);
    }
    nss.set_high();

    delay_ms(&clock, 1);

    nss.set_low();
    unsafe {
        spi2.transfer_dma(
            &SPI_WRITE_BUF,
            &mut SPI_READ_BUF,
            dma::DmaChannel::C1,
            dma::DmaChannel::C2,
            dma::ChannelCfg {
                priority: dma::Priority::Medium,
                circular: dma::Circular::Disabled,
                periph_incr: dma::IncrMode::Disabled,
                mem_incr: dma::IncrMode::Enabled,
            },
            dma::ChannelCfg {
                priority: dma::Priority::Medium,
                circular: dma::Circular::Disabled,
                periph_incr: dma::IncrMode::Disabled,
                mem_incr: dma::IncrMode::Enabled,
            },
            &mut dma,
        );
    }
    rprintln!("transferd!!!");

    cortex_m::interrupt::free(|cs| {
        DMA.borrow(cs).replace(Some(dma));
        SPI.borrow(cs).replace(Some(spi2));
        NSS.borrow(cs).replace(Some(nss));
    });

    unsafe {
        pac::NVIC::unmask(pac::interrupt::DMA1_STR1);
        pac::NVIC::unmask(pac::interrupt::DMA1_STR2);

        // cp.NVIC.set_priority(pac::Interrupt::DMA1_STR1, 0);
        // cp.NVIC.set_priority(pac::Interrupt::DMA1_STR2, 1);
    }

    // let mut spi_buffer: [u8; 2] = [0; 2];

    // spi_buffer = [0b10000000, 0b00000000]; //read fault
    // transfer_spi(true, &mut nss, &mut spi3, &mut spi_buffer);
    // delay_us(&clock, 10);

    // let rccregs = unsafe {&*pac::RCC::ptr()};
    // rprintln!("apb1henr {:#010x}", rccregs.apb1henr.read().bits());
    // rprintln!("apb1lenr {:#010x}", rccregs.apb1lenr.read().bits());
    // rprintln!("apb2enr {:#010x}", rccregs.apb2enr.read().bits());
    // let spiregs = unsafe {&*pac::SPI3::ptr()};
    // rprintln!("cr1: {:#010x}", spiregs.cr1.read().bits());
    // rprintln!("cr2: {:#010x}", spiregs.cr2.read().bits());
    // rprintln!("cfg1: {:#010x}", spiregs.cfg1.read().bits());
    // rprintln!("cfg2: {:#010x}", spiregs.cfg2.read().bits());
    // rprintln!("sr: {:#010x}", spiregs.sr.read().bits());

    // spi_buffer = [0b00001101, 0b01100000]; // write lock free
    // transfer_spi(false, &mut nss, &mut spi3, &mut spi_buffer);
    // delay_us(&clock, 10);
    // spi_buffer = [0b00000100, 0b10000000]; // write clear fault
    // transfer_spi(false, &mut nss, &mut spi3, &mut spi_buffer);
    // delay_us(&clock, 10);
    // spi_buffer = [0b00001110, 0b00000010]; // write ocp mode
    // transfer_spi(false, &mut nss, &mut spi3, &mut spi_buffer);
    // delay_us(&clock, 10);
    // spi_buffer = [0b10001110, 0b00000000]; //read ic11
    // transfer_spi(true, &mut nss, &mut spi3, &mut spi_buffer);
    // delay_us(&clock, 10);
    // spi_buffer = [0b10000000, 0b00000000]; //read fault
    // transfer_spi(true, &mut nss, &mut spi3, &mut spi_buffer);
    // delay_us(&clock, 10);
    // spi_buffer = [0b10000111, 0b00000000]; //read communication check
    // transfer_spi(true, &mut nss, &mut spi3, &mut spi_buffer);
    // delay_us(&clock, 10);
    
    // let pb1 = gpio::PB::<1>::init(gpio::PinMode::Analog, &clock);
    // let adc1_cfg = adc::Config {
    //     ..Default::default()
    // };
    // let mut adc1 = adc::Adc::<1>::init(pb1, adc1_cfg, &clock);

    // let tim1 = timer::Timer::<1>::init(timer::CountMode::Loop, &clock);
    // let ch_option = timer::ChannelOption {
    //     frequency: 10.kHz(),
    //     polarity: timer::Polarity::ActiveLow,
    //     alignment: Some(timer::Alignment::Center),
    //     deadtime: Some(1.micros()),
    // };
    // let (ch1, ch2, ch3, ch4) = tim1.split(ch_option);
    // let pe9 = gpio::PE::<9>::init(gpio::PinMode::AltFn(1, gpio::OutputType::PushPull), &clock);
    // let pe11 = gpio::PE::<11>::init(gpio::PinMode::AltFn(1, gpio::OutputType::PushPull), &clock);
    // let pe13 = gpio::PE::<13>::init(gpio::PinMode::AltFn(1, gpio::OutputType::PushPull), &clock);
    // let pe8 = gpio::PE::<8>::init(gpio::PinMode::AltFn(1, gpio::OutputType::PushPull), &clock);
    // let pe10 = gpio::PE::<10>::init(gpio::PinMode::AltFn(1, gpio::OutputType::PushPull), &clock);
    // let pe12 = gpio::PE::<12>::init(gpio::PinMode::AltFn(1, gpio::OutputType::PushPull), &clock);
    // let mut pwm1 = pwm::Pwm::<1, 1>::new_with_comp(ch1, pe9, pe8);
    // let mut pwm2 = pwm::Pwm::<1, 2>::new_with_comp(ch2, pe11, pe10);
    // let mut pwm3 = pwm::Pwm::<1, 3>::new_with_comp(ch3, pe13, pe12);
    
    // pwm1.set_duty(pwm1.get_max_duty() / 2);
    // pwm2.set_duty(pwm2.get_max_duty() / 2);
    // pwm3.set_duty(pwm3.get_max_duty() / 2);

    // pwm1.enable();
    // pwm2.enable();
    // pwm3.enable();

    // let tim1regs = unsafe {&*pac::TIM1::ptr()};
    // rprintln!("bdtr {:#010x}", tim1regs.bdtr.read().bits());
    // rprintln!("ccer {:#010x}", tim1regs.ccer.read().bits());
    // rprintln!("ccmr1_output {:#010x}", tim1regs.ccmr1_output().read().bits());
    // rprintln!("ccmr2_output {:#010x}", tim1regs.ccmr2_output().read().bits());

    // let mut tim2 = timer::Timer::<2>::init(timer::CountMode::Interrupt, &clock);

    // tim2.start(10.Hz());
    // tim2.listen();

    // cortex_m::interrupt::free(|cs| {
    //     TIMER.borrow(cs).replace(Some(tim2));
    // });

    // unsafe {
    //     // cp.NVIC.set_priority(pac::interrupt::TIM2, 0);
    //     pac::NVIC::unmask(pac::interrupt::TIM2);
    // }

    loop {
        // let ctr = cortex_m::interrupt::free(|cs| {
        //     let rc = TIMER.borrow(cs).borrow();
        //     let timer = rc.as_ref().unwrap();
        //     timer.counter() as u64
        // });
        // rprintln!("count{}", ctr);
        // rprintln!("overflows: {}", OVERFLOWS.load(Ordering::SeqCst) as u64);
        // delay_ms(&clock, 1000);
        // led.set_low();
        // delay_ms(&clock, 1000);
        // led.set_high();
        // // rprintln!("adc1: {}", adc1.read(adc::Channel::C5));
        loop {
            delay_us(&clock, 800);
            free(|cs|{
                let mut s = NSS.borrow(cs).borrow_mut();
                let nss = s.as_mut().unwrap();
                if nss.is_high() {
                    let mut s = SPI.borrow(cs).borrow_mut();
                    let spi2 = s.as_mut().unwrap();
                    let mut rc = DMA.borrow(cs).borrow_mut();
                    let mut dma = rc.as_mut().unwrap();
                    
                    nss.set_low();
                    
                    unsafe {
                        SPI_WRITE_BUF = [0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x13, 0xEA];
                        spi2.transfer_dma(
                            &SPI_WRITE_BUF,
                            &mut SPI_READ_BUF,
                            dma::DmaChannel::C1,
                            dma::DmaChannel::C2,
                            dma::ChannelCfg {
                                priority: dma::Priority::Medium,
                                circular: dma::Circular::Disabled,
                                periph_incr: dma::IncrMode::Disabled,
                                mem_incr: dma::IncrMode::Enabled,
                            },
                            dma::ChannelCfg {
                                priority: dma::Priority::Medium,
                                circular: dma::Circular::Disabled,
                                periph_incr: dma::IncrMode::Disabled,
                                mem_incr: dma::IncrMode::Enabled,
                            },
                            &mut dma,
                        );
                    }
                }
            });
        }
    }
}

// #[interrupt]
// fn TIM2() {
//     OVERFLOWS.fetch_add(1, core::sync::atomic::Ordering::SeqCst);

//     cortex_m::interrupt::free(|cs| {
//         let mut rc = TIMER.borrow(cs).borrow_mut();
//         let timer = rc.as_mut().unwrap();
//         timer.clear_irq();
//     })
// }

#[interrupt]
fn DMA1_STR1() {
    cortex_m::interrupt::free(|cs| {
        let mut rc = DMA.borrow(cs).borrow_mut();
        let dma = rc.as_mut().unwrap();
        let is = dma.transfer_is_complete(dma::DmaChannel::C1);
        rprintln!("transfer is complete? : {}", is);

        dma.clear_interrupt(dma::DmaChannel::C1, dma::DmaInterrupt::TransferComplete);
    });
    rprintln!("transmit complete");
}

#[interrupt]
fn DMA1_STR2() {
    cortex_m::interrupt::free(|cs| {
        let mut rc = DMA.borrow(cs).borrow_mut();
        let dma = rc.as_mut().unwrap();
        let mut rc = SPI.borrow(cs).borrow_mut();
        let spi = rc.as_mut().unwrap();
        let mut rc = NSS.borrow(cs).borrow_mut();
        let nss = rc.as_mut().unwrap();
        dma.clear_interrupt(dma::DmaChannel::C2, dma::DmaInterrupt::TransferComplete);
        spi.stop_dma(dma::DmaChannel::C2, Some(dma::DmaChannel::C1), dma);
        nss.set_high();
    });

    let values = unsafe { SPI_READ_BUF };
    for (i, &value) in values.iter().enumerate() {
        rprintln!("Received data 2 {}: {:#010x}", i, value);
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

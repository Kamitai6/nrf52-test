#![no_std]
#![no_main]

use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

use core::cell::{Cell, RefCell};
use cortex_m::interrupt::{free, Mutex};
use cortex_m::delay::Delay;
use cortex_m::peripheral::NVIC;
use cortex_m_rt::entry;
use hal::{
    clocks::{Clocks, PllCfg},
    dma::{self, Dma, DmaChannel, DmaInput, DmaInterrupt, DmaPeriph},
    gpio::{self, Pin, PinMode, Port},
    pac::{self, DMA1, SPI2, interrupt},
    spi::{self, BaudRate, Spi, SpiConfig, SpiMode},
};

#[link_section = ".sram3"]
static mut SPI_READ_BUF: [u8; 8] = [0; 8];

#[link_section = ".sram3"]
static mut SPI_WRITE_BUF: [u8; 8] = [0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x13, 0xEA];

static SPI: Mutex<RefCell<Option<Spi<SPI2>>>> = Mutex::new(RefCell::new(None));
static NSS: Mutex<RefCell<Option<Pin>>> = Mutex::new(RefCell::new(None));

pub struct Drv8343Reg {
    pub fault_status: u8,
    pub diag_status: [u8; 3],
    pub control: [u8; 14],
}

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("Start!!!");

    let mut cp = cortex_m::Peripherals::take().unwrap();
    let mut dp = pac::Peripherals::take().unwrap();

    let mut clock_cfg = Clocks::default(); //400MHz

    clock_cfg.pll1 = PllCfg {
        enabled: true,
        pllp_en: true,
        pllq_en: true,
        pllr_en: true,
        divm: 32,
        divn: 200,
        divp: 2,
        divq: 4,
        divr: 2,
    };

    clock_cfg.setup().unwrap();

    let _sck = Pin::new(Port::C, 10, PinMode::Alt(5));
    let _miso = Pin::new(Port::C, 11, PinMode::Alt(5));
    let _mosi = Pin::new(Port::C, 12, PinMode::Alt(5));
    let mut nss = Pin::new(Port::A, 15, PinMode::Output);
    nss.set_high();

    let mut led = Pin::new(Port::D, 0, PinMode::Output);
    led.set_high();

    let mut delay = Delay::new(cp.SYST, clock_cfg.systick());

    // `SpiConfig::default` is mode 0, full duplex, with software CS.
    let spi2_cfg = SpiConfig {
        mode: SpiMode::mode1(),
        ..Default::default()
    };

    let mut spi2 = Spi::new(
        dp.SPI2,
        spi2_cfg,
        BaudRate::Div128,
    );

    let mut dma = Dma::new(dp.DMA1);

    dma::mux(DmaPeriph::Dma1, DmaChannel::C1, DmaInput::Spi2Tx);
    dma::mux(DmaPeriph::Dma1, DmaChannel::C2, DmaInput::Spi2Rx);

    let mut spi_buf: [u8; 2] = [(0b1 << 15) | (0b0000000 << 8), 0b00000000]; //fault statusを読みたい
    nss.set_low();
    // delay.delay_us(1_00);
    spi2.transfer(&mut spi_buf).ok();
    let values = spi_buf;
    for (i, &value) in values.iter().enumerate() {
        rprintln!("Received data 1 {}: {:#010x}", i, value);
    }
    nss.set_high();

    delay.delay_us(8_00);

    nss.set_low();

    // unsafe {
    //     spi2.transfer_dma(
    //         &SPI_WRITE_BUF,
    //         &mut SPI_READ_BUF,
    //         DmaChannel::C1,
    //         DmaChannel::C2,
    //         dma::ChannelCfg {
    //             priority: dma::Priority::Medium,
    //             circular: dma::Circular::Disabled,
    //             periph_incr: dma::IncrMode::Disabled,
    //             mem_incr: dma::IncrMode::Enabled,
    //         },
    //         dma::ChannelCfg {
    //             priority: dma::Priority::Medium,
    //             circular: dma::Circular::Disabled,
    //             periph_incr: dma::IncrMode::Disabled,
    //             mem_incr: dma::IncrMode::Enabled,
    //         },
    //         DmaPeriph::Dma1,
    //     );
    // }
    // rprintln!("transferd!!!");

    // free(|cs| {
    //     SPI.borrow(cs).replace(Some(spi2));
    //     NSS.borrow(cs).replace(Some(nss));
    // });

    // unsafe {
    //     NVIC::unmask(pac::Interrupt::DMA1_STR1);
    //     NVIC::unmask(pac::Interrupt::DMA1_STR2);

    //     // Set interrupt priority. See the reference manual's NVIC section for details.
    //     cp.NVIC.set_priority(pac::Interrupt::DMA1_STR1, 0);
    //     cp.NVIC.set_priority(pac::Interrupt::DMA1_STR2, 1);
    // }

    loop {
        // delay.delay_us(8_00);
        // free(|cs|{
        //     let mut s = NSS.borrow(cs).borrow_mut();
        //     let nss = s.as_mut().unwrap();
        //     if nss.is_high() {
        //         let mut s = SPI.borrow(cs).borrow_mut();
        //         let spi2 = s.as_mut().unwrap();
                
        //         nss.set_low();
                
        //         unsafe {
        //             SPI_WRITE_BUF = [0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x13, 0xEA];
        //             spi2.transfer_dma(
        //                 &SPI_WRITE_BUF,
        //                 &mut SPI_READ_BUF,
        //                 DmaChannel::C1,
        //                 DmaChannel::C2,
        //                 dma::ChannelCfg {
        //                     priority: dma::Priority::Medium,
        //                     circular: dma::Circular::Disabled,
        //                     periph_incr: dma::IncrMode::Disabled,
        //                     mem_incr: dma::IncrMode::Enabled,
        //                 },
        //                 dma::ChannelCfg {
        //                     priority: dma::Priority::Medium,
        //                     circular: dma::Circular::Disabled,
        //                     periph_incr: dma::IncrMode::Disabled,
        //                     mem_incr: dma::IncrMode::Enabled,
        //                 },
        //                 DmaPeriph::Dma1,
        //             );
        //         }
        //     }
        // });
    }
}

#[interrupt]
fn DMA1_STR1() {
    free(|cs|{
        dma::clear_interrupt(DmaPeriph::Dma1, DmaChannel::C1, DmaInterrupt::TransferComplete);
    });
    rprintln!("transmit complete");
}

#[interrupt]
fn DMA1_STR2() {
    dma::clear_interrupt(DmaPeriph::Dma1, DmaChannel::C2, DmaInterrupt::TransferComplete);

    free(|cs|{
        let mut s = SPI.borrow(cs).borrow_mut();
        let spi2 = s.as_mut().unwrap();
        spi2.stop_dma(DmaChannel::C2, Some(DmaChannel::C1), DmaPeriph::Dma1);

        let mut s = NSS.borrow(cs).borrow_mut();
        let nss = s.as_mut().unwrap();
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
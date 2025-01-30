#![no_std]
#![no_main]

use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

use core::cell::{Cell, RefCell};

use cortex_m::peripheral::NVIC;
use cortex_m::delay::Delay;
use cortex_m_rt::entry;
use hal::{
    clocks::Clocks,
    dma::{self, Dma, DmaChannel, DmaInput, DmaInterrupt, DmaPeriph},
    gpio::{self, Pin, PinMode, Port},
    low_power,
    pac::{self, interrupt},
    prelude::*,
    spi::{self, BaudRate, Spi, SpiConfig, SpiMode},
};

// Byte 0 is for the address we pass in the `write` transfer; relevant data is in the rest of
// the values.
static mut SPI_READ_BUF: [u8; 8] = [0; 8];
static mut SPI_WRITE_BUF: [u8; 8] = [0x00, 0x00, 0xAA, 0xAA, 0x00, 0x00, 0xD0, 0xAB];

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("Start!!!");
    // Set up CPU peripherals
    let mut cp = cortex_m::Peripherals::take().unwrap();
    // Set up microcontroller peripherals
    let mut dp = pac::Peripherals::take().unwrap();

    let mut clock_cfg = Clocks::default(); //400MHz

    clock_cfg.pll1.pllq_en = true;

    clock_cfg.setup().unwrap();

    // Configure pins for Spi
    let _sck = Pin::new(Port::A, 12, PinMode::Alt(12));
    let _miso = Pin::new(Port::B, 14, PinMode::Alt(14));
    let _mosi = Pin::new(Port::B, 15, PinMode::Alt(15));

    let mut cs = Pin::new(Port::A, 11, PinMode::Output);

    let mut led = Pin::new(Port::D, 0, PinMode::Output);
    let mut delay = Delay::new(cp.SYST, clock_cfg.systick());

    let spi_cfg = SpiConfig {
        mode: SpiMode::mode1(),
        // `SpiConfig::default` is mode 0, full duplex, with software CS.
        ..Default::default()
    };

    // Set up an SPI peripheral, running at 4Mhz, in SPI mode 0.
    let mut spi = Spi::new(
        dp.SPI2,
        spi_cfg,
        BaudRate::Div256, // Eg 80Mhz apb clock / 32 = 2.5Mhz SPI clock.
    );

    // Set up DMA, for nonblocking (generally faster) conversion transfers:
    // let mut dma = Dma::new(dp.DMA1);

    // // Associate a pair of DMA channels with SPI1: One for transmit; one for receive.
    // // Note that mux is not used on F3, F4, and most L4s: DMA channels are hard-coded
    // // to peripherals on those platforms.
    // dma::mux(DmaPeriph::Dma2, DmaChannel::C1, DmaInput::Spi2Tx);
    // dma::mux(DmaPeriph::Dma2, DmaChannel::C2, DmaInput::Spi2Rx);

    // unsafe {
    //     // Write to SPI, using DMA.
    //     // spi.write_dma(&write_buf, DmaChannel::C1, Default::default(), DmaPeriph::Dma2);

    //     // Read (transfer) from SPI, using DMA.
    //     spi.transfer_dma(
    //         // Write buffer, starting with the registers we'd like to access, and 0-padded to
    //         // read 3 bytes.
    //         &SPI_WRITE_BUF,
    //         &mut SPI_READ_BUF,  // Read buf, where the data will go
    //         DmaChannel::C1,     // Write channel
    //         DmaChannel::C2,     // Read channel
    //         Default::default(), // Write channel config
    //         Default::default(), // Read channel config
    //         DmaPeriph::Dma2,
    //     );
    // }

    // Alternatively, use the blocking, non-DMA SPI API` (Also supports `embedded-hal` traits):

    // Assign peripheral structs as global, so we can access them in interrupts.
    // with(|cs| {
    //     DMA.borrow(cs).replace(Some(dma));
    //     SPI.borrow(cs).replace(Some(spi));
    // });

    // Unmask the interrupt line for DMA read complete. See the `DMA_CH3` interrupt handlers below,
    // where we set CS high, terminal the DMA read, and display the data read.
    // unsafe {
    //     NVIC::unmask(pac::Interrupt::DMA1_CH2);
    // }

    // Alternatively, we can take readings without DMA. This provides a simpler, memory-safe API,
    // and is compatible with the `embedded_hal::blocking::i2c traits.

    led.set_high();

    let mut write_buf: [u8; 8] = [0x00, 0x00, 0xAA, 0xAA, 0x00, 0x00, 0xD0, 0xAB];
    let mut read_buf: [u8; 8] = [0; 8];
    cs.set_low();
    delay.delay_us(1_00);
    spi.write(&write_buf).ok();
    spi.transfer(&mut read_buf).ok();
    let values = read_buf;
    for (i, &value) in values.iter().enumerate() {
        rprintln!("Received data 1 {}: {}", i, value);
    }
    cs.set_high();

    loop {
        delay.delay_us(8_00);
        write_buf = [0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x13, 0xEA];
        read_buf = [0; 8];
        cs.set_low();
        delay.delay_us(1_00);
        spi.write(&write_buf).ok();
        spi.transfer(&mut read_buf).ok();
        let values = read_buf;
        for (i, &value) in values.iter().enumerate() {
            rprintln!("Received data 2 {}: {}", i, value);
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
        cs.set_high();
    }
}

// #[interrupt]
// /// This interrupt fires when a DMA read is complete
// fn DMA1_CH2() {
//     dma::clear_interrupt(
//         DmaPeriph::Dma2,
//         DmaChannel::C2,
//         DmaInterrupt::TransferComplete,
//     );
//     with(|cs| {
//         rprintln!("SPI DMA read complete");
//         access_global!(SPI, spi, cs);
//         spi.stop_dma(DmaChannel::C1, Some(DmaChannel::C2), DmaPeriph::Dma2);

//         // See also this convenience function, which clears the interrupt and stops othe Txfer.:
//         spi.cleanup_dma(DmaPeriph::Dma2, DmaChannel::C1, Some(DmaChannel::C2));

//         unsafe {
//             // Ignore byte 0, which is the reg we passed during the write.
//             rprintln!("Data read: {:?}", SPI_READ_BUF);
//         }

//         // Set CS high.
//         gpio::set_high(Port::A, 1);
//     })
// }
#![no_std]
#![no_main]

use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};
use stm32h7xx_hal::dma::traits::TargetAddress;

// use rtic::app;

// use cmsis_dsp_api as dsp_api;
// use cmsis_dsp_sys as dsp_sys;

use core::cell::{Cell, RefCell};
use cortex_m::interrupt::{free, Mutex};
use cortex_m::delay::Delay;
use cortex_m::peripheral::NVIC;
use cortex_m_rt::entry;

use core::mem::MaybeUninit;

use cortex_m::asm;

use stm32h7xx_hal::device;
use stm32h7xx_hal::dma;
use stm32h7xx_hal::rcc;
use stm32h7xx_hal::time::Hertz;
use stm32h7xx_hal::rcc::rec::Sai1ClkSel;
use stm32h7xx_hal::spi::{self, SpiDmaExt};
use stm32h7xx_hal::gpio::Pin;
use stm32h7xx_hal::spi::Spi;
use stm32h7xx_hal::sai::{self, I2sUsers, SaiChannel, SaiI2sExt};
use stm32h7xx_hal::stm32;
use stm32h7xx_hal::{pac, prelude::*};

use pac::interrupt;


#[link_section = ".sram3"]
static mut SPI_READ_BUF: [u8; 8] = [0; 8];

#[link_section = ".sram3"]
static mut SPI_WRITE_BUF: [u8; 8] = [0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x13, 0xEA];

type TransferDma1Str1 = dma::Transfer<
    dma::dma::Stream1<stm32::DMA1>,
    sai::dma::ChannelA<stm32::SAI1>,
    dma::PeripheralToMemory,
    &'static mut [u32; 2],
    dma::DBTransfer,
>;

static SPI: Mutex<RefCell<Option<TransferDma1Str1>>> = Mutex::new(RefCell::new(None));
// static NSS: Mutex<RefCell<Option<Pin>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("Start!!!");

    let mut dp = pac::Peripherals::take().unwrap();
    let mut cp = device::CorePeripherals::take().unwrap();

    rprintln!("Setup RCC...                  ");
    let pwr = dp.PWR.constrain();
    let vos = pwr.freeze();
    let ccdr = dp
        .RCC
        .constrain()
        .sys_ck(100.MHz())
        .pclk1(100.MHz()) // DMA clock
        // PLL1
        // .pll1_strategy(rcc::PllConfigStrategy::Iterative)
        .pll1_p_ck(100.MHz())
        // PLL3
        // .pll3_strategy(rcc::PllConfigStrategy::Iterative)
        // .pll3_p_ck(PLL3_P_HZ)
        .freeze(vos, &dp.SYSCFG);

    cp.SCB.enable_icache();

    let gpioa = dp.GPIOA.split(ccdr.peripheral.GPIOA);
    let gpiob = dp.GPIOB.split(ccdr.peripheral.GPIOB);
    let gpioc = dp.GPIOC.split(ccdr.peripheral.GPIOC);
    let gpiod = dp.GPIOD.split(ccdr.peripheral.GPIOD);

    let sck = gpioa.pa12.into_alternate();
    let miso = gpiob.pb14.into_alternate();
    let mosi = gpiob.pb15.into_alternate();
    let mut nss = gpioa.pa11.into_push_pull_output();
    nss.set_high();

    let mut led = gpiod.pd0.into_push_pull_output();
    led.set_high();

    // let mut spi: spi::Spi<_, _, u8> = dp.SPI2.spi(
    //     (sck, miso, mosi),
    //     spi::MODE_1,
    //     2.MHz(),
    //     ccdr.peripheral.SPI2,
    //     &ccdr.clocks,
    // );

    let spi = {
        let mut spi: spi::Spi<_, _, u8> = dp.SPI2.spi(
            (sck, miso, mosi),
            spi::MODE_1,
            2.MHz(),
            ccdr.peripheral.SPI2,
            &ccdr.clocks,
        );

        spi.disable()
    };

    let dma1_streams = dma::dma::StreamsTuple::new(dp.DMA1, ccdr.peripheral.DMA1);

    #[allow(static_mut_refs)] // TODO: Fix this
    let dma_config = dma::dma::DmaConfig::default()
        .priority(dma::config::Priority::High)
        .memory_increment(true)
        .peripheral_increment(false)
        .circular_buffer(true)
        .fifo_enable(false);
    let mut dma1_str0: dma::Transfer<_, _, dma::MemoryToPeripheral, _, _> =
        dma::Transfer::init(
            dma1_streams.0,
            unsafe { pac::Peripherals::steal().SPI1 },
            unsafe { &mut SPI_WRITE_BUF },
            None,
            dma_config,
        );

    #[allow(static_mut_refs)] // TODO: Fix this
    let dma_config = dma_config
        .transfer_complete_interrupt(true)
        .half_transfer_interrupt(true);
    let mut dma1_str1: dma::Transfer<_, _, dma::PeripheralToMemory, _, _> =
        dma::Transfer::init(
            dma1_streams.1,
            unsafe { pac::Peripherals::steal().SPI1 },
            unsafe { &mut SPI_READ_BUF },
            None,
            dma_config,
        );

    nss.set_high();
    led.set_high();

    unsafe {
        NVIC::unmask(pac::Interrupt::DMA1_STR1);
        NVIC::unmask(pac::Interrupt::DMA1_STR2);

        // Set interrupt priority. See the reference manual's NVIC section for details.
        cp.NVIC.set_priority(pac::Interrupt::DMA1_STR1, 0);
        cp.NVIC.set_priority(pac::Interrupt::DMA1_STR2, 1);
    }

    dma1_str1.start(|spi| {
        spi.enable_dma_rx();
    });

    // dma1_str0.start(|sai1_rb| {
    //     spi.enable_dma(SaiChannel::ChannelA);
    //     info!("sai1 fifo waiting to receive data");
    //     while sai1_rb.cha().sr.read().flvl().is_empty() {}
    //     info!("audio started");

    //     sai1.enable();
    //     sai1.try_send(0, 0).unwrap();
    // });

    // free(|cs| {
    //     SPI.borrow(cs).replace(Some(spi2));
    //     NSS.borrow(cs).replace(Some(nss));
    // });

    loop {
        // // delay.delay_us(8_00);
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

// #[interrupt]
// fn DMA1_STR1() {
//     #[allow(static_mut_refs)] // TODO: Fix this
//     let tx_buffer: &'static mut [u32; DMA_BUFFER_LENGTH] =
//         unsafe { TX_BUFFER.assume_init_mut() };
//     #[allow(static_mut_refs)] // TODO: Fix this
//     let rx_buffer: &'static mut [u32; DMA_BUFFER_LENGTH] =
//         unsafe { RX_BUFFER.assume_init_mut() };

//     let stereo_block_length = tx_buffer.len() / 2;

//     #[allow(static_mut_refs)] // TODO: Fix this
//     if let Some(transfer) = unsafe { TRANSFER_DMA1_STR1.assume_init_mut() }
//     {
//         let skip = if transfer.get_half_transfer_flag() {
//             transfer.clear_half_transfer_interrupt();
//             (0, stereo_block_length)
//         } else if transfer.get_transfer_complete_flag() {
//             transfer.clear_transfer_complete_interrupt();
//             (stereo_block_length, 0)
//         } else {
//             return;
//         };

//         // pass thru
//         let mut index = 0;
//         while index < stereo_block_length {
//             let tx0 = index + skip.0;
//             let tx1 = tx0 + 1;
//             let rx0 = index + skip.1;
//             let rx1 = rx0 + 1;

//             tx_buffer[tx0] = rx_buffer[rx0];
//             tx_buffer[tx1] = rx_buffer[rx1];

//             index += 2;
//         }
//     }

//     free(|cs|{
//         // let is = dma.transfer_is_complete(DmaChannel::C1);
//         // rprintln!("transfer is complete? : {}", is);

//         dma::clear_interrupt(DmaPeriph::Dma1, DmaChannel::C1, DmaInterrupt::TransferComplete);
//     });
//     // (ctx.shared.dma).lock(|dma| {
//         // let is = dma.transfer_is_complete(DmaChannel::C1);
//         // rprintln!("transfer is complete? : {}", is);

//         // dma::clear_interrupt(DmaPeriph::Dma1, DmaChannel::C1, DmaInterrupt::TransferComplete);
//     // });
//     rprintln!("transmit complete");
// }

// #[interrupt]
// fn DMA1_STR2() {
//     dma::clear_interrupt(DmaPeriph::Dma1, DmaChannel::C2, DmaInterrupt::TransferComplete);

//     free(|cs|{
//         let mut s = SPI.borrow(cs).borrow_mut();
//         let spi2 = s.as_mut().unwrap();
//         // (ctx.shared.spi2).lock(|spi| {
//             spi2.stop_dma(DmaChannel::C2, Some(DmaChannel::C1), DmaPeriph::Dma1);
//         // });

//         let mut s = NSS.borrow(cs).borrow_mut();
//         let nss = s.as_mut().unwrap();
//         nss.set_high();
//     });

//     let values = unsafe { SPI_READ_BUF };
//     for (i, &value) in values.iter().enumerate() {
//         rprintln!("Received data 2 {}: {:#010x}", i, value);
//     }
//     let angle_lsb = ((values[1] & 0x3F) as u16) << 8 | (values[0] as u16);
//     rprintln!("angle {}", angle_lsb);
//     let error_lsb = (values[1] as u16) >> 6;
//     rprintln!("error {}", error_lsb);
//     let crc_lsb = (values[7] as u16);
//     rprintln!("crc {}", crc_lsb);
//     let vgain_lsb = (values[4] as u16);
//     rprintln!("vgain {}", vgain_lsb);
//     let rollcnt_lsb = (values[6] as u16) & 0x3F;
//     rprintln!("rollcnt {}", rollcnt_lsb);
// }
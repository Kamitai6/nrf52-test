#![no_std]
#![no_main]

use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

use core::cell::{Cell, RefCell};

use rtic::app;

// use cmsis_dsp_api as dsp_api;
// use cmsis_dsp_sys as dsp_sys;
// use cortex_m::peripheral::NVIC;
// use cortex_m::delay::Delay;
// use cortex_m::interrupt::free;
// use cortex_m_rt::entry;
use hal::{
    clocks::{Clocks, PllCfg},
    dma::{self, Dma, DmaChannel, DmaInput, DmaInterrupt, DmaPeriph},
    gpio::{self, Pin, PinMode, Port},
    low_power,
    pac::{self, DMA1, SPI2, interrupt},
    prelude::*,
    spi::{self, BaudRate, Spi, SpiConfig, SpiMode},
};

static mut SPI_READ_BUF: [u8; 8] = [0; 8];
static mut SPI_WRITE_BUF: [u8; 8] = [0x00, 0x00, 0xAA, 0xAA, 0x00, 0x00, 0xD0, 0xAB];

#[app(device = pac, peripherals = false)]
mod app {
    use super::*;

    #[shared]
    struct Shared {
        dma: Dma<DMA1>,
        spi2: Spi<SPI2>,
        nss: Pin,
    }

    #[local]
    struct Local {}

    #[init]
    fn init(mut ctx: init::Context) -> (Shared, Local) {
        rtt_init_print!();
        rprintln!("Start!!!");
        // Set up CPU peripherals
        let mut cp = ctx.core;
        // Set up microcontroller peripherals
        let mut dp = pac::Peripherals::take().unwrap();

        let mut clock_cfg = Clocks::default(); //400MHz

        // ---------------clock---------------
        // hsi(pllsrc) / divm -> pll1-ref-clk
        // pll1-ref-clk * divp(or divq or divr) -> pll1p(or pll1q or pll1r) clk
        clock_cfg.pll1 = PllCfg {
            enabled: true,
            // fractional: false,
            pllp_en: true,
            pllq_en: true,
            pllr_en: true,
            divm: 32,// 400MHz / 32 = 12.5MHz
            divn: 400,
            divp: 2,// pll1p clock = 12.5 * 2 = 25MHz
            divq: 8,// pll1q clock = 12.5 * 8 = 100MHz
            divr: 2,// pll1r clock = 12.5 * 2 = 25MHz
        };

        clock_cfg.setup().unwrap();

        // Configure pins for Spi
        let _sck = Pin::new(Port::A, 12, PinMode::Alt(12));
        let _miso = Pin::new(Port::B, 14, PinMode::Alt(14));
        let _mosi = Pin::new(Port::B, 15, PinMode::Alt(15));

        let mut nss = Pin::new(Port::A, 11, PinMode::Output);
        nss.set_high();
        let mut led = Pin::new(Port::D, 0, PinMode::Output);
        led.set_high();

        // `SpiConfig::default` is mode 0, full duplex, with software CS.
        let spi2_cfg = SpiConfig {
            mode: SpiMode::mode1(),
            ..Default::default()
        };

        // Set up an SPI peripheral, running at 4Mhz, in SPI mode 0.
        let mut spi2 = Spi::new(
            dp.SPI2,
            spi2_cfg,
            BaudRate::Div64, // 100MHz / 64 = 1.5625MHz
        );

        let mut dma = Dma::new(dp.DMA1);

        dma::mux(DmaPeriph::Dma1, DmaChannel::C1, DmaInput::Spi2Tx);
        dma::mux(DmaPeriph::Dma1, DmaChannel::C2, DmaInput::Spi2Rx);

        dma.enable_interrupt(DmaChannel::C1, DmaInterrupt::TransferComplete);
        dma.enable_interrupt(DmaChannel::C1, DmaInterrupt::TransferError);
        dma.enable_interrupt(DmaChannel::C2, DmaInterrupt::TransferComplete);

        (
            Shared {
                dma,
                spi2,
                nss,
            },
            Local {},
        )
    }

    #[idle(shared = [spi2, nss])]
    fn idle(mut ctx: idle::Context) -> ! {
        (ctx.shared.nss, ctx.shared.spi2).lock(|nss, spi2| {
            unsafe {
                SPI_WRITE_BUF = [0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x13, 0xEA];
                // SPI_WRITE_BUF = [0x00, 0x00, 0xAA, 0xAA, 0x00, 0x00, 0xD0, 0xAB];
            }
    
            nss.set_low();
    
            unsafe {
                spi2.transfer_dma(
                    &SPI_WRITE_BUF,
                    &mut SPI_READ_BUF,
                    DmaChannel::C1,
                    DmaChannel::C2,
                    dma::ChannelCfg {
                        priority: dma::Priority::Medium,
                        circular: dma::Circular::Disabled,
                        // Increment the buffer address, not the peripheral address.
                        periph_incr: dma::IncrMode::Disabled,
                        mem_incr: dma::IncrMode::Enabled,
                    },
                    Default::default(),
                    DmaPeriph::Dma1,
                );
            }
            rprintln!("transferd!!!");
        });

        // let mut nss_is_high = false;
        // ctx.shared.nss.lock(|nss| {
        //     nss_is_high = nss.is_high();
        // });
        // if nss_is_high {
        //     (ctx.shared.nss, ctx.shared.spi2).lock(|nss, spi2| {
        //         unsafe {
        //             SPI_WRITE_BUF = [0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x13, 0xEA];
        //         }
        
        //         nss.set_low();
        
        //         unsafe {
        //             spi2.transfer_dma(
        //                 &SPI_WRITE_BUF,
        //                 &mut SPI_READ_BUF,
        //                 DmaChannel::C1,
        //                 DmaChannel::C2,
        //                 Default::default(),
        //                 Default::default(),
        //                 DmaPeriph::Dma1,
        //             );
        //         }
        //     });
        // }

        loop {
        }
    }

    #[task(binds = DMA1_STR1, shared = [dma], priority = 2)]
    fn test(mut ctx: test::Context) {
        (ctx.shared.dma).lock(|dma| {
            let is = dma.transfer_is_complete(DmaChannel::C1);
            rprintln!("transfer is complete? : {}", is);
        });
        rprintln!("transmit complete");
    }

    #[task(binds = DMA1_STR2, shared = [spi2, nss], priority = 2)]
    fn imu_tc_isr(mut ctx: imu_tc_isr::Context) {
        dma::clear_interrupt(
            DmaPeriph::Dma1,
            DmaChannel::C1,
            DmaInterrupt::TransferComplete,
        );

        (ctx.shared.spi2).lock(|spi| {
            // Note that this step is mandatory, per STM32 RM.
            spi.stop_dma(DmaChannel::C1, Some(DmaChannel::C2), DmaPeriph::Dma1);
        });

        ctx.shared.nss.lock(|nss| {
            nss.set_high();
        });

        let values = unsafe { SPI_READ_BUF };
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
    }
}
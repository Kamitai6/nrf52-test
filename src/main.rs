#![no_std]
#![no_main]

use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

use rtic::app;

// use cmsis_dsp_api as dsp_api;
// use cmsis_dsp_sys as dsp_sys;
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

        // Configure pins for Spi
        let _sck = Pin::new(Port::A, 12, PinMode::Alt(5));
        let _miso = Pin::new(Port::B, 14, PinMode::Alt(5));
        let _mosi = Pin::new(Port::B, 15, PinMode::Alt(5));
        let mut nss = Pin::new(Port::A, 11, PinMode::Output);
        nss.set_high();

        let mut led = Pin::new(Port::D, 0, PinMode::Output);
        led.set_high();

        // `SpiConfig::default` is mode 0, full duplex, with software CS.
        let spi2_cfg = SpiConfig {
            mode: SpiMode::mode1(),
            ..Default::default()
        };

        let mut spi2 = Spi::new(
            dp.SPI2,
            spi2_cfg,
            BaudRate::Div64,
        );

        let mut dma = Dma::new(dp.DMA1);

        dma::mux(DmaPeriph::Dma1, DmaChannel::C1, DmaInput::Spi2Tx);
        dma::mux(DmaPeriph::Dma1, DmaChannel::C2, DmaInput::Spi2Rx);

        let mut spi_buf: [u8; 8] = [0x00, 0x00, 0xAA, 0xAA, 0x00, 0x00, 0xD0, 0xAB];
        nss.set_low();
        // delay.delay_us(1_00);
        spi2.transfer(&mut spi_buf).ok();
        let values = spi_buf;
        for (i, &value) in values.iter().enumerate() {
            rprintln!("Received data 1 {}: {:#010x}", i, value);
        }
        nss.set_high();

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
                        periph_incr: dma::IncrMode::Disabled,
                        mem_incr: dma::IncrMode::Enabled,
                    },
                    dma::ChannelCfg {
                        priority: dma::Priority::Medium,
                        circular: dma::Circular::Disabled,
                        periph_incr: dma::IncrMode::Disabled,
                        mem_incr: dma::IncrMode::Enabled,
                    },
                    DmaPeriph::Dma1,
                );
            }
            rprintln!("transferd!!!");
        });

        loop {
        }
    }

    #[task(binds = DMA1_STR1, shared = [dma], priority = 2)]
    fn write_interrupt(mut ctx: write_interrupt::Context) {
        (ctx.shared.dma).lock(|dma| {
            let is = dma.transfer_is_complete(DmaChannel::C1);
            rprintln!("transfer is complete? : {}", is);

            dma::clear_interrupt(DmaPeriph::Dma1, DmaChannel::C1, DmaInterrupt::TransferComplete);
        });
        rprintln!("transmit complete");
    }

    #[task(binds = DMA1_STR2, shared = [spi2, nss], priority = 2)]
    fn read_interrupt(mut ctx: read_interrupt::Context) {
        dma::clear_interrupt(DmaPeriph::Dma1, DmaChannel::C2, DmaInterrupt::TransferComplete);

        (ctx.shared.spi2).lock(|spi| {
            spi.stop_dma(DmaChannel::C2, Some(DmaChannel::C1), DmaPeriph::Dma1);
        });

        ctx.shared.nss.lock(|nss| {
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
}
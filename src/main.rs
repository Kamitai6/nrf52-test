#![no_std]
#![no_main]

mod resources;
mod parameter;

use resources::{shared, local};

use rtic::app;

use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

use cortex_m::{asm, interrupt::free, interrupt::{Mutex}};

use h7lib::*;
use periph::{pwr, rcc, gpio, adc, spi, timer, dma, ethernet};
use plugin::{pwm, ethernet_phy};


#[app(device = pac, peripherals = true)]
mod app {
    use super::*;

    #[shared]
    struct SharedResources {
        resource: shared::Shared,
    }
    #[local]
    struct LocalResources {
        #[link_section = ".sram3"]
        spi2_read_buf: [u8; 8],
        #[link_section = ".sram3"]
        spi2_write_buf: [u8; 8],
        #[link_section = ".sram3"]
        spi3_read_buf: [u8; 8],
        #[link_section = ".sram3"]
        spi3_write_buf: [u8; 8],
    }

    #[init]
    fn init(mut ctx: init::Context) -> (SharedResources, LocalResources) {
        rtt_init_print!();
        
        let pwr_config = pwr::PwrConfig {
            ..Default::default()
        };
        let pwr = pwr::Power::init(pwr_config);

        let rcc_config = rcc::Config {
            sys_ck: Some(200.MHz()),
            rcc_hclk: Some(200.MHz()),
            pll1: rcc::PllConfig {
                q_ck: Some(100.MHz()),
                ..Default::default()
            },
            ..Default::default()
        };
        let clock = rcc::Rcc::init(pwr, rcc_config);

        let resource = Shared::new(&clock);

        task1::initialize();
        task2::initialize();

        (
            SharedResources {
                resource
            },
            LocalResources {
            },
        )
    }

    // #[task(binds = DMA, local = [], shared=[])]
    // fn gpio_interrupt_handler(mut ctx: gpio_interrupt_handler::Context) {
        // unsafe {
        //     spi2.transfer_dma(
        //         &SPI_WRITE_BUF,
        //         &mut SPI_READ_BUF,
        //         dma::DmaChannel::C1,
        //         dma::DmaChannel::C2,
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
        //         &mut dma,
        //     );
        // }
    // }
}
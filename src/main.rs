#![no_std]
#![no_main]

mod resources;
use resources::{instance, parameter};

use rtic::app;

use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

use cortex_m::{asm, interrupt::free, interrupt::{Mutex}};

use h7lib::*;
use periph::{pwr, rcc, gpio, adc, spi, timer, dma, ethernet};
use plugin::{pwm, ethernet_phy};

fn main() -> ! {
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
        pll2: rcc::PllConfig {
            p_ck: Some(4.MHz()),
            ..Default::default()
        },
        ..Default::default()
    };
    let clock = rcc::Rcc::init(pwr, rcc_config);

    let ins = instance::Instance::new(&clock);

    loop {
        
    }
}

// #[interrupt]
// fn TIM1() {
//     unsafe {
//         spi2.transfer_dma(
//             &SPI_WRITE_BUF,
//             &mut SPI_READ_BUF,
//             dma::DmaChannel::C1,
//             dma::DmaChannel::C2,
//             dma::ChannelCfg {
//                 priority: dma::Priority::Medium,
//                 circular: dma::Circular::Disabled,
//                 periph_incr: dma::IncrMode::Disabled,
//                 mem_incr: dma::IncrMode::Enabled,
//             },
//             dma::ChannelCfg {
//                 priority: dma::Priority::Medium,
//                 circular: dma::Circular::Disabled,
//                 periph_incr: dma::IncrMode::Disabled,
//                 mem_incr: dma::IncrMode::Enabled,
//             },
//             &mut dma,
//         );
//     }
// }
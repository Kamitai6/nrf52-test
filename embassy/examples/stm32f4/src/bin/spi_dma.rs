#![no_std]
#![no_main]

use core::fmt::Write;
use core::str::from_utf8;

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::spi::{Config, Spi};
use embassy_stm32::time::Hertz;
use heapless::String;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("Hello World!");

    let mut spi_config = Config::default();
    spi_config.frequency = Hertz(1_000_000);

    let mut spi = Spi::new(p.SPI1, p.PB3, p.PB5, p.PB4, p.DMA2_CH3, p.DMA2_CH2, spi_config);

    for n in 0u32.. {
        let mut write: String<128> = String::new();
        let mut read = [0; 128];
        core::write!(&mut write, "Hello DMA World {}!\r\n", n).unwrap();
        spi.transfer(&mut read[0..write.len()], write.as_bytes()).await.ok();
        info!("read via spi+dma: {}", from_utf8(&read).unwrap());
    }
}

// これを最初に呼び出しているみたい
// fn init_hw(config: Config) -> Peripherals {
//     critical_section::with(|cs| {
//         let p = Peripherals::take_with_cs(cs);

//         #[cfg(dbgmcu)]
//         crate::pac::DBGMCU.cr().modify(|cr| {
//             #[cfg(dbgmcu_h5)]
//             {
//                 cr.set_stop(config.enable_debug_during_sleep);
//                 cr.set_standby(config.enable_debug_during_sleep);
//             }
//             #[cfg(any(dbgmcu_f0, dbgmcu_c0, dbgmcu_g0, dbgmcu_u0, dbgmcu_u5, dbgmcu_wba, dbgmcu_l5))]
//             {
//                 cr.set_dbg_stop(config.enable_debug_during_sleep);
//                 cr.set_dbg_standby(config.enable_debug_during_sleep);
//             }
//             #[cfg(any(
//                 dbgmcu_f1, dbgmcu_f2, dbgmcu_f3, dbgmcu_f4, dbgmcu_f7, dbgmcu_g4, dbgmcu_f7, dbgmcu_l0, dbgmcu_l1,
//                 dbgmcu_l4, dbgmcu_wb, dbgmcu_wl
//             ))]
//             {
//                 cr.set_dbg_sleep(config.enable_debug_during_sleep);
//                 cr.set_dbg_stop(config.enable_debug_during_sleep);
//                 cr.set_dbg_standby(config.enable_debug_during_sleep);
//             }
//             #[cfg(dbgmcu_h7)]
//             {
//                 cr.set_d1dbgcken(config.enable_debug_during_sleep);
//                 cr.set_d3dbgcken(config.enable_debug_during_sleep);
//                 cr.set_dbgsleep_d1(config.enable_debug_during_sleep);
//                 cr.set_dbgstby_d1(config.enable_debug_during_sleep);
//                 cr.set_dbgstop_d1(config.enable_debug_during_sleep);
//             }
//         });

//         #[cfg(not(any(stm32f1, stm32wb, stm32wl)))]
//         rcc::enable_and_reset_with_cs::<peripherals::SYSCFG>(cs);
//         #[cfg(not(any(stm32h5, stm32h7, stm32h7rs, stm32wb, stm32wl)))]
//         rcc::enable_and_reset_with_cs::<peripherals::PWR>(cs);
//         #[cfg(not(any(stm32f2, stm32f4, stm32f7, stm32l0, stm32h5, stm32h7, stm32h7rs)))]
//         rcc::enable_and_reset_with_cs::<peripherals::FLASH>(cs);

//         // Enable the VDDIO2 power supply on chips that have it.
//         // Note that this requires the PWR peripheral to be enabled first.
//         #[cfg(any(stm32l4, stm32l5))]
//         {
//             crate::pac::PWR.cr2().modify(|w| {
//                 // The official documentation states that we should ideally enable VDDIO2
//                 // through the PVME2 bit, but it looks like this isn't required,
//                 // and CubeMX itself skips this step.
//                 w.set_iosv(config.enable_independent_io_supply);
//             });
//         }
//         #[cfg(stm32u5)]
//         {
//             crate::pac::PWR.svmcr().modify(|w| {
//                 w.set_io2sv(config.enable_independent_io_supply);
//             });
//         }

//         // dead battery functionality is still present on these
//         // chips despite them not having UCPD- disable it
//         #[cfg(any(stm32g070, stm32g0b0))]
//         {
//             crate::pac::SYSCFG.cfgr1().modify(|w| {
//                 w.set_ucpd1_strobe(true);
//                 w.set_ucpd2_strobe(true);
//             });
//         }

//         unsafe {
//             #[cfg(ucpd)]
//             ucpd::init(
//                 cs,
//                 #[cfg(peri_ucpd1)]
//                 config.enable_ucpd1_dead_battery,
//                 #[cfg(peri_ucpd2)]
//                 config.enable_ucpd2_dead_battery,
//             );

//             #[cfg(feature = "_split-pins-enabled")]
//             crate::pac::SYSCFG.pmcr().modify(|pmcr| {
//                 #[cfg(feature = "split-pa0")]
//                 pmcr.set_pa0so(true);
//                 #[cfg(feature = "split-pa1")]
//                 pmcr.set_pa1so(true);
//                 #[cfg(feature = "split-pc2")]
//                 pmcr.set_pc2so(true);
//                 #[cfg(feature = "split-pc3")]
//                 pmcr.set_pc3so(true);
//             });

//             gpio::init(cs);
//             dma::init(
//                 cs,
//                 #[cfg(bdma)]
//                 config.bdma_interrupt_priority,
//                 #[cfg(dma)]
//                 config.dma_interrupt_priority,
//                 #[cfg(gpdma)]
//                 config.gpdma_interrupt_priority,
//             );
//             #[cfg(feature = "exti")]
//             exti::init(cs);

//             rcc::init(config.rcc);

//             // must be after rcc init
//             #[cfg(feature = "_time-driver")]
//             time_driver::init(cs);

//             #[cfg(feature = "low-power")]
//             {
//                 crate::rcc::REFCOUNT_STOP2 = 0;
//                 crate::rcc::REFCOUNT_STOP1 = 0;
//             }
//         }

//         p
//     })
// }

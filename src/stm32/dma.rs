// use stm32h5::stm32h562;

// use core::{
//     ops::Deref,
//     sync::atomic::{self, Ordering},
// };

// pub struct DMA<'a> {
//     dp: &'a stm32h562::Peripherals,
// }

// impl<'a> DMA<'a> {
//     pub fn new(dp: &stm32h562::Peripherals) -> DMA {
//         DMA { dp }
//     }

//     pub fn dma1_init(&self) {
//         self.dp.RCC.ahb1enr().modify(|_, w| w.gpdma1en().set_bit());

//         self.dp.GPDMA1.c0cr().modify(|_, w| w.en().clear_bit());
//         while self.dp.GPDMA1.c0cr().read().en().bit_is_set() {}

//         self.dp
//             .GPDMA1
//             .c0tr1()
//             .write(|w| unsafe { w.bits(periph_addr) });

//         // atomic::compiler_fence(Ordering::SeqCst);

//         self.dp
//             .GPDMA1
//             .c0tr2()
//             .modify(|_, w| unsafe { w.bits(mem_addr) });

//         self.dp
//             .GPDMA1
//             .c0br1()
//             .write(|w| unsafe { w.bits(num_data) });

//         self.dp
//             .GPDMA1
//             .c0llr()
//             .write(|w| unsafe { w.bits(num_data) });

//         __HAL_LINKDMA(&SpiHandle, hdmatx, LcdDmaHandle);

//         HAL_DMA_ConfigChannelAttributes
//     }
//     //     /// Stop a DMA transfer, if in progress.
//     //     pub fn stop(&mut self, channel: DmaChannel) {
//     //         let cr = &regs.st[channel as usize].cr;
//     //         cr.modify(|_, w| w.en().clear_bit());
//     //         while cr.read().en().bit_is_set() {}
//     //         match periph {
//     //             DmaPeriph::Dma1 => {
//     //                 let mut regs = unsafe { &(*DMA1::ptr()) };
//     //                 stop_internal(&mut regs, channel);
//     //             }
//     //             #[cfg(not(any(feature = "f3x4", feature = "g0", feature = "wb")))]
//     //             DmaPeriph::Dma2 => {
//     //                 let mut regs = unsafe { &(*pac::DMA2::ptr()) };
//     //                 stop_internal(&mut regs, channel);
//     //             }
//     //         }
//     //     }

//     //     /// Clear an interrupt flag.
//     //     pub fn clear_interrupt(&mut self, channel: DmaChannel, interrupt: DmaInterrupt) {
//     //         match periph {
//     //             DmaPeriph::Dma1 => {
//     //                 let mut regs = unsafe { &(*DMA1::ptr()) };
//     //                 clear_interrupt_internal(&mut regs, channel, interrupt);
//     //             }
//     //             DmaPeriph::Dma2 => {
//     //                 let mut regs = unsafe { &(*pac::DMA2::ptr()) };
//     //                 clear_interrupt_internal(&mut regs, channel, interrupt);
//     //             }
//     //         }
//     //         match channel {
//     //             DmaChannel::C0 => match interrupt {
//     //                 DmaInterrupt::TransferError => regs.lifcr.write(|w| w.cteif0().set_bit()),
//     //                 DmaInterrupt::HalfTransfer => regs.lifcr.write(|w| w.chtif0().set_bit()),
//     //                 DmaInterrupt::TransferComplete => regs.lifcr.write(|w| w.ctcif0().set_bit()),
//     //                 DmaInterrupt::DirectModeError => regs.lifcr.write(|w| w.cdmeif0().set_bit()),
//     //                 DmaInterrupt::FifoError => regs.lifcr.write(|w| w.cfeif0().set_bit()),
//     //             },
//     //             DmaChannel::C1 => match interrupt {
//     //                 DmaInterrupt::TransferError => regs.lifcr.write(|w| w.cteif1().set_bit()),
//     //                 DmaInterrupt::HalfTransfer => regs.lifcr.write(|w| w.chtif1().set_bit()),
//     //                 DmaInterrupt::TransferComplete => regs.lifcr.write(|w| w.ctcif1().set_bit()),
//     //                 DmaInterrupt::DirectModeError => regs.lifcr.write(|w| w.cdmeif1().set_bit()),
//     //                 DmaInterrupt::FifoError => regs.lifcr.write(|w| w.cfeif1().set_bit()),
//     //             },
//     //             DmaChannel::C2 => match interrupt {
//     //                 DmaInterrupt::TransferError => regs.lifcr.write(|w| w.cteif2().set_bit()),
//     //                 DmaInterrupt::HalfTransfer => regs.lifcr.write(|w| w.chtif2().set_bit()),
//     //                 DmaInterrupt::TransferComplete => regs.lifcr.write(|w| w.ctcif2().set_bit()),
//     //                 DmaInterrupt::DirectModeError => regs.lifcr.write(|w| w.cdmeif2().set_bit()),
//     //                 DmaInterrupt::FifoError => regs.lifcr.write(|w| w.cfeif2().set_bit()),
//     //             },
//     //             DmaChannel::C3 => match interrupt {
//     //                 DmaInterrupt::TransferError => regs.lifcr.write(|w| w.cteif3().set_bit()),
//     //                 DmaInterrupt::HalfTransfer => regs.lifcr.write(|w| w.chtif3().set_bit()),
//     //                 DmaInterrupt::TransferComplete => regs.lifcr.write(|w| w.ctcif3().set_bit()),
//     //                 DmaInterrupt::DirectModeError => regs.lifcr.write(|w| w.cdmeif3().set_bit()),
//     //                 DmaInterrupt::FifoError => regs.lifcr.write(|w| w.cfeif3().set_bit()),
//     //             },
//     //             DmaChannel::C4 => match interrupt {
//     //                 DmaInterrupt::TransferError => regs.hifcr.write(|w| w.cteif4().set_bit()),
//     //                 DmaInterrupt::HalfTransfer => regs.hifcr.write(|w| w.chtif4().set_bit()),
//     //                 DmaInterrupt::TransferComplete => regs.hifcr.write(|w| w.ctcif4().set_bit()),
//     //                 DmaInterrupt::DirectModeError => regs.hifcr.write(|w| w.cdmeif4().set_bit()),
//     //                 DmaInterrupt::FifoError => regs.hifcr.write(|w| w.cfeif4().set_bit()),
//     //             },
//     //             DmaChannel::C5 => match interrupt {
//     //                 DmaInterrupt::TransferError => regs.hifcr.write(|w| w.cteif5().set_bit()),
//     //                 DmaInterrupt::HalfTransfer => regs.hifcr.write(|w| w.chtif5().set_bit()),
//     //                 DmaInterrupt::TransferComplete => regs.hifcr.write(|w| w.ctcif5().set_bit()),
//     //                 DmaInterrupt::DirectModeError => regs.hifcr.write(|w| w.cdmeif5().set_bit()),
//     //                 DmaInterrupt::FifoError => regs.hifcr.write(|w| w.cfeif5().set_bit()),
//     //             },
//     //             DmaChannel::C6 => match interrupt {
//     //                 DmaInterrupt::TransferError => regs.hifcr.write(|w| w.cteif6().set_bit()),
//     //                 DmaInterrupt::HalfTransfer => regs.hifcr.write(|w| w.chtif6().set_bit()),
//     //                 DmaInterrupt::TransferComplete => regs.hifcr.write(|w| w.ctcif6().set_bit()),
//     //                 DmaInterrupt::DirectModeError => regs.hifcr.write(|w| w.cdmeif6().set_bit()),
//     //                 DmaInterrupt::FifoError => regs.hifcr.write(|w| w.cfeif6().set_bit()),
//     //             },
//     //             DmaChannel::C7 => match interrupt {
//     //                 DmaInterrupt::TransferError => regs.hifcr.write(|w| w.cteif7().set_bit()),
//     //                 DmaInterrupt::HalfTransfer => regs.hifcr.write(|w| w.chtif7().set_bit()),
//     //                 DmaInterrupt::TransferComplete => regs.hifcr.write(|w| w.ctcif7().set_bit()),
//     //                 DmaInterrupt::DirectModeError => regs.hifcr.write(|w| w.cdmeif7().set_bit()),
//     //                 DmaInterrupt::FifoError => regs.hifcr.write(|w| w.cfeif7().set_bit()),
//     //             },
//     //         }
//     //     }

//     //     pub fn transfer_is_complete(&mut self, channel: DmaChannel) -> bool {
//     //         match channel {
//     //             DmaChannel::C0 => self.regs.lisr.read().tcif0().bit_is_set(),
//     //             DmaChannel::C1 => self.regs.lisr.read().tcif1().bit_is_set(),
//     //             DmaChannel::C2 => self.regs.lisr.read().tcif2().bit_is_set(),
//     //             DmaChannel::C3 => self.regs.lisr.read().tcif3().bit_is_set(),
//     //             DmaChannel::C4 => self.regs.hisr.read().tcif4().bit_is_set(),
//     //             DmaChannel::C5 => self.regs.hisr.read().tcif5().bit_is_set(),
//     //             DmaChannel::C6 => self.regs.hisr.read().tcif6().bit_is_set(),
//     //             DmaChannel::C7 => self.regs.hisr.read().tcif7().bit_is_set(),
//     //         }
//     //     }

//     //     /// Enable a specific type of interrupt.
//     //     pub fn enable_interrupt(&mut self, channel: DmaChannel, interrupt: DmaInterrupt) {
//     //         // Can only be set when the channel is disabled.
//     //         let cr = &regs.st[channel as usize].cr;

//     //         match interrupt {
//     //             DmaInterrupt::TransferError => cr.modify(|_, w| w.teie().set_bit()),
//     //             DmaInterrupt::HalfTransfer => cr.modify(|_, w| w.htie().set_bit()),
//     //             DmaInterrupt::TransferComplete => cr.modify(|_, w| w.tcie().set_bit()),
//     //             DmaInterrupt::DirectModeError => cr.modify(|_, w| w.dmeie().set_bit()),
//     //             DmaInterrupt::FifoError => regs.st[channel as usize]
//     //                 .fcr
//     //                 .modify(|_, w| w.feie().set_bit()),
//     //         }

//     //         match periph {
//     //             DmaPeriph::Dma1 => {
//     //                 let mut regs = unsafe { &(*DMA1::ptr()) };
//     //                 enable_interrupt_internal(&mut regs, channel, interrupt);
//     //             }
//     //             DmaPeriph::Dma2 => {
//     //                 let mut regs = unsafe { &(*pac::DMA2::ptr()) };
//     //                 enable_interrupt_internal(&mut regs, channel, interrupt);
//     //             }
//     //         }
//     //     }

//     //     /// Disable a specific type of interrupt.
//     //     /// todo: Non-H7 version too!
//     //     pub fn disable_interrupt(&mut self, channel: DmaChannel, interrupt: DmaInterrupt) {
//     //         // Can only be set when the channel is disabled.
//     //         // todo: Is this true for disabling interrupts true, re the channel must be disabled?
//     //         let cr = &self.regs.st[channel as usize].cr;

//     //         let originally_enabled = cr.read().en().bit_is_set();

//     //         if originally_enabled {
//     //             cr.modify(|_, w| w.en().clear_bit());
//     //             while cr.read().en().bit_is_set() {}
//     //         }

//     //         match interrupt {
//     //             DmaInterrupt::TransferError => cr.modify(|_, w| w.teie().clear_bit()),
//     //             DmaInterrupt::HalfTransfer => cr.modify(|_, w| w.htie().clear_bit()),
//     //             DmaInterrupt::TransferComplete => cr.modify(|_, w| w.tcie().clear_bit()),
//     //             DmaInterrupt::DirectModeError => cr.modify(|_, w| w.dmeie().clear_bit()),
//     //             DmaInterrupt::FifoError => self.regs.st[channel as usize]
//     //                 .fcr
//     //                 .modify(|_, w| w.feie().clear_bit()),
//     //         }

//     //         if originally_enabled {
//     //             cr.modify(|_, w| w.en().set_bit());
//     //             while cr.read().en().bit_is_clear() {}
//     //         }

//     //         // Can only be set when the channel is disabled.
//     //         let cr = &regs.st[channel as usize].cr;

//     //         // todo DRY

//     //         match interrupt {
//     //             DmaInterrupt::TransferError => cr.modify(|_, w| w.teie().clear_bit()),
//     //             DmaInterrupt::HalfTransfer => cr.modify(|_, w| w.htie().clear_bit()),
//     //             DmaInterrupt::TransferComplete => cr.modify(|_, w| w.tcie().clear_bit()),
//     //             DmaInterrupt::DirectModeError => cr.modify(|_, w| w.dmeie().clear_bit()),
//     //             DmaInterrupt::FifoError => regs.st[channel as usize]
//     //                 .fcr
//     //                 .modify(|_, w| w.feie().clear_bit()),
//     //         }
//     //         match periph {
//     //             DmaPeriph::Dma1 => {
//     //                 let mut regs = unsafe { &(*DMA1::ptr()) };
//     //                 disable_interrupt_internal(&mut regs, channel, interrupt);
//     //             }
//     //             DmaPeriph::Dma2 => {
//     //                 let mut regs = unsafe { &(*pac::DMA2::ptr()) };
//     //                 disable_interrupt_internal(&mut regs, channel, interrupt);
//     //             }
//     //         }
//     //     }

//     //     /// Configure a specific DMA channel to work with a specific peripheral.
//     //     pub fn mux1(periph: DmaPeriph, channel: DmaChannel, input: DmaInput) {
//     //         unsafe {
//     //             let mux = unsafe { &(*DMAMUX::ptr()) };

//     //             match periph {
//     //                 DmaPeriph::Dma1 => {
//     //                     mux.ccr[channel as usize].modify(|_, w| w.dmareq_id().bits(input as u8));
//     //                 }
//     //                 DmaPeriph::Dma2 => {
//     //                     mux.ccr[channel as usize + 8].modify(|_, w| w.dmareq_id().bits(input as u8));
//     //                 }
//     //             }
//     //         }
//     //     }

//     //     /// Configure a specific DMA channel to work with a specific peripheral, on DMAMUX2.
//     //     pub fn mux2(periph: DmaPeriph, channel: DmaChannel, input: DmaInput2, mux: &mut DMAMUX2) {
//     //         mux.ccr[channel as usize].modify(|_, w| unsafe { w.dmareq_id().bits(input as u8) });
//     //     }
// }

// #[derive(Copy, Clone)]
// #[repr(usize)]
// pub enum DmaInput1 {
//     Adc1 = 9,
//     Adc2 = 10,
//     Tim1Ch1 = 11,
//     Tim1Ch2 = 12,
//     Tim1Ch3 = 13,
//     Tim1Ch4 = 14,
//     Tim1Up = 15,
//     Tim1Trig = 16,
//     Tim1Com = 17,
//     Tim2Ch1 = 18,
//     Tim2Ch2 = 19,
//     Tim2Ch3 = 20,
//     Tim2Ch4 = 21,
//     Tim2Up = 22,
//     Tim3Ch1 = 23,
//     Tim3Ch2 = 24,
//     Tim3Ch3 = 25,
//     Tim3Ch4 = 26,
//     Tim3Up = 27,
//     Tim3Trig = 28,
//     Tim4Ch1 = 29,
//     Tim4Ch2 = 30,
//     Tim4Ch3 = 31,
//     Tim4Up = 32,
//     I2c1Rx = 33,
//     I2c1Tx = 34,
//     I2c2Rx = 35,
//     I2c2Tx = 36,
//     Spi1Rx = 37,
//     Spi1Tx = 38,
//     Spi2Rx = 39,
//     Spi2Tx = 40,
//     Usart1Rx = 41,
//     Usart1Tx = 42,
//     Usart2Rx = 43,
//     Usart2Tx = 44,
//     Usart3Rx = 45,
//     Usart3Tx = 46,
//     Tim5Ch1 = 55,
//     Tim5Ch2 = 56,
//     Tim5Ch3 = 57,
//     Tim5Ch4 = 58,
//     Tim5Up = 59,
//     Tim5Trig = 60,
//     Spi3Rx = 61,
//     Spi3Tx = 62,
//     Uart4Rx = 63,
//     Uart4Tx = 64,
//     Uart5Rx = 65,
//     Uart5Tx = 66,
//     DacCh1 = 67,
//     DacCh2 = 68,
//     Tim6Up = 69,
//     Tim7Up = 70,
//     Uart6Rx = 71,
//     Uart6Tx = 72,
//     I2c3Rx = 73,
//     I2c3Tx = 74,
//     Dcmi = 75,
//     CrypIn = 76,
//     CrypOut = 77,
//     HashIn = 78,
//     Uart7Rx = 79,
//     Uart7Tx = 80,
//     Uart8Rx = 81,
//     Uart8Tx = 82,
//     Sai1A = 87,
//     Sai1B = 88,
//     Sai2A = 89,
//     Sai2B = 90,
//     Dfsdm1F0 = 101,
//     Dfsdm1F1 = 102,
//     Dfsdm1F2 = 103,
//     Dfsdm1F3 = 104,
//     Sai3A = 113,
//     Sai3B = 114,
//     Adc3 = 115,
//     Uart9Rx = 116,
//     Uart9Tx = 117,
//     Uart10Rx = 118,
//     Uart10Tx = 119,
// }

// #[derive(Copy, Clone)]
// #[repr(usize)]
// pub enum DmaInput2 {
//     Lpuart1Rx = 9,
//     Lpuart1Tx = 10,
//     Spi6Rx = 11,
//     Spi6Tx = 12,
//     I2c4Rx = 13,
//     I3crTx = 14,
//     Sai4A = 15,
//     Sai4B = 16,
// }

// #[derive(Copy, Clone)]
// #[repr(u8)]
// /// L4 RM, 11.4.3, "DMA arbitration":
// /// The priorities are managed in two stages:
// /// • software: priority of each channel is configured in the DMA_CCRx register, to one of
// /// the four different levels:
// /// – very high
// /// – high
// /// – medium
// /// – low
// /// • hardware: if two requests have the same software priority level, the channel with the
// /// lowest index gets priority. For example, channel 2 gets priority over channel 4.
// /// Only write to this when the channel is disabled.
// pub enum Priority {
//     Low = 0b00,
//     Medium = 0b01,
//     High = 0b10,
//     VeryHigh = 0b11,
// }

// #[derive(Copy, Clone)]
// #[repr(u8)]
// /// Represents a DMA channel to select, eg when configuring for use with a peripheral.
// /// u8 representation is used to index registers on H7 PAC (And hopefully on future PACs if they
// /// adopt H7's approach)
// pub enum DmaChannel {
//     C0 = 0,
//     C1 = 1,
//     C2 = 2,
//     C3 = 3,
//     C4 = 4,
//     C5 = 5,
//     C6 = 6,
//     C7 = 7,
//     C8 = 8,
// }

// #[derive(Copy, Clone)]
// #[repr(u8)]
// /// Set in CCR.
// /// Can only be set when channel is disabled.
// pub enum Direction {
//     ReadFromPeriph = 0,
//     ReadFromMem = 1,
//     MemToMem = 2,
// }

// #[derive(Copy, Clone, PartialEq)]
// #[repr(u8)]
// /// Set in CCR.
// /// Can only be set when channel is disabled.
// pub enum Circular {
//     Disabled = 0,
//     Enabled = 1,
// }

// #[derive(Copy, Clone)]
// #[repr(u8)]
// /// Peripheral and memory increment mode. (CCR PINC and MINC bits)
// /// Can only be set when channel is disabled.
// pub enum IncrMode {
//     // Can only be set when channel is disabled.
//     Disabled = 0,
//     Enabled = 1,
// }

// #[derive(Copy, Clone)]
// #[repr(u8)]
// /// Peripheral and memory increment mode. (CCR PSIZE and MSIZE bits)
// /// Can only be set when channel is disabled.
// pub enum DataSize {
//     S8 = 0b00, // ie 8 bits
//     S16 = 0b01,
//     S32 = 0b10,
// }

// #[derive(Copy, Clone)]
// /// Interrupt type. Set in CCR using TEIE, HTIE, and TCIE bits.
// /// Can only be set when channel is disabled.
// pub enum DmaInterrupt {
//     TransferError,
//     HalfTransfer,
//     TransferComplete,
//     DirectModeError,
//     FifoError,
// }

// /// This struct is used to pass common (non-peripheral and non-use-specific) data when configuring
// /// a channel.
// #[derive(Clone)]
// pub struct ChannelCfg {
//     /// Channel priority compared to other channels; can be low, medium, high, or very high. Defaults
//     /// to medium.
//     pub priority: Priority,
//     /// Enable or disable circular DMA. If enabled, the transfer continues after reaching the end of
//     /// the buffer, looping to the beginning. A TC interrupt first each time the end is reached, if
//     /// set. Defaults to disabled.
//     pub circular: Circular,
//     /// Whether we increment the peripheral address on data word transfer; generally (and by default)
//     /// disabled.
//     pub periph_incr: IncrMode,
//     /// Whether we increment the buffer address on data word transfer; generally (and by default)
//     /// enabled.
//     pub mem_incr: IncrMode,
// }

// impl Default for ChannelCfg {
//     fn default() -> Self {
//         Self {
//             priority: Priority::Medium,
//             circular: Circular::Disabled,
//             // Increment the buffer address, not the peripheral address.
//             periph_incr: IncrMode::Disabled,
//             mem_incr: IncrMode::Enabled,
//         }
//     }
// }

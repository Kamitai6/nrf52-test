//! Support for the Direct Memory Access (DMA) peripheral. This module handles initialization, and transfer
//! configuration for DMA. The `Dma::cfg_channel` method is called by modules that use DMA.

// todo: This module could be greatly simplified if [this issue](https://github.com/stm32-rs/stm32-rs/issues/610)
// todo is addressed: Ie H7 PAC approach adopted by other modules.

// todo: Use this clip or something similar to end terminate while loops, as in other modules.
// let mut i = 0;
// while asdf {
//     i += 1;
//     if i >= MAX_ITERS {
//         return Err(Error::Hardware);
//     }
// }

use core::{
    ops::Deref,
    sync::atomic::{self, Ordering},
};

use crate::{
    pac::{self, RCC},
    // util::rcc_en_reset,
    // MAX_ITERS,
};

use crate::pac::{dma1, dma2, DMA1, DMA2};
use pac::DMAMUX1 as DMAMUX;
use pac::DMAMUX2;

// todo: Several sections of this are only correct for DMA1.

#[derive(Clone, Copy)]
pub enum DmaPeriph {
    Dma1,
    Dma2,
}

// todo: Trigger, synchronization etc mappings. Perhaps DmaTrigger, DmaSync enums etc.

#[derive(Copy, Clone)]
#[repr(usize)]
/// A list of DMA input sources. The integer values represent their DMAMUX register value, on
/// MCUs that use this. H743 RM, Table 121: DMAMUX1: Assignment of multiplexer inputs to resources.
/// (Table 118 in RM0468)
/// Note that this is only for DMAMUX1
pub enum DmaInput {
    Adc1 = 9,
    Adc2 = 10,
    Tim1Ch1 = 11,
    Tim1Ch2 = 12,
    Tim1Ch3 = 13,
    Tim1Ch4 = 14,
    Tim1Up = 15,
    Tim1Trig = 16,
    Tim1Com = 17,
    Tim2Ch1 = 18,
    Tim2Ch2 = 19,
    Tim2Ch3 = 20,
    Tim2Ch4 = 21,
    Tim2Up = 22,
    Tim3Ch1 = 23,
    Tim3Ch2 = 24,
    Tim3Ch3 = 25,
    Tim3Ch4 = 26,
    Tim3Up = 27,
    Tim3Trig = 28,
    Tim4Ch1 = 29,
    Tim4Ch2 = 30,
    Tim4Ch3 = 31,
    Tim4Up = 32,
    I2c1Rx = 33,
    I2c1Tx = 34,
    I2c2Rx = 35,
    I2c2Tx = 36,
    Spi1Rx = 37,
    Spi1Tx = 38,
    Spi2Rx = 39,
    Spi2Tx = 40,
    Usart1Rx = 41,
    Usart1Tx = 42,
    Usart2Rx = 43,
    Usart2Tx = 44,
    Usart3Rx = 45,
    Usart3Tx = 46,
    Tim8Ch1 = 47,
    Tim8Ch2 = 48,
    Tim8Ch3 = 49,
    Tim8Ch4 = 50,
    Tim8Up = 51,
    Tim8Trig = 52,
    Tim8Com = 53,
    Tim5Ch1 = 55,
    Tim5Ch2 = 56,
    Tim5Ch3 = 57,
    Tim5Ch4 = 58,
    Tim5Up = 59,
    Tim5Trig = 60,
    Spi3Rx = 61,
    Spi3Tx = 62,
    Uart4Rx = 63,
    Uart4Tx = 64,
    Uart5Rx = 65,
    Uart5Tx = 66,
    DacCh1 = 67,
    DacCh2 = 68,
    Tim6Up = 69,
    Tim7Up = 70,
    Uart6Rx = 71,
    Uart6Tx = 72,
    I2c3Rx = 73,
    I2c3Tx = 74,
    Dcmi = 75,
    CrypIn = 76,
    CrypOut = 77,
    HashIn = 78,
    Uart7Rx = 79,
    Uart7Tx = 80,
    Uart8Rx = 81,
    Uart8Tx = 82,
    Sai1A = 87,
    Sai1B = 88,
    Sai2A = 89,
    Sai2B = 90,
    Dfsdm1F0 = 101,
    Dfsdm1F1 = 102,
    Dfsdm1F2 = 103,
    Dfsdm1F3 = 104,
    Sai3A = 113,
    Sai3B = 114,
    Adc3 = 115,
    Uart9Rx = 116,
    Uart9Tx = 117,
    Uart10Rx = 118,
    Uart10Tx = 119,
}

#[derive(Copy, Clone)]
#[repr(usize)]
/// A list of DMA input sources for DMAMUX2. Used for BDMA. See H742 RM, Table 124.
pub enum DmaInput2 {
    Lpuart1Rx = 9,
    Lpuart1Tx = 10,
    Spi6Rx = 11,
    Spi6Tx = 12,
    I2c4Rx = 13,
    I3crTx = 14,
    Sai4A = 15,
    Sai4B = 16,
}

#[derive(Copy, Clone)]
#[repr(u8)]
/// L4 RM, 11.4.3, "DMA arbitration":
/// The priorities are managed in two stages:
/// • software: priority of each channel is configured in the DMA_CCRx register, to one of
/// the four different levels:
/// – very high
/// – high
/// – medium
/// – low
/// • hardware: if two requests have the same software priority level, the channel with the
/// lowest index gets priority. For example, channel 2 gets priority over channel 4.
/// Only write to this when the channel is disabled.
pub enum Priority {
    Low = 0b00,
    Medium = 0b01,
    High = 0b10,
    VeryHigh = 0b11,
}

#[derive(Copy, Clone)]
#[repr(u8)]
/// Represents a DMA channel to select, eg when configuring for use with a peripheral.
/// u8 representation is used to index registers on H7 PAC (And hopefully on future PACs if they
/// adopt H7's approach)
pub enum DmaChannel {
    // H7 calls these Streams. We use the `Channel` name for consistency.
    C0 = 0,
    C1 = 1,
    C2 = 2,
    C3 = 3,
    C4 = 4,
    C5 = 5,
    C6 = 6,
    C7 = 7,
}

#[derive(Copy, Clone)]
#[repr(u8)]
/// Set in CCR.
/// Can only be set when channel is disabled.
pub enum Direction {
    /// DIR = 0 defines typically a peripheral-to-memory transfer
    ReadFromPeriph = 0,
    /// DIR = 1 defines typically a memory-to-peripheral transfer.
    ReadFromMem = 1,
    MemToMem = 2,
}

#[derive(Copy, Clone, PartialEq)]
#[repr(u8)]
/// Set in CCR.
/// Can only be set when channel is disabled.
pub enum Circular {
    Disabled = 0,
    Enabled = 1,
}

#[derive(Copy, Clone)]
#[repr(u8)]
/// Peripheral and memory increment mode. (CCR PINC and MINC bits)
/// Can only be set when channel is disabled.
pub enum IncrMode {
    // Can only be set when channel is disabled.
    Disabled = 0,
    Enabled = 1,
}

#[derive(Copy, Clone)]
#[repr(u8)]
/// Peripheral and memory increment mode. (CCR PSIZE and MSIZE bits)
/// Can only be set when channel is disabled.
pub enum DataSize {
    S8 = 0b00, // ie 8 bits
    S16 = 0b01,
    S32 = 0b10,
}

#[derive(Copy, Clone)]
/// Interrupt type. Set in CCR using TEIE, HTIE, and TCIE bits.
/// Can only be set when channel is disabled.
pub enum DmaInterrupt {
    TransferError,
    HalfTransfer,
    TransferComplete,
    DirectModeError,
    FifoError,
}

/// This struct is used to pass common (non-peripheral and non-use-specific) data when configuring
/// a channel.
#[derive(Clone)]
pub struct ChannelCfg {
    /// Channel priority compared to other channels; can be low, medium, high, or very high. Defaults
    /// to medium.
    pub priority: Priority,
    /// Enable or disable circular DMA. If enabled, the transfer continues after reaching the end of
    /// the buffer, looping to the beginning. A TC interrupt first each time the end is reached, if
    /// set. Defaults to disabled.
    pub circular: Circular,
    /// Whether we increment the peripheral address on data word transfer; generally (and by default)
    /// disabled.
    pub periph_incr: IncrMode,
    /// Whether we increment the buffer address on data word transfer; generally (and by default)
    /// enabled.
    pub mem_incr: IncrMode,
}

impl Default for ChannelCfg {
    fn default() -> Self {
        Self {
            priority: Priority::Medium,
            circular: Circular::Disabled,
            // Increment the buffer address, not the peripheral address.
            periph_incr: IncrMode::Disabled,
            mem_incr: IncrMode::Enabled,
        }
    }
}

/// Represents a Direct Memory Access (DMA) peripheral.
pub struct Dma<D> {
    pub regs: D,
}

impl<D> Dma<D>
where
    D: Deref<Target = dma1::RegisterBlock>,
{
    /// Initialize a DMA peripheral, including enabling and resetting
    /// its RCC peripheral clock.
    pub fn new(regs: D) -> Self {
        // todo: Enable RCC for DMA 2 etc!
        let rcc = unsafe { &(*RCC::ptr()) };
        // rcc_en_reset!(ahb1, dma1, rcc);

        Self { regs }
    }

    /// Configure a DMA channel. See L4 RM 0394, section 11.4.4. Sets the Transfer Complete
    /// interrupt. Note that this fn has been (perhaps) depreciated by the standalone fn.
    pub fn cfg_channel(
        &mut self,
        channel: DmaChannel,
        periph_addr: u32,
        mem_addr: u32,
        num_data: u32,
        direction: Direction,
        periph_size: DataSize,
        mem_size: DataSize,
        cfg: ChannelCfg,
    ) {
        let regs = &mut self.regs;
        // todo: The H7 sections are different, but we consolidated the comments. Figure out
        // todo what's different and fix it by following the steps

        regs.st[channel as usize]
            .cr
            .modify(|_, w| w.en().clear_bit());
        while regs.st[channel as usize].cr.read().en().bit_is_set() {}

        // H743 RM Section 15.3.19 The following sequence is needed to configure a DMA stream x:
        // 1. Set the peripheral register address in the DMA_CPARx register.
        // The data is moved from/to this address to/from the memory after the peripheral event,
        // or after the channel is enabled in memory-to-memory mode.
        regs.st[channel as usize]
            .par
            .write(|w| unsafe { w.bits(periph_addr) });

        atomic::compiler_fence(Ordering::SeqCst);

        // 2. Set the memory address in the DMA_CMARx register.
        // The data is written to/read from the memory after the peripheral event or after the
        // channel is enabled in memory-to-memory mode.
        regs.st[channel as usize]
            .m0ar
            .write(|w| unsafe { w.bits(mem_addr) });

        // todo: m1ar too, if in double-buffer mode.

        // 3. Configure the total number of data to transfer in the DMA_CNDTRx register.
        // After each data transfer, this value is decremented.
        regs.st[channel as usize]
            .ndtr
            .write(|w| unsafe { w.bits(num_data) });

        // 4. Configure the parameters listed below in the DMA_CCRx register:
        // (These are listed below by their corresponding reg write code)

        // todo: See note about sep reg writes to disable channel, and when you need to do this.

        // 5. Activate the channel by setting the EN bit in the DMA_CCRx register.
        // A channel, as soon as enabled, may serve any DMA request from the peripheral connected
        // to this channel, or may start a memory-to-memory block transfer.
        // Note: The two last steps of the channel configuration procedure may be merged into a single
        // access to the DMA_CCRx register, to configure and enable the channel.
        // When a channel is enabled and still active (not completed), the software must perform two
        // separate write accesses to the DMA_CCRx register, to disable the channel, then to
        // reprogram the channel for another next block transfer.
        // Some fields of the DMA_CCRx register are read-only when the EN bit is set to 1

        // (later): The circular mode must not be used in memory-to-memory mode. Before enabling a
        // channel in circular mode (CIRC = 1), the software must clear the MEM2MEM bit of the
        // DMA_CCRx register. When the circular mode is activated, the amount of data to transfer is
        // automatically reloaded with the initial value programmed during the channel configuration
        // phase, and the DMA requests continue to be served

        // (See remainder of steps in `set_ccr()!` macro.

        // todo: Let user set mem2mem mode?

        // See the [Embedonomicon section on DMA](https://docs.rust-embedded.org/embedonomicon/dma.html)
        // for info on why we use `compiler_fence` here:
        // "We use Ordering::Release to prevent all preceding memory operations from being moved
        // after [starting DMA], which performs a volatile write."

        let cr = &regs.st[channel as usize].cr;

        let originally_enabled = cr.read().en().bit_is_set();
        if originally_enabled {
            cr.modify(|_, w| w.en().clear_bit());
            while cr.read().en().bit_is_set() {}
        }

        cr.modify(|_, w| unsafe {
            // – the channel priority
            w.pl().bits(cfg.priority as u8);
            // – the data transfer direction
            // This bit [DIR] must be set only in memory-to-peripheral and peripheral-to-memory modes.
            // 0: read from peripheral
            w.dir().bits(direction as u8);
            // – the circular mode
            w.circ().bit(cfg.circular as u8 != 0);
            // – the peripheral and memory incremented mode
            w.pinc().bit(cfg.periph_incr as u8 != 0);
            w.minc().bit(cfg.mem_incr as u8 != 0);
            // – the peripheral and memory data size
            w.psize().bits(periph_size as u8);
            w.msize().bits(mem_size as u8);
            // – the interrupt enable at half and/or full transfer and/or transfer error
            w.tcie().set_bit();
            // (See `Step 5` above.)
            w.en().set_bit()
        });

        if originally_enabled {
            cr.modify(|_, w| w.en().set_bit());
            while cr.read().en().bit_is_clear() {}
        }
    }
}

/// Stop a DMA transfer, if in progress.
fn stop_internal<D>(regs: &mut D, channel: DmaChannel)
where
    D: Deref<Target = dma1::RegisterBlock>,
{
    // L4 RM:
    // Once the software activates a channel, it waits for the completion of the programmed
    // transfer. The DMA controller is not able to resume an aborted active channel with a possible
    // suspended bus transfer.
    // To correctly stop and disable a channel, the software clears the EN bit of the DMA_CCRx
    // register.

    // The software secures that no pending request from the peripheral is served by the
    // DMA controller before the transfer completion.
    // todo?

    let cr = &regs.st[channel as usize].cr;
    cr.modify(|_, w| w.en().clear_bit());
    while cr.read().en().bit_is_set() {}

    // The software waits for the transfer complete or transfer error interrupt.
    // (Handed by calling code)

    // (todo: set ifcr.cficx bit to clear all interrupts?)

    // When a channel transfer error occurs, the EN bit of the DMA_CCRx register is cleared by
    // hardware. This EN bit can not be set again by software to re-activate the channel x, until the
    // TEIFx bit of the DMA_ISR register is set
}

/// Stop a DMA transfer, if in progress.
pub fn stop(periph: DmaPeriph, channel: DmaChannel) {
    match periph {
        DmaPeriph::Dma1 => {
            let mut regs = unsafe { &(*DMA1::ptr()) };
            stop_internal(&mut regs, channel);
        }
        DmaPeriph::Dma2 => {
            let mut regs = unsafe { &(*pac::DMA2::ptr()) };
            stop_internal(&mut regs, channel);
        }
    }
}

fn clear_interrupt_internal<D>(regs: &mut D, channel: DmaChannel, interrupt: DmaInterrupt)
where
    D: Deref<Target = dma1::RegisterBlock>,
{
    match channel {
        DmaChannel::C0 => match interrupt {
            DmaInterrupt::TransferError => regs.lifcr.write(|w| w.cteif0().set_bit()),
            DmaInterrupt::HalfTransfer => regs.lifcr.write(|w| w.chtif0().set_bit()),
            DmaInterrupt::TransferComplete => regs.lifcr.write(|w| w.ctcif0().set_bit()),
            DmaInterrupt::DirectModeError => regs.lifcr.write(|w| w.cdmeif0().set_bit()),
            DmaInterrupt::FifoError => regs.lifcr.write(|w| w.cfeif0().set_bit()),
        }
        DmaChannel::C1 => match interrupt {
            DmaInterrupt::TransferError => regs.lifcr.write(|w| w.cteif1().set_bit()),
            DmaInterrupt::HalfTransfer => regs.lifcr.write(|w| w.chtif1().set_bit()),
            DmaInterrupt::TransferComplete => regs.lifcr.write(|w| w.ctcif1().set_bit()),
            DmaInterrupt::DirectModeError => regs.lifcr.write(|w| w.cdmeif1().set_bit()),
            DmaInterrupt::FifoError => regs.lifcr.write(|w| w.cfeif1().set_bit()),
        }
        DmaChannel::C2 => match interrupt {
            DmaInterrupt::TransferError => regs.lifcr.write(|w| w.cteif2().set_bit()),
            DmaInterrupt::HalfTransfer => regs.lifcr.write(|w| w.chtif2().set_bit()),
            DmaInterrupt::TransferComplete => regs.lifcr.write(|w| w.ctcif2().set_bit()),
            DmaInterrupt::DirectModeError => regs.lifcr.write(|w| w.cdmeif2().set_bit()),
            DmaInterrupt::FifoError => regs.lifcr.write(|w| w.cfeif2().set_bit()),
        }
        DmaChannel::C3 => match interrupt {
            DmaInterrupt::TransferError => regs.lifcr.write(|w| w.cteif3().set_bit()),
            DmaInterrupt::HalfTransfer => regs.lifcr.write(|w| w.chtif3().set_bit()),
            DmaInterrupt::TransferComplete => regs.lifcr.write(|w| w.ctcif3().set_bit()),
            DmaInterrupt::DirectModeError => regs.lifcr.write(|w| w.cdmeif3().set_bit()),
            DmaInterrupt::FifoError => regs.lifcr.write(|w| w.cfeif3().set_bit()),
        }
        DmaChannel::C4 => match interrupt {
            DmaInterrupt::TransferError => regs.hifcr.write(|w| w.cteif4().set_bit()),
            DmaInterrupt::HalfTransfer => regs.hifcr.write(|w| w.chtif4().set_bit()),
            DmaInterrupt::TransferComplete => regs.hifcr.write(|w| w.ctcif4().set_bit()),
            DmaInterrupt::DirectModeError => regs.hifcr.write(|w| w.cdmeif4().set_bit()),
            DmaInterrupt::FifoError => regs.hifcr.write(|w| w.cfeif4().set_bit()),
        }
        DmaChannel::C5 => match interrupt {
            DmaInterrupt::TransferError => regs.hifcr.write(|w| w.cteif5().set_bit()),
            DmaInterrupt::HalfTransfer => regs.hifcr.write(|w| w.chtif5().set_bit()),
            DmaInterrupt::TransferComplete => regs.hifcr.write(|w| w.ctcif5().set_bit()),
            DmaInterrupt::DirectModeError => regs.hifcr.write(|w| w.cdmeif5().set_bit()),
            DmaInterrupt::FifoError => regs.hifcr.write(|w| w.cfeif5().set_bit()),
        }
        DmaChannel::C6 => match interrupt {
            DmaInterrupt::TransferError => regs.hifcr.write(|w| w.cteif6().set_bit()),
            DmaInterrupt::HalfTransfer => regs.hifcr.write(|w| w.chtif6().set_bit()),
            DmaInterrupt::TransferComplete => regs.hifcr.write(|w| w.ctcif6().set_bit()),
            DmaInterrupt::DirectModeError => regs.hifcr.write(|w| w.cdmeif6().set_bit()),
            DmaInterrupt::FifoError => regs.hifcr.write(|w| w.cfeif6().set_bit()),
        }
        DmaChannel::C7 => match interrupt {
            DmaInterrupt::TransferError => regs.hifcr.write(|w| w.cteif7().set_bit()),
            DmaInterrupt::HalfTransfer => regs.hifcr.write(|w| w.chtif7().set_bit()),
            DmaInterrupt::TransferComplete => regs.hifcr.write(|w| w.ctcif7().set_bit()),
            DmaInterrupt::DirectModeError => regs.hifcr.write(|w| w.cdmeif7().set_bit()),
            DmaInterrupt::FifoError => regs.hifcr.write(|w| w.cfeif7().set_bit()),
        }
    }
}

fn enable_interrupt_internal<D>(regs: &mut D, channel: DmaChannel, interrupt: DmaInterrupt)
where
    D: Deref<Target = dma1::RegisterBlock>,
{
    // Can only be set when the channel is disabled.
    let cr = &regs.st[channel as usize].cr;

    match interrupt {
        DmaInterrupt::TransferError => cr.modify(|_, w| w.teie().set_bit()),
        DmaInterrupt::HalfTransfer => cr.modify(|_, w| w.htie().set_bit()),
        DmaInterrupt::TransferComplete => cr.modify(|_, w| w.tcie().set_bit()),
        DmaInterrupt::DirectModeError => cr.modify(|_, w| w.dmeie().set_bit()),
        DmaInterrupt::FifoError => regs.st[channel as usize]
            .fcr
            .modify(|_, w| w.feie().set_bit()),
    }
}

fn disable_interrupt_internal<D>(regs: &mut D, channel: DmaChannel, interrupt: DmaInterrupt)
where
    D: Deref<Target = dma1::RegisterBlock>,
{
    // Can only be set when the channel is disabled.
    let cr = &regs.st[channel as usize].cr;

    // todo DRY

    match interrupt {
        DmaInterrupt::TransferError => cr.modify(|_, w| w.teie().clear_bit()),
        DmaInterrupt::HalfTransfer => cr.modify(|_, w| w.htie().clear_bit()),
        DmaInterrupt::TransferComplete => cr.modify(|_, w| w.tcie().clear_bit()),
        DmaInterrupt::DirectModeError => cr.modify(|_, w| w.dmeie().clear_bit()),
        DmaInterrupt::FifoError => regs.st[channel as usize]
            .fcr
            .modify(|_, w| w.feie().clear_bit()),
    }
}

/// Enable a specific type of interrupt.
pub fn enable_interrupt(periph: DmaPeriph, channel: DmaChannel, interrupt: DmaInterrupt) {
    match periph {
        DmaPeriph::Dma1 => {
            let mut regs = unsafe { &(*DMA1::ptr()) };
            enable_interrupt_internal(&mut regs, channel, interrupt);
        }
        DmaPeriph::Dma2 => {
            let mut regs = unsafe { &(*pac::DMA2::ptr()) };
            enable_interrupt_internal(&mut regs, channel, interrupt);
        }
    }
}

/// Disable a specific type of interrupt.
pub fn disable_interrupt(periph: DmaPeriph, channel: DmaChannel, interrupt: DmaInterrupt) {
    match periph {
        DmaPeriph::Dma1 => {
            let mut regs = unsafe { &(*DMA1::ptr()) };
            disable_interrupt_internal(&mut regs, channel, interrupt);
        }
        DmaPeriph::Dma2 => {
            let mut regs = unsafe { &(*pac::DMA2::ptr()) };
            disable_interrupt_internal(&mut regs, channel, interrupt);
        }
    }
}

/// Clear an interrupt flag.
pub fn clear_interrupt(periph: DmaPeriph, channel: DmaChannel, interrupt: DmaInterrupt) {
    match periph {
        DmaPeriph::Dma1 => {
            let mut regs = unsafe { &(*DMA1::ptr()) };
            clear_interrupt_internal(&mut regs, channel, interrupt);
        }
        DmaPeriph::Dma2 => {
            let mut regs = unsafe { &(*pac::DMA2::ptr()) };
            clear_interrupt_internal(&mut regs, channel, interrupt);
        }
    }
}

/// Configure a specific DMA channel to work with a specific peripheral.
pub fn mux(periph: DmaPeriph, channel: DmaChannel, input: DmaInput) {
    // Note: This is similar in API and purpose to `channel_select` above,
    // for different families. We're keeping it as a separate function instead
    // of feature-gating within the same function so the name can be recognizable
    // from the RM etc.

    // G4 example:
    // "The mapping of resources to DMAMUX is hardwired.
    // DMAMUX is used with DMA1 and DMA2:
    // For category 3 and category 4 devices:
    // •
    // DMAMUX channels 0 to 7 are connected to DMA1 channels 1 to 8
    // •
    // DMAMUX channels 8 to 15 are connected to DMA2 channels 1 to 8
    // For category 2 devices:
    // •
    // DMAMUX channels 0 to 5 are connected to DMA1 channels 1 to 6
    // •
    // DMAMUX channels 6 to 11 are connected to DMA2 channels 1 to 6"
    //
    // H723/25/33/35"
    // DMAMUX1 is used with DMA1 and DMA2 in D2 domain
    // •
    // DMAMUX1 channels 0 to 7 are connected to DMA1 channels 0 to 7
    // •
    // DMAMUX1 channels 8 to 15 are connected to DMA2 channels 0 to 7
    // (Note: The H7 and G4 cat 3/4 mappings are the same, except for H7's use of 0-7, and G4's use of 1-8.)

    // todo: With this in mind, some of the mappings below are not correct on some G4 variants.

    unsafe {
        let mux = unsafe { &(*DMAMUX::ptr()) };

        match periph {
            DmaPeriph::Dma1 => {
                mux.ccr[channel as usize].modify(|_, w| w.dmareq_id().bits(input as u8));
            }
            DmaPeriph::Dma2 => {
                mux.ccr[channel as usize + 8].modify(|_, w| w.dmareq_id().bits(input as u8));
            }
        }
    }
}

/// Configure a specific DMA channel to work with a specific peripheral, on DMAMUX2.
pub fn mux2(periph: DmaPeriph, channel: DmaChannel, input: DmaInput2, mux: &mut DMAMUX2) {
    mux.ccr[channel as usize].modify(|_, w| unsafe { w.dmareq_id().bits(input as u8) });
}
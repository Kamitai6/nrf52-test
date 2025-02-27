#![no_std]
#![allow(unused_unsafe)]

// Used for while loops, to allow returning an error instead of hanging.
pub(crate) const MAX_ITERS: u32 = 300_000; // todo: What should this be?

#[cfg(not(any(
    feature = "h735",
    feature = "h743",
    feature = "h743v",
    feature = "h747cm4",
    feature = "h747cm7",
    feature = "h753",
    feature = "h753v",
    feature = "h7b3",
)))]
compile_error!("This crate requires an MCU-specifying feature to be enabled. eg `l552`.");

// Re-export of the [svd2rust](https://crates.io/crates/svd2rust) auto-generated API for
// stm32 peripherals.

// H7 PAC
#[cfg(feature = "h735")]
pub use stm32h7::stm32h735 as pac;
#[cfg(feature = "h743")]
pub use stm32h7::stm32h743 as pac;
#[cfg(feature = "h743v")]
pub use stm32h7::stm32h743v as pac;
#[cfg(feature = "h747cm4")]
pub use stm32h7::stm32h747cm4 as pac;
#[cfg(feature = "h747cm7")]
pub use stm32h7::stm32h747cm7 as pac;
#[cfg(feature = "h753")]
pub use stm32h7::stm32h753 as pac;
#[cfg(feature = "h753v")]
pub use stm32h7::stm32h753v as pac;
#[cfg(feature = "h7b3")]
pub use stm32h7::stm32h7b3 as pac;

pub use fugit::{
    HertzU32 as Hertz, KilohertzU32 as KiloHertz, MegahertzU32 as MegaHertz,
    MicrosDurationU32 as MicroSeconds, MillisDurationU32 as MilliSeconds,
    NanosDurationU32 as NanoSeconds,
};

/// Bits per second
pub type Bps = Hertz;

/// Extension trait that adds convenience methods to the `u32` type
pub trait U32Ext {
    /// Wrap in `Bps`
    fn bps(self) -> Bps;
}

impl U32Ext for u32 {
    fn bps(self) -> Bps {
        Bps::from_raw(self)
    }
}


pub mod other;
pub mod periph;

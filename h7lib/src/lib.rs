#![no_std]
#![allow(unused_unsafe)]

extern crate paste;

#[cfg(not(any(
    feature = "h742",
    feature = "h743",
    feature = "h753",
    feature = "h750",
    feature = "h742v",
    feature = "h743v",
    feature = "h753v",
    feature = "h750v",
)))]
compile_error!("This crate requires an MCU-specifying feature to be enabled. eg `l552`.");

// Re-export of the [svd2rust](https://crates.io/crates/svd2rust) auto-generated API for
// stm32 peripherals.

// H7 PAC
#[cfg(feature = "h742")]
pub use stm32h7::stm32h742 as pac;
#[cfg(feature = "h743")]
pub use stm32h7::stm32h743 as pac;
#[cfg(feature = "h753")]
pub use stm32h7::stm32h753 as pac;
#[cfg(feature = "h750")]
pub use stm32h7::stm32h750 as pac;
#[cfg(feature = "h742v")]
pub use stm32h7::stm32h742v as pac;
#[cfg(feature = "h743v")]
pub use stm32h7::stm32h743v as pac;
#[cfg(feature = "h753v")]
pub use stm32h7::stm32h753v as pac;
#[cfg(feature = "h750v")]
pub use stm32h7::stm32h750v as pac;

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

//! This module provides functionality for General Purpose Input and Output (GPIO) pins,
//! including all GPIOx register functions. It also configures GPIO interrupts using SYSCFG and EXTI
//! registers as appropriate. It allows pin mode configuration, interrupts, and DMA.
//!
//! The primary API uses a `Pin` struct, with its methods. There are also standalone functions
//! available to set and read pin state, and clear interrupts, without access to a `Pin`.

use crate::pac::{self, EXTI, RCC};

use crate::pac::DMA1;

use cfg_if::cfg_if;
use paste::paste;

use crate::dma::{self, ChannelCfg, DmaChannel};

#[derive(Copy, Clone)]
#[repr(u8)]
/// Values for `GPIOx_MODER`. Sets pin to input, output, and other functionality.
pub enum PinMode {
    /// An input pin; read by firmware; set by something connected to the pin.
    Input,
    /// An output pin; set by firmware; read by something connected to the pin.
    Output,
    /// An alternate function, as defined in the MCU's user manual. Used for various
    /// onboard peripherals like buses, timers etc.
    Alt(u8),
    /// For use with the onboard ADC and DAC. Prevent parasitic power loss on the pin
    // if using it for one of these functionalities.
    Analog,
}

impl PinMode {
    /// We use this function to find the value bits due to being unable to repr(u8) with
    /// the wrapped `AltFn` value.
    fn val(&self) -> u8 {
        match self {
            Self::Input => 0b00,
            Self::Output => 0b01,
            Self::Alt(_) => 0b10,
            Self::Analog => 0b11,
        }
    }
}

#[derive(Copy, Clone)]
#[repr(u8)]
/// Values for `GPIOx_OTYPER`.
pub enum OutputType {
    PushPull = 0,
    OpenDrain = 1,
}

#[derive(Copy, Clone)]
#[repr(u8)]
/// Values for `GPIOx_OSPEEDR`. This configures I/O output speed. See the user manual
/// for your MCU for what speeds these are. Note that Fast speed (0b10) is not
/// available on all STM32 families.
pub enum OutputSpeed {
    Low = 0b00,
    Medium = 0b01,
    #[cfg(not(feature = "f3"))]
    High = 0b10, // Called "Fast" on some families.
    VeryHigh = 0b11, // Called "High" on some families.
}

#[derive(Copy, Clone)]
#[repr(u8)]
/// Values for `GPIOx_PUPDR`. Sets if the pin uses the internal pull-up or pull-down
// resistor.
pub enum Pull {
    Floating = 0b00,
    Up = 0b01,
    Dn = 0b10,
}

#[derive(Copy, Clone)]
#[repr(u8)]
/// Values for `GPIOx_IDR` and `GPIOx_ODR`.
pub enum PinState {
    High = 1,
    Low = 0,
}

#[derive(Copy, Clone)]
#[repr(u8)]
/// Values for `GPIOx_LCKR`.
pub enum CfgLock {
    NotLocked = 0,
    Locked = 1,
}

#[derive(Copy, Clone)]
#[repr(u8)]
/// Values for `GPIOx_BRR`.
pub enum ResetState {
    NoAction = 0,
    Reset = 1,
}

// todo: If you get rid of Port struct, rename this enum Port
#[derive(Copy, Clone, PartialEq)]
/// GPIO port letter
pub enum Port {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    #[cfg(any(feature = "h747cm4", feature = "h747cm7",))]
    I,
}

impl Port {
    /// See F303 RM section 12.1.3: each reg has an associated value
    fn cr_val(&self) -> u8 {
        match self {
            Self::A => 0,
            Self::B => 1,
            Self::C => 2,
            Self::D => 3,
            Self::E => 4,
            Self::F => 5,
            Self::G => 6,
            Self::H => 7,
            #[cfg(any(feature = "h747cm4", feature = "h747cm7",))]
            Self::I => 8,
        }
    }
}

#[derive(Copy, Clone, Debug)]
/// The pulse edge used to trigger interrupts. Either rising, falling, or either.
pub enum Edge {
    /// Interrupts trigger on rising pin edge.
    Rising,
    /// Interrupts trigger on falling pin edge.
    Falling,
    /// Interrupts trigger on either rising or falling pin edges.
    Either,
}

// These macros are used to interate over pin number, for use with PAC fields.
macro_rules! set_field {
    ($regs:expr, $pin:expr, $reg:ident,$field:ident, $bit:ident, $val:expr, [$($num:expr),+]) => {
        paste! {
            unsafe {
                match $pin {
                    $(
                        $num => (*$regs).$reg.modify(|_, w| w.[<$field $num>]().$bit($val)),
                    )+
                    _ => panic!("GPIO pins must be 0 - 15."),
                }
            }
        }
    }
}

macro_rules! set_alt {
    ($regs: expr, $pin:expr, $field_af:ident, $val:expr, [$(($num:expr, $lh:ident)),+]) => {
        paste! {
            unsafe {
                match $pin {
                    $(
                        $num => {
                            (*$regs).moder.modify(|_, w| w.[<moder $num>]().bits(PinMode::Alt(0).val()));
                            (*$regs).[<afr $lh>].modify(|_, w| w.[<$field_af $num>]().bits($val));
                            (*$regs).[<afr $lh>].modify(|_, w| w.[<$field_af $lh $num>]().bits($val));
                        }
                    )+
                    _ => panic!("GPIO pins must be 0 - 15."),
                }
            }
        }
    }
}

macro_rules! get_input_data {
    ($regs: expr, $pin:expr, [$($num:expr),+]) => {
        paste! {
            unsafe {
                match $pin {
                    $(
                        $num => (*$regs).idr.read().[<idr $num>]().bit_is_set(),
                    )+
                    _ => panic!("GPIO pins must be 0 - 15."),
                }
            }
        }
    }
}

macro_rules! set_state {
    ($regs: expr, $pin:expr, $offset: expr, [$($num:expr),+]) => {
        paste! {
            unsafe {
                match $pin {
                    $(
                        $num => (*$regs).bsrr.write(|w| w.bits(1 << ($offset + $num))),
                    )+
                    _ => panic!("GPIO pins must be 0 - 15."),
                }
            }
        }
    }
}

// todo: Consolidate these exti macros

// Reduce DRY for setting up interrupts.
macro_rules! set_exti {
    ($pin:expr, $rising:expr, $falling:expr, $val:expr, [$(($num:expr, $crnum:expr)),+]) => {
        let exti = unsafe { &(*pac::EXTI::ptr()) };
        let syscfg  = unsafe { &(*pac::SYSCFG::ptr()) };

        paste! {
            match $pin {
                $(
                    $num => {
                    // todo: Core 2 interrupts for wb. (?)
                        cfg_if! {
                            if #[cfg(all(feature = "h7", not(any(feature = "h747cm4", feature = "h747cm7"))))] {
                                exti.cpuimr1.modify(|_, w| w.[<mr $num>]().set_bit());
                            } else if #[cfg(any(feature = "h747cm4", feature = "h747cm7"))] {
                                exti.c1imr1.modify(|_, w| w.[<mr $num>]().set_bit());
                            } else {
                                exti.imr1.modify(|_, w| w.[<mr $num>]().set_bit());
                            }
                        }

                        exti.rtsr1.modify(|_, w| w.[<tr $num>]().bit($rising));
                        exti.ftsr1.modify(|_, w| w.[<tr $num>]().bit($falling));

                        syscfg
                            .[<exticr $crnum>]
                            .modify(|_, w| unsafe { w.[<exti $num>]().bits($val) });
                    }
                )+
                _ => panic!("GPIO pins must be 0 - 15."),
            }
        }
    }
}

#[cfg(feature = "f4")]
// Similar to `set_exti`, but with reg names sans `1`.
macro_rules! set_exti_f4 {
    ($pin:expr, $rising:expr, $falling:expr, $val:expr, [$(($num:expr, $crnum:expr)),+]) => {
        let exti = unsafe { &(*pac::EXTI::ptr()) };
        let syscfg  = unsafe { &(*pac::SYSCFG::ptr()) };

        paste! {
            match $pin {
                $(
                    $num => {
                        exti.imr.modify(|_, w| w.[<mr $num>]().unmasked());
                        exti.rtsr.modify(|_, w| w.[<tr $num>]().bit($rising));
                        exti.ftsr.modify(|_, w| w.[<tr $num>]().bit($falling));
                        syscfg
                            .[<exticr $crnum>]
                            .modify(|_, w| unsafe { w.[<exti $num>]().bits($val) });
                    }
                )+
                _ => panic!("GPIO pins must be 0 - 15."),
            }
        }
    }
}

#[derive(Clone)]
/// Represents a single GPIO pin. Allows configuration, and reading/setting state.
pub struct Pin {
    /// The GPIO Port letter. Eg A, B, C.
    pub port: Port,
    /// The pin number: 1 - 15.
    pub pin: u8,
}

impl Pin {
    /// Internal function to get the appropriate GPIO block pointer.
    const fn regs(&self) -> *const pac::gpioa::RegisterBlock {
        // Note that we use this `const` fn and pointer casting since not all ports actually
        // deref to GPIOA in PAC.
        regs(self.port)
    }

    /// Create a new pin, with a specific mode. Enables the RCC peripheral clock to the port,
    /// if not already enabled. Example: `let pa1 = Pin::new(Port::A, 1, PinMode::Output);` Leaves settings
    /// other than mode and alternate function (if applicable) at their hardware defaults.
    pub fn new(port: Port, pin: u8, mode: PinMode) -> Self {
        assert!(pin <= 15, "Pin must be 0 - 15.");

        let rcc = unsafe { &(*RCC::ptr()) };

        match port {
            Port::A => {
                if rcc.ahb4enr.read().gpioaen().bit_is_clear() {
                    rcc.ahb4enr.modify(|_, w| w.gpioaen().set_bit());
                    rcc.ahb4rstr.modify(|_, w| w.gpioarst().set_bit());
                    rcc.ahb4rstr.modify(|_, w| w.gpioarst().clear_bit());
                }
            }
            Port::B => {
                if rcc.ahb4enr.read().gpioben().bit_is_clear() {
                    rcc.ahb4enr.modify(|_, w| w.gpioben().set_bit());
                    rcc.ahb4rstr.modify(|_, w| w.gpiobrst().set_bit());
                    rcc.ahb4rstr.modify(|_, w| w.gpiobrst().clear_bit());
                }
            }
            Port::C => {
                if rcc.ahb4enr.read().gpiocen().bit_is_clear() {
                    rcc.ahb4enr.modify(|_, w| w.gpiocen().set_bit());
                    rcc.ahb4rstr.modify(|_, w| w.gpiocrst().set_bit());
                    rcc.ahb4rstr.modify(|_, w| w.gpiocrst().clear_bit());
                }
            }
            Port::D => {
                if rcc.ahb4enr.read().gpioden().bit_is_clear() {
                    rcc.ahb4enr.modify(|_, w| w.gpioden().set_bit());
                    rcc.ahb4rstr.modify(|_, w| w.gpiodrst().set_bit());
                    rcc.ahb4rstr.modify(|_, w| w.gpiodrst().clear_bit());
                }
            }
            Port::E => {
                if rcc.ahb4enr.read().gpioeen().bit_is_clear() {
                    rcc.ahb4enr.modify(|_, w| w.gpioeen().set_bit());
                    rcc.ahb4rstr.modify(|_, w| w.gpioerst().set_bit());
                    rcc.ahb4rstr.modify(|_, w| w.gpioerst().clear_bit());
                }
            }
            Port::F => {
                if rcc.ahb4enr.read().gpiofen().bit_is_clear() {
                    rcc.ahb4enr.modify(|_, w| w.gpiofen().set_bit());
                    rcc.ahb4rstr.modify(|_, w| w.gpiofrst().set_bit());
                    rcc.ahb4rstr.modify(|_, w| w.gpiofrst().clear_bit());
                }
            }
            Port::G => {
                if rcc.ahb4enr.read().gpiogen().bit_is_clear() {
                    rcc.ahb4enr.modify(|_, w| w.gpiogen().set_bit());
                    rcc.ahb4rstr.modify(|_, w| w.gpiogrst().set_bit());
                    rcc.ahb4rstr.modify(|_, w| w.gpiogrst().clear_bit());
                }
            }
            Port::H => {
                if rcc.ahb4enr.read().gpiohen().bit_is_clear() {
                    rcc.ahb4enr.modify(|_, w| w.gpiohen().set_bit());
                    rcc.ahb4rstr.modify(|_, w| w.gpiohrst().set_bit());
                    rcc.ahb4rstr.modify(|_, w| w.gpiohrst().clear_bit());
                }
            }
            #[cfg(any(feature = "h747cm4", feature = "h747cm7"))]
            Port::I => {
                if rcc.ahb4enr.read().gpioien().bit_is_clear() {
                    rcc.ahb4enr.modify(|_, w| w.gpioien().set_bit());
                    rcc.ahb4rstr.modify(|_, w| w.gpioirst().set_bit());
                    rcc.ahb4rstr.modify(|_, w| w.gpioirst().clear_bit());
                }
            }
        }

        let mut result = Self { port, pin };
        result.mode(mode);

        result
    }

    /// Set pin mode. Eg, Output, Input, Analog, or Alt. Sets the `MODER` register.
    pub fn mode(&mut self, value: PinMode) {
        set_field!(
            self.regs(),
            self.pin,
            moder,
            moder,
            bits,
            value.val(),
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
        );

        if let PinMode::Alt(alt) = value {
            self.alt_fn(alt);
        }
    }

    /// Set output type. Sets the `OTYPER` register.
    pub fn output_type(&mut self, value: OutputType) {
        set_field!(
            self.regs(),
            self.pin,
            otyper,
            ot,
            bit,
            value as u8 != 0,
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
        );
    }

    /// Set output speed to Low, Medium, or High. Sets the `OSPEEDR` register.
    pub fn output_speed(&mut self, value: OutputSpeed) {
        set_field!(
            self.regs(),
            self.pin,
            ospeedr,
            ospeedr,
            bits,
            value as u8,
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
        );
    }

    /// Set internal pull resistor: Pull up, pull down, or floating. Sets the `PUPDR` register.
    pub fn pull(&mut self, value: Pull) {
        set_field!(
            self.regs(),
            self.pin,
            pupdr,
            pupdr,
            bits,
            value as u8,
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
        );
    }

    /// Lock or unlock a port configuration. Sets the `LCKR` register.
    pub fn cfg_lock(&mut self, value: CfgLock) {
        set_field!(
            self.regs(),
            self.pin,
            lckr,
            lck,
            bit,
            value as u8 != 0,
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
        );
    }

    /// Read the input data register. Eg determine if the pin is high or low. See also `is_high()`
    /// and `is_low()`. Reads from the `IDR` register.
    pub fn get_state(&mut self) -> PinState {
        let val = get_input_data!(
            self.regs(),
            self.pin,
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
        );
        if val {
            PinState::High
        } else {
            PinState::Low
        }
    }

    /// Set a pin state (ie set high or low output voltage level). See also `set_high()` and
    /// `set_low()`. Sets the `BSRR` register. Atomic.
    pub fn set_state(&mut self, value: PinState) {
        let offset = match value {
            PinState::Low => 16,
            PinState::High => 0,
        };

        set_state!(
            self.regs(),
            self.pin,
            offset,
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
        );
    }

    /// Set up a pin's alternate function. We set this up initially using `mode()`.
    fn alt_fn(&mut self, value: u8) {
        assert!(value <= 15, "Alt function must be 0 to 15.");

        set_alt!(self.regs(), self.pin, afr, value, [(0, l), (1, l), (2, l),
            (3, l), (4, l), (5, l), (6, l), (7, l), (8, h), (9, h), (10, h), (11, h), (12, h),
            (13, h), (14, h), (15, h)]
        )
    }

    /// Configure this pin as an interrupt source. Set the edge as Rising or Falling.
    pub fn enable_interrupt(&mut self, edge: Edge) {
        let rising = match edge {
            Edge::Falling => false,
            _ => true, // rising or either.
        };

        let falling = match edge {
            Edge::Rising => false,
            _ => true, // falling or either.
        };

        set_exti!(self.pin, rising, falling, self.port.cr_val(), [(0, 1), (1, 1), (2, 1),
            (3, 1), (4, 2), (5, 2), (6, 2), (7, 2), (8, 3), (9, 3), (10, 3), (11, 3), (12, 4),
            (13, 4), (14, 4), (15, 4)]
        );
    }

    /// Check if the pin's input voltage is high. Reads from the `IDR` register.
    pub fn is_high(&self) -> bool {
        get_input_data!(
            self.regs(),
            self.pin,
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
        )
    }

    /// Check if the pin's input voltage is low. Reads from the `IDR` register.
    pub fn is_low(&self) -> bool {
        !self.is_high()
    }

    /// Set the pin's output voltage to high. Sets the `BSRR` register. Atomic.
    pub fn set_high(&mut self) {
        self.set_state(PinState::High);
    }

    /// Set the pin's output voltage to low. Sets the `BSRR` register. Atomic.
    pub fn set_low(&mut self) {
        self.set_state(PinState::Low);
    }

    /// Toggle output voltage between low and high. Sets the `BSRR` register. Atomic.
    pub fn toggle(&mut self) {
        // if self.is_high() {
        if Pin::is_high(self) {
            Pin::set_low(self);
            // self.set_low();
        } else {
            // self.set_high();
            Pin::set_high(self);
        }
    }
}

/// Check if a pin's input voltage is high. Reads from the `IDR` register.
/// Does not require a `Pin` struct.
pub fn is_high(port: Port, pin: u8) -> bool {
    get_input_data!(
        regs(port),
        pin,
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
    )
}

/// Check if a pin's input voltage is low. Reads from the `IDR` register.
/// Does not require a `Pin` struct.
pub fn is_low(port: Port, pin: u8) -> bool {
    !is_high(port, pin)
}

/// Set a pin's output voltage to high. Sets the `BSRR` register. Atomic.
/// Does not require a `Pin` struct.
pub fn set_high(port: Port, pin: u8) {
    set_state(port, pin, PinState::High);
}

/// Set a pin's output voltage to low. Sets the `BSRR` register. Atomic.
/// Does not require a `Pin` struct.
pub fn set_low(port: Port, pin: u8) {
    set_state(port, pin, PinState::Low);
}

/// Set a pin state (ie set high or low output voltage level). See also `set_high()` and
/// `set_low()`. Sets the `BSRR` register. Atomic.
/// Does not require a `Pin` struct.
pub fn set_state(port: Port, pin: u8, value: PinState) {
    let offset = match value {
        PinState::Low => 16,
        PinState::High => 0,
    };

    set_state!(
        regs(port),
        pin,
        offset,
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
    );
}

/// Clear an EXTI interrupt, lines 0 - 15. Note that this function currently doesn't support
/// higher extis, but will work for all GPIO interrupts.
pub fn clear_exti_interrupt(line: u8) {
    // todo: Macro to avoid DRY?
    unsafe {
        cfg_if! {
            if #[cfg(any(feature = "h747cm4", feature = "h747cm7"))] {
                (*EXTI::ptr()).c1pr1.modify(|_, w| {
                    match line {
                        0 => w.pr0().set_bit(),
                        1 => w.pr1().set_bit(),
                        2 => w.pr2().set_bit(),
                        3 => w.pr3().set_bit(),
                        4 => w.pr4().set_bit(),
                        5 => w.pr5().set_bit(),
                        6 => w.pr6().set_bit(),
                        7 => w.pr7().set_bit(),
                        8 => w.pr8().set_bit(),
                        9 => w.pr9().set_bit(),
                        10 => w.pr10().set_bit(),
                        11 => w.pr11().set_bit(),
                        12 => w.pr12().set_bit(),
                        13 => w.pr13().set_bit(),
                        14 => w.pr14().set_bit(),
                        15 => w.pr15().set_bit(),
                        _ => panic!(),
                    }
                });
            } else if #[cfg(feature = "h7")] {
                (*EXTI::ptr()).cpupr1.modify(|_, w| {
                    match line {
                        0 => w.pr0().set_bit(),
                        1 => w.pr1().set_bit(),
                        2 => w.pr2().set_bit(),
                        3 => w.pr3().set_bit(),
                        4 => w.pr4().set_bit(),
                        5 => w.pr5().set_bit(),
                        6 => w.pr6().set_bit(),
                        7 => w.pr7().set_bit(),
                        8 => w.pr8().set_bit(),
                        9 => w.pr9().set_bit(),
                        10 => w.pr10().set_bit(),
                        11 => w.pr11().set_bit(),
                        12 => w.pr12().set_bit(),
                        13 => w.pr13().set_bit(),
                        14 => w.pr14().set_bit(),
                        15 => w.pr15().set_bit(),
                        _ => panic!(),
                    }
                });
            }
        }
    }
}

const fn regs(port: Port) -> *const pac::gpioa::RegisterBlock {
    // Note that we use this `const` fn and pointer casting since not all ports actually
    // deref to GPIOA in PAC.
    match port {
        Port::A => crate::pac::GPIOA::ptr(),
        Port::B => crate::pac::GPIOB::ptr() as _,
        Port::C => crate::pac::GPIOC::ptr() as _,
        Port::D => crate::pac::GPIOD::ptr() as _,
        Port::E => crate::pac::GPIOE::ptr() as _,
        Port::F => crate::pac::GPIOF::ptr() as _,
        Port::G => crate::pac::GPIOG::ptr() as _,
        Port::H => crate::pac::GPIOH::ptr() as _,
        #[cfg(any(feature = "h747cm4", feature = "h747cm7",))]
        Port::I => crate::pac::GPIOI::ptr() as _,
    }
}

/// Write a series of words to the BSRR (atomic output) register. Note that these are direct writes
/// to the full, 2-sided register - not a series of low/high values.
pub unsafe fn write_dma(
    buf: &[u32],
    port: Port,
    dma_channel: DmaChannel,
    channel_cfg: ChannelCfg,
    dma_periph: dma::DmaPeriph,
) {
    let (ptr, len) = (buf.as_ptr(), buf.len());

    let periph_addr = &(*(regs(port))).bsrr as *const _ as u32;

    let num_data = len as u32;

    match dma_periph {
        dma::DmaPeriph::Dma1 => {
            let mut regs = unsafe { &(*DMA1::ptr()) };
            dma::cfg_channel(
                &mut regs,
                dma_channel,
                periph_addr,
                ptr as u32,
                num_data,
                dma::Direction::ReadFromMem,
                dma::DataSize::S32,
                dma::DataSize::S32,
                channel_cfg,
            );
        }
        dma::DmaPeriph::Dma2 => {
            let mut regs = unsafe { &(*pac::DMA2::ptr()) };
            dma::cfg_channel(
                &mut regs,
                dma_channel,
                periph_addr,
                ptr as u32,
                num_data,
                dma::Direction::ReadFromMem,
                dma::DataSize::S32,
                dma::DataSize::S32,
                channel_cfg,
            );
        }
    }
}

/// Read a series of words from the IDR register.
pub unsafe fn read_dma(
    buf: &[u32],
    port: Port,
    dma_channel: DmaChannel,
    channel_cfg: ChannelCfg,
    dma_periph: dma::DmaPeriph,
) {
    let (ptr, len) = (buf.as_ptr(), buf.len());

    let periph_addr = &(*(regs(port))).idr as *const _ as u32;

    let num_data = len as u32;

    match dma_periph {
        dma::DmaPeriph::Dma1 => {
            let mut regs = unsafe { &(*DMA1::ptr()) };
            dma::cfg_channel(
                &mut regs,
                dma_channel,
                periph_addr,
                ptr as u32,
                num_data,
                dma::Direction::ReadFromPeriph,
                dma::DataSize::S32,
                dma::DataSize::S32,
                channel_cfg,
            );
        }
        dma::DmaPeriph::Dma2 => {
            let mut regs = unsafe { &(*pac::DMA2::ptr()) };
            dma::cfg_channel(
                &mut regs,
                dma_channel,
                periph_addr,
                ptr as u32,
                num_data,
                dma::Direction::ReadFromPeriph,
                dma::DataSize::S32,
                dma::DataSize::S32,
                channel_cfg,
            );
        }
    }
}

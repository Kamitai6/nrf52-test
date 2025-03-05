///! GPIO

use crate::{pac, rcc_en_reset};

#[derive(Clone, Copy)]
#[repr(u8)]
/// Values for `GPIOx_OTYPER`.
pub enum OutputType {
    PushPull  = 0,
    OpenDrain = 1,
}
pub type OT = OutputType;

#[derive(Clone, Copy)]
#[repr(u8)]
/// Values for `GPIOx_MODER`. Sets pin to input, output, and other functionality.
pub enum PinMode {
    Input,
    Output(OT),
    AltFn(u8, OT),
    Analog,
}
impl PinMode {
    /// We use this function to find the value bits due to being unable to repr(u8) with
    /// the wrapped `AltFn` value.
    fn val(&self) -> u8 {
        match self {
            Self::Input       => 0,
            Self::Output(_)   => 1,
            Self::AltFn(_, _) => 2,
            Self::Analog      => 3,
        }
    }
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Speed {
    Low      = 0, 
    Medium   = 1,
    High     = 2, // Called "Fast" on some families.
    VeryHigh = 3, // Called "High" on some families.
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Pull {
    Floating = 0,
    Up       = 1,
    Down     = 2,
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum PinState {
    Low  = 0,
    High = 1,
}

#[derive(Clone, Copy)]
#[repr(u8)]
/// The pulse edge used to trigger interrupts.
pub enum Edge {
    Rising  = 0,
    Falling = 1,
    Both  = 2,
}

#[derive(Clone)]
/// Represents a single GPIO pin. Allows configuration, and reading/setting state.
pub(super) struct GPIO<const PORT: char, const PIN: u8> {
    pub mode: PinMode,

    regs_ptr: *const pac::gpioa::RegisterBlock,
}

impl<const PORT: char, const PIN: u8> GPIO<PORT, PIN> {
    const CHECK: () = {
        assert!(PIN <= 15, "Pin must be 0 - 15.");
    };
    /// Create a new pin, with a specific mode. Enables the RCC peripheral clock to the port,
    /// if not already enabled. Example: `let pa1 = Pin::new(Port::A, 1, PinMode::Output);` Leaves settings
    /// other than mode and alternate function (if applicable) at their hardware defaults.
    pub fn init(mode: PinMode) -> Self {
        let _ = Self::CHECK;
        let regs_ptr = match PORT {
            'A' => crate::pac::GPIOA::ptr(),
            'B' => crate::pac::GPIOB::ptr(),
            'C' => crate::pac::GPIOC::ptr(),
            'D' => crate::pac::GPIOD::ptr(),
            'E' => crate::pac::GPIOE::ptr(),
            'F' => crate::pac::GPIOF::ptr(),
            'G' => crate::pac::GPIOG::ptr(),
            'H' => crate::pac::GPIOH::ptr(),
            _ => unreachable!(),
        };

        let regs = unsafe { &(*regs_ptr) };
        let rcc = unsafe { &(*pac::RCC::ptr()) };

        match PORT {
            'A' => rcc_en_reset!(ahb4, gpioa, rcc),
            'B' => rcc_en_reset!(ahb4, gpiob, rcc),
            'C' => rcc_en_reset!(ahb4, gpioc, rcc),
            'D' => rcc_en_reset!(ahb4, gpiod, rcc),
            'E' => rcc_en_reset!(ahb4, gpioe, rcc),
            'F' => rcc_en_reset!(ahb4, gpiof, rcc),
            'G' => rcc_en_reset!(ahb4, gpiog, rcc),
            'H' => rcc_en_reset!(ahb4, gpioh, rcc),
            _ => unreachable!(),
        };

        match mode {
            PinMode::Output(outtype) => {
                regs.otyper.modify(|r, w| unsafe {
                    w.bits(r.bits() & !(0b1 << PIN) | u32::from((outtype as u8) << PIN))
                });
            }
            PinMode::AltFn(af, outtype) => {
                regs.otyper.modify(|r, w| unsafe {
                    w.bits(r.bits() & !(0b1 << PIN) | u32::from((outtype as u8) << PIN))
                });

                if PIN < 8 {
                    let offset = 4 * PIN;
                    regs.afrl.modify(|r, w| unsafe {
                        w.bits(
                            (r.bits() & !(0b1111 << offset))
                                | u32::from(af << offset),
                        )
                    });
                } else {
                    let offset = 4 * (PIN - 8);
                    regs.afrh.modify(|r, w| unsafe {
                        w.bits(
                            (r.bits() & !(0b1111 << offset))
                                | u32::from(af << offset),
                        )
                    });
                }
            }
            _ => {}
        }

        let offset = 2 * PIN;
        regs.moder.modify(|r, w| unsafe {
            w.bits(
                (r.bits() & !(0b11 << offset)) | u32::from(mode.val() << offset),
            )
        });

        Self {
            mode,
            regs_ptr,
        }
    }

    #[inline(always)]
    /// Set output speed to Low, Medium, or High. Sets the `OSPEEDR` register.
    pub fn set_speed(&mut self, speed: Speed) {
        let regs = unsafe { &(*self.regs_ptr) };
        let offset = 2 * PIN;

        unsafe {
            regs.ospeedr.modify(|r, w| {
                w.bits(
                    (r.bits() & !(0b11 << offset)) | ((speed as u32) << offset),
                )
            });
        }
    }

    #[inline(always)]
    /// Set internal pull resistor: Pull up, pull down, or floating. Sets the `PUPDR` register.
    pub fn set_pull(&mut self, pull: Pull) {
        let regs = unsafe { &(*self.regs_ptr) };
        let offset = 2 * PIN;
        unsafe {
            regs.pupdr.modify(|r, w| {
                w.bits((r.bits() & !(0b11 << offset)) | u32::from((pull as u8) << offset))
            });
        }
    }

    #[inline(always)]
    /// Read the input data register. Eg determine if the pin is high or low. See also `is_high()`
    /// and `is_low()`. Reads from the `IDR` register.
    pub fn get_state(&mut self) -> PinState {
        if self.is_high() {
            PinState::High
        } else {
            PinState::Low
        }
    }

    #[inline(always)]
    /// Set a pin state (ie set high or low output voltage level). See also `set_high()` and
    /// `set_low()`. Sets the `BSRR` register. Atomic.
    pub fn set_state(&mut self, state: PinState) {
        let regs: &pac::gpioa::RegisterBlock = unsafe { &(*self.regs_ptr) };
        let offset = match state {
            PinState::Low => 16,
            PinState::High => 0,
        };

        regs.bsrr.write(|w| unsafe { w.bits(1 << (offset + PIN))});
    }

    #[inline(always)]
    /// Check if the pin's input voltage is high. Reads from the `IDR` register.
    pub fn is_high(&self) -> bool {
        let regs: &pac::gpioa::RegisterBlock = unsafe { &(*self.regs_ptr) };
        regs.idr.read().bits() & (1 << PIN) != 0
    }

    #[inline(always)]
    /// Check if the pin's input voltage is low. Reads from the `IDR` register.
    pub fn is_low(&self) -> bool {
        !self.is_high()
    }

    #[inline(always)]
    /// Set the pin's output voltage to high. Sets the `BSRR` register. Atomic.
    pub fn set_high(&mut self) {
        self.set_state(PinState::High);
    }

    #[inline(always)]
    /// Set the pin's output voltage to low. Sets the `BSRR` register. Atomic.
    pub fn set_low(&mut self) {
        self.set_state(PinState::Low);
    }

    #[inline(always)]
    /// Toggle output voltage between low and high. Sets the `BSRR` register. Atomic.
    pub fn toggle(&mut self) {
        if self.is_high() {
            self.set_low();
        } else {
            self.set_high();
        }
    }

    /// Configure this pin as an interrupt source. Set the edge as Rising or Falling.
    pub fn enable_interrupt(&mut self, edge: Edge) {
        // 安全にポインタを取得
        let exti = unsafe { &(*pac::EXTI::ptr()) };
        let syscfg = unsafe { &(*pac::SYSCFG::ptr()) };
    
        // ピンに対応するビットマスク
        let bitmask = 1 << PIN;
    
        // --- IMR の設定 ---
        // 特定のコア向けの割り込みマスクをビット演算でセットする
        exti.cpuimr1.modify(|r, w| unsafe { w.bits(r.bits() | bitmask) });

        // --- 立ち上がりトリガの設定 ---
        let (rise_enable, fall_enable) = match edge {
            Edge::Rising => (true, false),
            Edge::Falling => (false, true),
            Edge::Both => (true, true),
        };
        exti.rtsr1.modify(|r, w| {
            unsafe { w.bits((r.bits() & !bitmask) | ((rise_enable as u32) * bitmask)) }
        });
        exti.ftsr1.modify(|r, w| {
            unsafe { w.bits((r.bits() & !bitmask) | ((fall_enable as u32) * bitmask)) }
        });

        // --- SYSCFG の EXTI 設定 ---
        // EXTI の設定はピン番号で使用するレジスタが変わるので、
        // ピン番号 0-3: exticr1, 4-7: exticr2, 8-11: exticr3, 12-15: exticr4
        // また各レジスタ内は 4 ビットごとにピン設定があるので、シフト量は (pin % 4) * 4
        let offset = (PIN % 4) * 4;
        let port_num = u32::from(PORT) - u32::from('A');

        match PIN {
            0..=3 => {
                syscfg.exticr1.modify(|r, w| unsafe {
                    w.bits((r.bits() & !(0xf << offset)) | (port_num << offset))
                });
            }
            4..=7 => {
                syscfg.exticr2.modify(|r, w| unsafe {
                    w.bits((r.bits() & !(0xf << offset)) | (port_num << offset))
                });
            }
            8..=11 => {
                syscfg.exticr3.modify(|r, w| unsafe {
                    w.bits((r.bits() & !(0xf << offset)) | (port_num << offset))
                });
            }
            12..=15 => {
                syscfg.exticr4.modify(|r, w| unsafe {
                    w.bits((r.bits() & !(0xf << offset)) | (port_num << offset))
                });
            }
            _ => unreachable!(),
        }
    }
}

pub type PA<const PIN: u8> = GPIO<'A', PIN>;
pub type PB<const PIN: u8> = GPIO<'B', PIN>;
pub type PC<const PIN: u8> = GPIO<'C', PIN>;
pub type PD<const PIN: u8> = GPIO<'D', PIN>;
pub type PE<const PIN: u8> = GPIO<'E', PIN>;
pub type PF<const PIN: u8> = GPIO<'F', PIN>;
pub type PG<const PIN: u8> = GPIO<'G', PIN>;
pub type PH<const PIN: u8> = GPIO<'H', PIN>;
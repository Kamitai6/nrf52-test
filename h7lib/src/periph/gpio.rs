use crate::pac;

#[derive(Copy, Clone, PartialEq)]
/// GPIO port letter
pub enum Port {
    A = 0,
    B = 1,
    C = 2,
    D = 3,
    E = 4,
    F = 5,
    G = 6,
    H = 7,
    #[cfg(any(feature = "h747cm4", feature = "h747cm7",))]
    I = 8,
}

/// Values for `GPIOx_OTYPER`.
pub enum OutputType {
    PushPull  = 0,
    OpenDrain = 1,
}
pub type OT = OutputType;

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

pub enum Speed {
    Low      = 0, 
    Medium   = 1,
    High     = 2, // Called "Fast" on some families.
    VeryHigh = 3, // Called "High" on some families.
}

pub enum Pull {
    Floating = 0,
    Up       = 1,
    Down     = 2,
}

pub enum PinState {
    Low  = 0,
    High = 1,
}

/// The pulse edge used to trigger interrupts.
pub enum Edge {
    Rising  = 0,
    Falling = 1,
    Both  = 2,
}

#[derive(Clone)]
/// Represents a single GPIO pin. Allows configuration, and reading/setting state.
pub struct GPIO {
    /// The GPIO Port letter. Eg A, B, C.
    pub port: Port,
    /// The pin number: 1 - 15.
    pub pin: u8,

    pub mode: PinMode,

    regs_ptr: *const pac::gpioa::RegisterBlock,
}

impl GPIO {
    /// Create a new pin, with a specific mode. Enables the RCC peripheral clock to the port,
    /// if not already enabled. Example: `let pa1 = Pin::new(Port::A, 1, PinMode::Output);` Leaves settings
    /// other than mode and alternate function (if applicable) at their hardware defaults.
    pub fn new(port: Port, pin: u8, mode: PinMode) -> Self {
        assert!(pin <= 15, "Pin must be 0 - 15.");

        let regs_ptr = match port {
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
        };

        let regs = unsafe { &(*regs_ptr) };
        let rcc = unsafe { &(*pac::RCC::ptr()) };

        unsafe {
            rcc.ahb4enr.modify(|r, w| {
                w.bits(r.bits() | 1 << port)
            });

            rcc.ahb4rstr.modify(|r, w| {
                w.bits(r.bits() | 1 << port)
            });

            rcc.ahb4rstr.modify(|r, w| {
                w.bits(r.bits() & !(1 << port))
            });
        }

        match mode {
            PinMode::Output(outtype) => {
                regs.otyper.modify(|r, w| unsafe {
                    w.bits(r.bits() & !(0b1 << pin) | (outtype << pin))
                });
            }
            PinMode::AltFn(af, outtype) => {
                regs.otyper.modify(|r, w| unsafe {
                    w.bits(r.bits() & !(0b1 << pin) | (outtype << pin))
                });

                if pin < 8 {
                    let offset = 4 * pin;
                    regs.afrl.modify(|r, w| unsafe {
                        w.bits(
                            (r.bits() & !(0b1111 << offset))
                                | (af << offset),
                        )
                    });
                } else {
                    let offset = 4 * (pin - 8);
                    regs.afrh.modify(|r, w| unsafe {
                        w.bits(
                            (r.bits() & !(0b1111 << offset))
                                | (af << offset),
                        )
                    });
                }
            }
            _ => {}
        }

        let offset = 2 * pin;
        regs.moder.modify(|r, w| unsafe {
            w.bits(
                (r.bits() & !(0b11 << offset)) | (mode.val() << offset),
            )
        });

        Self {
            port,
            pin,
            mode,
            regs_ptr,
        }
    }

    #[inline(always)]
    /// Set output speed to Low, Medium, or High. Sets the `OSPEEDR` register.
    pub fn set_speed(&mut self, speed: Speed) {
        let regs = unsafe { &(*self.regs_ptr) };
        let offset = 2 * self.pin;

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
        let offset = 2 * self.pin;
        unsafe {
            regs.pupdr.modify(|r, w| {
                w.bits((r.bits() & !(0b11 << offset)) | (pull << offset))
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

        regs.bsrr.write(|w| unsafe { w.bits(1 << (offset + self.pin))});
    }

    #[inline(always)]
    /// Check if the pin's input voltage is high. Reads from the `IDR` register.
    pub fn is_high(&self) -> bool {
        let regs: &pac::gpioa::RegisterBlock = unsafe { &(*self.regs_ptr) };
        regs.idr.read().bits() & (1 << self.pin) != 0
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
        let bitmask = 1 << self.pin;
    
        // --- IMR の設定 ---
        // 特定のコア向けの割り込みマスクをビット演算でセットする
        #[cfg(all(not(any(feature = "h747cm4", feature = "h747cm7"))))]
        exti.cpuimr1.modify(|r, w| unsafe { w.bits(r.bits() | bitmask) });
        #[cfg(any(feature = "h747cm4", feature = "h747cm7"))]
        exti.c1imr1.modify(|r, w| unsafe { w.bits(r.bits() | bitmask)});

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
        let offset = (self.pin % 4) * 4;

        match self.pin {
            0..=3 => {
                syscfg.exticr1.modify(|r, w| unsafe {
                    w.bits((r.bits() & !(0xf << offset)) | (self.port << offset))
                });
            }
            4..=7 => {
                syscfg.exticr2.modify(|r, w| unsafe {
                    w.bits((r.bits() & !(0xf << offset)) | (self.port << offset))
                });
            }
            8..=11 => {
                syscfg.exticr3.modify(|r, w| unsafe {
                    w.bits((r.bits() & !(0xf << offset)) | (self.port << offset))
                });
            }
            12..=15 => {
                syscfg.exticr4.modify(|r, w| unsafe {
                    w.bits((r.bits() & !(0xf << offset)) | (self.port << offset))
                });
            }
            _ => unreachable!(),
        }
    }
}
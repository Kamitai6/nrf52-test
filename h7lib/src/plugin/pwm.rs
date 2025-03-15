//! Pulse Width Modulation (PWM)
//!
//! PWM output is avaliable for the advanced control timers (`TIM1`, `TIM8`),
//! the general purpose timers (`TIM[2-5]`, `TIM[12-17]`) 
//!
//! Timers support up to 4 simultaneous PWM output channels

use core::cell::RefCell;
use crate::{pac, Hertz, NanoSeconds, rcc_en_reset};
use crate::periph::{gpio, timer};

// Dual channel timers
// pins! {
//     pac::TIM12:
//         CH1(ComplementaryImpossible): [
//             gpio::PB14<Alternate<2>>,
//             gpio::PH6<Alternate<2>>
//         ]
//         CH2(ComplementaryImpossible): [
//             gpio::PB15<Alternate<2>>,
//             gpio::PH9<Alternate<2>>
//         ]
//         CH1N: []
//         CH2N: []
//     pac::TIM13:
//         CH1(ComplementaryImpossible): [
//             gpio::PA6<Alternate<9>>,
//             gpio::PF8<Alternate<9>>
//         ]
//         CH2(ComplementaryImpossible): []
//         CH1N: []
//         CH2N: []
//     pac::TIM14:
//         CH1(ComplementaryImpossible): [
//             gpio::PA7<Alternate<9>>,
//             gpio::PF9<Alternate<9>>
//         ]
//         CH2(ComplementaryImpossible): []
//         CH1N: []
//         CH2N: []
//     pac::TIM15:
//         CH1(ComplementaryDisabled): [
//             gpio::PA2<Alternate<4>>,
//             gpio::PE5<Alternate<4>>,
//         ]
//         CH2(ComplementaryImpossible): [
//             gpio::PA3<Alternate<4>>,
//             gpio::PE6<Alternate<4>>
//         ]
//         CH1N: [
//             gpio::PA1<Alternate<4>>,
//             gpio::PE4<Alternate<4>>
//         ]
//         CH2N: []
//     pac::TIM16:
//         CH1(ComplementaryDisabled): [
//             gpio::PB8<Alternate<1>>,
//             gpio::PF6<Alternate<1>>
//         ]
//         CH2(ComplementaryImpossible): []
//         CH1N: [
//             gpio::PB6<Alternate<1>>,
//             gpio::PF8<Alternate<1>>
//         ]
//         CH2N: []
//     pac::TIM17:
//         CH1(ComplementaryDisabled): [
//             gpio::PB9<Alternate<1>>,
//             gpio::PF7<Alternate<1>>
//         ]
//         CH2(ComplementaryImpossible): []
//         CH1N: [
//             gpio::PB7<Alternate<1>>,
//             gpio::PF9<Alternate<1>>
//         ]
//         CH2N: []
// }
// // Quad channel timers
// pins! {
//     pac::TIM1:
//         CH1(ComplementaryDisabled): [
//             gpio::PA8<Alternate<1>>,
//             gpio::PE9<Alternate<1>>,
//         ]
//         CH2(ComplementaryDisabled): [
//             gpio::PA9<Alternate<1>>,
//             gpio::PE11<Alternate<1>>,
//         ]
//         CH3(ComplementaryDisabled): [
//             gpio::PA10<Alternate<1>>,
//             gpio::PE13<Alternate<1>>,
//         ]
//         CH4(ComplementaryImpossible): [
//             gpio::PA11<Alternate<1>>,
//             gpio::PE14<Alternate<1>>
//         ]
//         CH1N: [
//             gpio::PA7<Alternate<1>>,
//             gpio::PB13<Alternate<1>>,
//             gpio::PE8<Alternate<1>>,
//         ]
//         CH2N: [
//             gpio::PB0<Alternate<1>>,
//             gpio::PB14<Alternate<1>>,
//             gpio::PE10<Alternate<1>>,
//         ]
//         CH3N: [
//             gpio::PB1<Alternate<1>>,
//             gpio::PB15<Alternate<1>>,
//             gpio::PE12<Alternate<1>>,
//         ]
//         CH4N: []
//     pac::TIM2:
//         CH1(ComplementaryImpossible): [
//             gpio::PA0<Alternate<1>>,
//             gpio::PA5<Alternate<1>>,
//             gpio::PA15<Alternate<1>>
//         ]
//         CH2(ComplementaryImpossible): [
//             gpio::PA1<Alternate<1>>,
//             gpio::PB3<Alternate<1>>
//         ]
//         CH3(ComplementaryImpossible): [
//             gpio::PA2<Alternate<1>>,
//             gpio::PB10<Alternate<1>>
//         ]
//         CH4(ComplementaryImpossible): [
//             gpio::PA3<Alternate<1>>,
//             gpio::PB11<Alternate<1>>
//         ]
//         CH1N: []
//         CH2N: []
//         CH3N: []
//         CH4N: []
//     pac::TIM3:
//         CH1(ComplementaryImpossible): [
//             gpio::PA6<Alternate<2>>,
//             gpio::PB4<Alternate<2>>,
//             gpio::PC6<Alternate<2>>
//         ]
//         CH2(ComplementaryImpossible): [
//             gpio::PA7<Alternate<2>>,
//             gpio::PB5<Alternate<2>>,
//             gpio::PC7<Alternate<2>>
//         ]
//         CH3(ComplementaryImpossible): [
//             gpio::PB0<Alternate<2>>,
//             gpio::PC8<Alternate<2>>
//         ]
//         CH4(ComplementaryImpossible): [
//             gpio::PB1<Alternate<2>>,
//             gpio::PC9<Alternate<2>>
//         ]
//         CH1N: []
//         CH2N: []
//         CH3N: []
//         CH4N: []
//     pac::TIM4:
//         CH1(ComplementaryImpossible): [
//             gpio::PB6<Alternate<2>>,
//             gpio::PD12<Alternate<2>>
//         ]
//         CH2(ComplementaryImpossible): [
//             gpio::PB7<Alternate<2>>,
//             gpio::PD13<Alternate<2>>
//         ]
//         CH3(ComplementaryImpossible): [
//             gpio::PB8<Alternate<2>>,
//             gpio::PD14<Alternate<2>>
//         ]
//         CH4(ComplementaryImpossible): [
//             gpio::PB9<Alternate<2>>,
//             gpio::PD15<Alternate<2>>
//         ]
//         CH1N: []
//         CH2N: []
//         CH3N: []
//         CH4N: []
//     pac::TIM5:
//         CH1(ComplementaryImpossible): [
//             gpio::PA0<Alternate<2>>,
//             gpio::PH10<Alternate<2>>
//         ]
//         CH2(ComplementaryImpossible): [
//             gpio::PA1<Alternate<2>>,
//             gpio::PH11<Alternate<2>>
//         ]
//         CH3(ComplementaryImpossible): [
//             gpio::PA2<Alternate<2>>,
//             gpio::PH12<Alternate<2>>
//         ]
//         CH4(ComplementaryImpossible): [
//             gpio::PA3<Alternate<2>>,
//         ]
//         CH1N: []
//         CH2N: []
//         CH3N: []
//         CH4N: []
//     pac::TIM8:
//         CH1(ComplementaryDisabled): [
//             gpio::PC6<Alternate<3>>,
//         ]
//         CH2(ComplementaryDisabled): [
//             gpio::PC7<Alternate<3>>,
//         ]
//         CH3(ComplementaryDisabled): [
//             gpio::PC8<Alternate<3>>,
//         ]
//         CH4(ComplementaryImpossible): [
//             gpio::PC9<Alternate<3>>,
//         ]
//         CH1N: [
//             gpio::PA5<Alternate<3>>,
//             gpio::PA7<Alternate<3>>,
//             gpio::PH13<Alternate<3>>,
//         ]
//         CH2N: [
//             gpio::PB0<Alternate<3>>,
//             gpio::PB14<Alternate<3>>,
//             gpio::PH14<Alternate<3>>,
//         ]
//         CH3N: [
//             gpio::PB1<Alternate<3>>,
//             gpio::PB15<Alternate<3>>,
//             gpio::PH15<Alternate<3>>,
//         ]
//         CH4N: []
// }

/// Enum for IO polarity
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Polarity {
    ActiveHigh,
    ActiveLow,
}

pub struct Pwm<const T: u8, const N: u8> {
    channel: timer::Channel<T, N>,
    with_complexity: bool,
}

macro_rules! make_pwm {
    ($T:expr, $N:expr, $ccmrx_output:ident, $ocxpe:ident, $ocxm:ident, $cntType:ident) => {
        paste::paste! {
        impl Pwm<$T, $N> {
            pub fn new<G>(channel: timer::Channel<$T, $N>, pin: G) -> Self
            {
                Self {
                    channel,
                    with_complexity: false,
                }
            }
            pub fn new_with_comp<G, CG>(channel: timer::Channel<$T, $N>, pin: G, pin2: CG) -> Self
            {
                let tim = unsafe { &*pac::[<TIM $T>]::ptr() };
                let enabled = (tim.ccer.read().bits() & 1 << (($N - 1) * 4)) != 0;

                assert!(!enabled);

                Self {
                    channel,
                    with_complexity: true,
                }
            }

            pub fn enable(&mut self) {
                let tim = unsafe { &*pac::[<TIM $T>]::ptr() };

                tim.$ccmrx_output().modify(|_, w|
                    w.$ocxpe()
                        .enabled() // Enable preload
                        .$ocxm()
                        .pwm_mode1() // PWM Mode
                );

                self.ccer_enable();
            }

            pub fn disable(&mut self) {
                self.ccer_disable();
            }

            pub fn get_duty(&self) -> $cntType {
                let tim = unsafe { &*pac::[<TIM $T>]::ptr() };

                tim.ccr[($N - 1) as usize].read().ccr().bits()
            }

            pub fn get_max_duty(&self) -> $cntType {
                let tim = unsafe { &*pac::[<TIM $T>]::ptr() };

                let arr = tim.arr.read().arr().bits();

                // One PWM cycle is ARR+1 counts long
                // Valid PWM duty cycles are 0 to ARR+1
                // However, if ARR is 65535 on a 16-bit timer, we can't add 1
                // In that case, 100% duty cycle is not possible, only 65535/65536
                if arr == $cntType::MAX {
                    arr
                }
                else {
                    arr + 1
                }
            }

            pub fn set_duty(&mut self, duty: $cntType) {
                let tim = unsafe { &*pac::[<TIM $T>]::ptr() };

                tim.ccr[($N - 1) as usize].write(|w| w.ccr().bits(duty));
            }

            fn ccer_enable(&mut self) {
                let tim = unsafe { &*pac::[<TIM $T>]::ptr() };
                if self.with_complexity {
                    tim.ccer.modify(|r, w| unsafe { w.bits(r.bits() | 1 << (($N - 1) * 4) | 1 << (($N - 1) * 4 + 2)) });
                }
                else {
                    tim.ccer.modify(|r, w| unsafe { w.bits(r.bits() | 1 << (($N - 1) * 4)) });
                }
            }
            fn ccer_disable(&mut self) {
                let tim = unsafe { &*pac::[<TIM $T>]::ptr() };

                if self.with_complexity {
                    tim.ccer.modify(|r, w| unsafe { w.bits(r.bits() & !(1 << (($N - 1) * 4)) & !(1 << (($N - 1) * 4 + 2))) });
                }
                else {
                    tim.ccer.modify(|r, w| unsafe { w.bits(r.bits() & !(1 << (($N - 1) * 4))) });
                }
            }

            fn set_polarity(&mut self, pol: Polarity) {
                let tim = unsafe { &*pac::[<TIM $T>]::ptr() };

                tim.ccer.modify(|r, w| unsafe { w.bits(match pol {
                    Polarity::ActiveLow => r.bits() | 1 << (($N - 1) * 4 + 1),
                    Polarity::ActiveHigh => r.bits() & !(1 << (($N - 1) * 4 + 1)),
                })});
            }

            fn set_comp_polarity(&mut self, pol: Polarity) {
                let tim = unsafe { &*pac::[<TIM $T>]::ptr() };

                tim.ccer.modify(|r, w| unsafe { w.bits(match pol {
                    Polarity::ActiveLow => r.bits() | 1 << (($N - 1) * 4 + 3),
                    Polarity::ActiveHigh => r.bits() & !(1 << (($N - 1) * 4 + 3)),
                })});
            }
        }}
    }
}

make_pwm!(1, 1, ccmr1_output, oc1pe, oc1m, u16);
make_pwm!(1, 2, ccmr1_output, oc2pe, oc2m, u16);
make_pwm!(1, 3, ccmr2_output, oc3pe, oc3m, u16);
make_pwm!(1, 4, ccmr2_output, oc4pe, oc4m, u16);

make_pwm!(2, 1, ccmr1_output, oc1pe, oc1m, u32);
make_pwm!(2, 2, ccmr1_output, oc2pe, oc2m, u32);
make_pwm!(2, 3, ccmr2_output, oc3pe, oc3m, u32);
make_pwm!(2, 4, ccmr2_output, oc4pe, oc4m, u32);

make_pwm!(3, 1, ccmr1_output, oc1pe, oc1m, u16);
make_pwm!(3, 2, ccmr1_output, oc2pe, oc2m, u16);
make_pwm!(3, 3, ccmr2_output, oc3pe, oc3m, u16);
make_pwm!(3, 4, ccmr2_output, oc4pe, oc4m, u16);

make_pwm!(4, 1, ccmr1_output, oc1pe, oc1m, u16);
make_pwm!(4, 2, ccmr1_output, oc2pe, oc2m, u16);
make_pwm!(4, 3, ccmr2_output, oc3pe, oc3m, u16);
make_pwm!(4, 4, ccmr2_output, oc4pe, oc4m, u16);

make_pwm!(5, 1, ccmr1_output, oc1pe, oc1m, u32);
make_pwm!(5, 2, ccmr1_output, oc2pe, oc2m, u32);
make_pwm!(5, 3, ccmr2_output, oc3pe, oc3m, u32);
make_pwm!(5, 4, ccmr2_output, oc4pe, oc4m, u32);

make_pwm!(8, 1, ccmr1_output, oc1pe, oc1m, u16);
make_pwm!(8, 2, ccmr1_output, oc2pe, oc2m, u16);
make_pwm!(8, 3, ccmr2_output, oc3pe, oc3m, u16);
make_pwm!(8, 4, ccmr2_output, oc4pe, oc4m, u16);

make_pwm!(12, 1, ccmr1_output, oc1pe, oc1m, u16);
make_pwm!(12, 2, ccmr1_output, oc2pe, oc2m, u16);

make_pwm!(13, 1, ccmr1_output, oc1pe, oc1m, u16);

make_pwm!(14, 1, ccmr1_output, oc1pe, oc1m, u16);

make_pwm!(15, 1, ccmr1_output, oc1pe, oc1m, u16);
make_pwm!(15, 2, ccmr1_output, oc2pe, oc2m, u16);

make_pwm!(16, 1, ccmr1_output, oc1pe, oc1m, u16);

make_pwm!(17, 1, ccmr1_output, oc1pe, oc1m, u16);
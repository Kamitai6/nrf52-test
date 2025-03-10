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
pins! {
    pac::TIM12:
        CH1(ComplementaryImpossible): [
            gpio::PB14<Alternate<2>>,
            gpio::PH6<Alternate<2>>
        ]
        CH2(ComplementaryImpossible): [
            gpio::PB15<Alternate<2>>,
            gpio::PH9<Alternate<2>>
        ]
        CH1N: []
        CH2N: []
    pac::TIM13:
        CH1(ComplementaryImpossible): [
            gpio::PA6<Alternate<9>>,
            gpio::PF8<Alternate<9>>
        ]
        CH2(ComplementaryImpossible): []
        CH1N: []
        CH2N: []
    pac::TIM14:
        CH1(ComplementaryImpossible): [
            gpio::PA7<Alternate<9>>,
            gpio::PF9<Alternate<9>>
        ]
        CH2(ComplementaryImpossible): []
        CH1N: []
        CH2N: []
    pac::TIM15:
        CH1(ComplementaryDisabled): [
            gpio::PA2<Alternate<4>>,
            gpio::PE5<Alternate<4>>,
        ]
        CH2(ComplementaryImpossible): [
            gpio::PA3<Alternate<4>>,
            gpio::PE6<Alternate<4>>
        ]
        CH1N: [
            gpio::PA1<Alternate<4>>,
            gpio::PE4<Alternate<4>>
        ]
        CH2N: []
    pac::TIM16:
        CH1(ComplementaryDisabled): [
            gpio::PB8<Alternate<1>>,
            gpio::PF6<Alternate<1>>
        ]
        CH2(ComplementaryImpossible): []
        CH1N: [
            gpio::PB6<Alternate<1>>,
            gpio::PF8<Alternate<1>>
        ]
        CH2N: []
    pac::TIM17:
        CH1(ComplementaryDisabled): [
            gpio::PB9<Alternate<1>>,
            gpio::PF7<Alternate<1>>
        ]
        CH2(ComplementaryImpossible): []
        CH1N: [
            gpio::PB7<Alternate<1>>,
            gpio::PF9<Alternate<1>>
        ]
        CH2N: []
}
// Quad channel timers
pins! {
    pac::TIM1:
        CH1(ComplementaryDisabled): [
            gpio::PA8<Alternate<1>>,
            gpio::PE9<Alternate<1>>,
        ]
        CH2(ComplementaryDisabled): [
            gpio::PA9<Alternate<1>>,
            gpio::PE11<Alternate<1>>,
        ]
        CH3(ComplementaryDisabled): [
            gpio::PA10<Alternate<1>>,
            gpio::PE13<Alternate<1>>,
        ]
        CH4(ComplementaryImpossible): [
            gpio::PA11<Alternate<1>>,
            gpio::PE14<Alternate<1>>
        ]
        CH1N: [
            gpio::PA7<Alternate<1>>,
            gpio::PB13<Alternate<1>>,
            gpio::PE8<Alternate<1>>,
        ]
        CH2N: [
            gpio::PB0<Alternate<1>>,
            gpio::PB14<Alternate<1>>,
            gpio::PE10<Alternate<1>>,
        ]
        CH3N: [
            gpio::PB1<Alternate<1>>,
            gpio::PB15<Alternate<1>>,
            gpio::PE12<Alternate<1>>,
        ]
        CH4N: []
    pac::TIM2:
        CH1(ComplementaryImpossible): [
            gpio::PA0<Alternate<1>>,
            gpio::PA5<Alternate<1>>,
            gpio::PA15<Alternate<1>>
        ]
        CH2(ComplementaryImpossible): [
            gpio::PA1<Alternate<1>>,
            gpio::PB3<Alternate<1>>
        ]
        CH3(ComplementaryImpossible): [
            gpio::PA2<Alternate<1>>,
            gpio::PB10<Alternate<1>>
        ]
        CH4(ComplementaryImpossible): [
            gpio::PA3<Alternate<1>>,
            gpio::PB11<Alternate<1>>
        ]
        CH1N: []
        CH2N: []
        CH3N: []
        CH4N: []
    pac::TIM3:
        CH1(ComplementaryImpossible): [
            gpio::PA6<Alternate<2>>,
            gpio::PB4<Alternate<2>>,
            gpio::PC6<Alternate<2>>
        ]
        CH2(ComplementaryImpossible): [
            gpio::PA7<Alternate<2>>,
            gpio::PB5<Alternate<2>>,
            gpio::PC7<Alternate<2>>
        ]
        CH3(ComplementaryImpossible): [
            gpio::PB0<Alternate<2>>,
            gpio::PC8<Alternate<2>>
        ]
        CH4(ComplementaryImpossible): [
            gpio::PB1<Alternate<2>>,
            gpio::PC9<Alternate<2>>
        ]
        CH1N: []
        CH2N: []
        CH3N: []
        CH4N: []
    pac::TIM4:
        CH1(ComplementaryImpossible): [
            gpio::PB6<Alternate<2>>,
            gpio::PD12<Alternate<2>>
        ]
        CH2(ComplementaryImpossible): [
            gpio::PB7<Alternate<2>>,
            gpio::PD13<Alternate<2>>
        ]
        CH3(ComplementaryImpossible): [
            gpio::PB8<Alternate<2>>,
            gpio::PD14<Alternate<2>>
        ]
        CH4(ComplementaryImpossible): [
            gpio::PB9<Alternate<2>>,
            gpio::PD15<Alternate<2>>
        ]
        CH1N: []
        CH2N: []
        CH3N: []
        CH4N: []
    pac::TIM5:
        CH1(ComplementaryImpossible): [
            gpio::PA0<Alternate<2>>,
            gpio::PH10<Alternate<2>>
        ]
        CH2(ComplementaryImpossible): [
            gpio::PA1<Alternate<2>>,
            gpio::PH11<Alternate<2>>
        ]
        CH3(ComplementaryImpossible): [
            gpio::PA2<Alternate<2>>,
            gpio::PH12<Alternate<2>>
        ]
        CH4(ComplementaryImpossible): [
            gpio::PA3<Alternate<2>>,
        ]
        CH1N: []
        CH2N: []
        CH3N: []
        CH4N: []
    pac::TIM8:
        CH1(ComplementaryDisabled): [
            gpio::PC6<Alternate<3>>,
        ]
        CH2(ComplementaryDisabled): [
            gpio::PC7<Alternate<3>>,
        ]
        CH3(ComplementaryDisabled): [
            gpio::PC8<Alternate<3>>,
        ]
        CH4(ComplementaryImpossible): [
            gpio::PC9<Alternate<3>>,
        ]
        CH1N: [
            gpio::PA5<Alternate<3>>,
            gpio::PA7<Alternate<3>>,
            gpio::PH13<Alternate<3>>,
        ]
        CH2N: [
            gpio::PB0<Alternate<3>>,
            gpio::PB14<Alternate<3>>,
            gpio::PH14<Alternate<3>>,
        ]
        CH3N: [
            gpio::PB1<Alternate<3>>,
            gpio::PB15<Alternate<3>>,
            gpio::PH15<Alternate<3>>,
        ]
        CH4N: []
}

pub struct Pwm<const N: u8> {
    timer: timer::Timer<N>,
    channel: u8
}

impl<const N: u8> Pwm<N> {
    fn new(gpio::GPIO<>, Option(GPIO)){}

    fn enable(&mut self) {
        let tim = unsafe { &*<$TIMX>::ptr() };

        tim.$ccmrx_output().modify(|_, w|
            w.$ocxpe()
                .enabled() // Enable preload
                .$ocxm()
                .pwm_mode1() // PWM Mode
        );

        self.ccer_enable();
    }

    fn disable(&mut self) {
        self.ccer_disable();
    }

    fn get_duty(&self) -> Self::Duty {
        let tim = unsafe { &*<$TIMX>::ptr() };

        tim.ccr[$CH as usize].read().ccr().bits()
    }

    fn get_max_duty(&self) -> Self::Duty {
        let tim = unsafe { &*<$TIMX>::ptr() };

        let arr = tim.arr.read().arr().bits();

        // One PWM cycle is ARR+1 counts long
        // Valid PWM duty cycles are 0 to ARR+1
        // However, if ARR is 65535 on a 16-bit timer, we can't add 1
        // In that case, 100% duty cycle is not possible, only 65535/65536
        if arr == Self::Duty::MAX {
            arr
        }
        else {
            arr + 1
        }
    }

    fn set_duty(&mut self, duty: Self::Duty) {
        let tim = unsafe { &*<$TIMX>::ptr() };

        tim.ccr[$CH as usize].write(|w| w.ccr().bits(duty));
    }

    fn ccer_enable(&mut self) {
        let tim = unsafe { &*<$TIMX>::ptr() };

        tim.ccer.modify(|r, w| unsafe { w.bits(r.bits() | Ch::<C>::EN) });
    }
    fn ccer_disable(&mut self) {
        let tim = unsafe { &*<$TIMX>::ptr() };

        tim.ccer.modify(|r, w| unsafe { w.bits(r.bits() & !Ch::<C>::EN) });
    }

    pub fn set_polarity(&mut self, pol: Polarity) {
        let tim = unsafe { &*<$TIMX>::ptr() };

        tim.ccer.modify(|r, w| unsafe { w.bits(match pol {
            Polarity::ActiveLow => r.bits() | Ch::<C>::POL,
            Polarity::ActiveHigh => r.bits() & !Ch::<C>::POL,
        })});
    }

    pub fn into_complementary<NPIN>(self, _npin: NPIN) -> Pwm<$TIMX, C, ComplementaryEnabled>
        where NPIN: NPins<$TIMX, Ch<C>> {
        // Make sure we aren't switching to complementary after we enable the channel
        let tim = unsafe { &*<$TIMX>::ptr() };

        let enabled = (tim.ccer.read().bits() & Ch::<C>::EN) != 0;

        assert!(!enabled);

        Pwm::new()
    }

    pub fn set_comp_polarity(&mut self, pol: Polarity) {
        let tim = unsafe { &*<$TIMX>::ptr() };

        tim.ccer.modify(|r, w| unsafe { w.bits(match pol {
            Polarity::ActiveLow => r.bits() | Ch::<C>::N_POL,
            Polarity::ActiveHigh => r.bits() & !Ch::<C>::N_POL,
        })});
    }
}

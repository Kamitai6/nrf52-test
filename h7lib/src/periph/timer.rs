//! Timers

// use crate::stm32::rcc::{d2ccip2r as ccip2r, d3ccipr as srdccipr};

use crate::{pac, Hertz};
use super::rcc;
use crate::pac::{
    TIM1, TIM12, TIM13, TIM14, TIM15, TIM16, TIM17, TIM2, TIM3, TIM4, TIM5,
    TIM6, TIM7, TIM8,
};

pub enum TimerType {
    INT,
    LOOP
}

/// Associate clocks with timers
pub trait GetClk {
    fn get_clk(clocks: &rcc::CoreClocks) -> Option<Hertz>;
}
/// Timers with CK_INT derived from rcc_tim[xy]_ker_ck
macro_rules! impl_tim_ker_ck {
    ($($ckX:ident: $($TIMX:ident),+)+) => {
        $(
            $(
                impl GetClk for $TIMX {
                    fn get_clk(clocks: &rcc::CoreClocks) -> Option<Hertz> {
                        Some(clocks.$ckX)
                    }
                }
            )+
        )+
    }
}
impl_tim_ker_ck! {
    timx_ker_ck: TIM2, TIM3, TIM4, TIM5, TIM6, TIM7, TIM12, TIM13, TIM14
    timy_ker_ck: TIM1, TIM8, TIM15, TIM16, TIM17
}

/// Hardware timers
pub struct Timer<TIM>
{
    pub tim_type: TimerType,
    clock: u32,
    regs: TIM,
}

macro_rules! make_timer {
    ($TIMX:ident, $timX:ident, $cntType:ty) => {
        impl Timer<pac::$TIMX> {
            pub fn $timX(regs: pac::$TIMX, tim_type: TimerType, clocks: rcc::CoreClocks) -> Self {
                // enable and reset peripheral to a clean state
                // let _ = prec.enable().reset(); // drop, can be recreated by free method

                let clock = $TIMX::get_clk(&clocks)
                .expect(concat!(stringify!($TIMX), ": Input clock not running!")).raw();

                Self {
                    tim_type,
                    clock,
                    regs,
                }
            }

            pub fn start(mut self, value: Hertz) {
                self.pause();

                // UEV event occours on next overflow
                self.urs_counter_only();
                self.clear_irq();
                
                match self.tim_type {
                    TimerType::LOOP => self.set_stopwatch_frequency(value),
                    TimerType::INT => self.set_timeout_interval(value),
                }
                // Generate an update event to force an update of the ARR
                // register. This ensures the first timer cycle is of the
                // specified duration.
                self.apply_freq();

                // Start counter
                self.resume();
            }

            fn wait(&mut self) -> Result<(), ()> {
                if self.is_irq_clear() {
                    Err(())
                } else {
                    self.clear_irq();
                    Ok(())
                }
            }

            /// Configures the timer's frequency and counter reload value
            /// so that it underflows at the timeout's frequency
            pub fn set_timeout_interval(&mut self, timeout: Hertz) {
                let ticks = self.clock / timeout.raw();

                self.set_timeout_ticks(ticks);
            }

            pub fn set_timeout<T>(&mut self, timeout: T)
            where
                T: Into<core::time::Duration>
            {
                const NANOS_PER_SECOND: u64 = 1_000_000_000;
                let timeout = timeout.into();

                let clk = u64::from(self.clock);
                let ticks = u32::try_from(
                    clk * timeout.as_secs() +
                    clk * u64::from(timeout.subsec_nanos()) / NANOS_PER_SECOND,
                )
                .unwrap_or(u32::MAX);

                self.set_timeout_ticks(ticks.max(1));
            }

            /// Sets the timer's prescaler and auto reload register so that the timer will reach
            /// the ARR after `ticks - 1` amount of timer clock ticks.
            ///
            /// ```
            /// // Set auto reload register to 50000 and prescaler to divide by 2.
            /// timer.set_timeout_ticks(100000);
            /// ```
            ///
            /// This function will round down if the prescaler is used to extend the range:
            /// ```
            /// // Set auto reload register to 50000 and prescaler to divide by 2.
            /// timer.set_timeout_ticks(100001);
            /// ```
            fn set_timeout_ticks(&mut self, ticks: u32) {
                let psc = u16::try_from(ticks / (1 << 16)).expect("ticks / (1 << 16) is overflow");
                // Note (unwrap): Never panics because the divisor is always such that the result fits in 16 bits.
                // Also note that the timer counts `0..=arr`, so subtract 1 to get the correct period.
                let arr = u16::try_from(ticks / (u32::from(psc) + 1)).unwrap_or(u16::MAX).saturating_sub(1);
                self.regs.psc.write(|w| w.psc().bits(psc));
                #[allow(unused_unsafe)] // method is safe for some timers
                self.regs.arr.write(|w| unsafe { w.bits(u32::from(arr)) });
            }

            /// Configures the timer to count up at the given frequency
            ///
            /// Counts from 0 to the counter's maximum value, then repeats.
            /// Because this only uses the timer prescaler, the frequency
            /// is rounded to a multiple of the timer's kernel clock.
            pub fn set_stopwatch_frequency(&mut self, frequency: Hertz) {
                let div = self.clock / frequency.raw();

                let psc = u16::try_from(div - 1).expect("div - 1 is overflow");
                self.regs.psc.write(|w| w.psc().bits(psc));

                let counter_max = u32::from(<$cntType>::MAX);
                #[allow(unused_unsafe)] // method is safe for some timers
                self.regs.arr.write(|w| unsafe { w.bits(counter_max) });
            }

            /// Applies frequency/timeout changes immediately
            ///
            /// The timer will normally update its prescaler and auto-reload
            /// value when its counter overflows. This function causes
            /// those changes to happen immediately. Also clears the counter.
            pub fn apply_freq(&mut self) {
                self.regs.egr.write(|w| w.ug().set_bit());
            }

            /// Pauses the TIM peripheral
            pub fn pause(&mut self) {
                self.regs.cr1.modify(|_, w| w.cen().clear_bit());
            }

            /// Resume (unpause) the TIM peripheral
            pub fn resume(&mut self) {
                self.regs.cr1.modify(|_, w| w.cen().set_bit());
            }

            /// Set Update Request Source to counter overflow/underflow only
            pub fn urs_counter_only(&mut self) {
                self.regs.cr1.modify(|_, w| w.urs().counter_only());
            }

            /// Reset the counter of the TIM peripheral
            pub fn reset_counter(&mut self) {
                self.regs.cnt.reset();
            }

            /// Read the counter of the TIM peripheral
            pub fn counter(&self) -> u32 {
                self.regs.cnt.read().cnt().bits().into()
            }

            /// Start listening for `event`
            pub fn listen(&mut self) {
                self.regs.dier.write(|w| w.uie().set_bit());
            }

            /// Stop listening for `event`
            pub fn unlisten(&mut self) {
                // Disable update event interrupt
                self.regs.dier.write(|w| w.uie().clear_bit());
                let _ = self.regs.dier.read();
                let _ = self.regs.dier.read(); // Delay 2 peripheral clocks
            }

            /// Check if Update Interrupt flag is cleared
            pub fn is_irq_clear(&mut self) -> bool {
                self.regs.sr.read().uif().bit_is_clear()
            }

            /// Clears interrupt flag
            pub fn clear_irq(&mut self) {
                self.regs.sr.modify(|_, w| {
                    // Clears timeout event
                    w.uif().clear_bit()
                });
                let _ = self.regs.sr.read();
                let _ = self.regs.sr.read(); // Delay 2 peripheral clocks
            }

            /// Releases the TIM peripheral
            pub fn free(mut self) -> Self {
                // pause counter
                self.pause();

                self
            }
        }
    }
}

// Advanced-control
make_timer!(TIM1, tim1, u16);
make_timer!(TIM8, tim8, u16);

make_timer!(TIM2, tim2, u32);
make_timer!(TIM3, tim3, u16);
make_timer!(TIM4, tim4, u16);
make_timer!(TIM5, tim5, u32);

make_timer!(TIM6, tim6, u16);
make_timer!(TIM7, tim7, u16);

make_timer!(TIM12, tim12, u16);
make_timer!(TIM13, tim13, u16);
make_timer!(TIM14, tim14, u16);
make_timer!(TIM15, tim15, u16);
make_timer!(TIM16, tim16, u16);
make_timer!(TIM17, tim17, u16);

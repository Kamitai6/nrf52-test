//! Timers

// use crate::stm32::rcc::{d2ccip2r as ccip2r, d3ccipr as srdccipr};

use crate::{pac, Hertz, NanoSeconds, rcc_en_reset};
use super::rcc;

#[derive(Copy, Clone, Debug)]
pub enum CountMode {
    Interrupt,
    Loop,
}

#[derive(Copy, Clone, Debug)]
pub enum ChannelFunction {
    Pwm, // + complementary pwm
    EncoderSensor, // + hole sensor
    InputCapture,
    OutputCompare,
    OnePulse,
    Slave,
}

/// Enum for IO polarity
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Polarity {
    ActiveHigh,
    ActiveLow,
}

/// Whether a PWM signal is left-aligned, right-aligned, or center-aligned
#[derive(Copy, Clone, Debug)]
pub enum Alignment {
    Left,
    Right,
    Center,
}

pub struct ChannelOption {
    polarity: Polarity,
    alignment: Option<Alignment>,
    deadtime: Option<NanoSeconds>,
}

/// Timer channel
pub struct Channel<const TIM: u8, const N: u8> {
}

impl<const TIM: u8, const N: u8> Channel<TIM, N> {
    fn set_compare() {
    }
    fn get_count() {
    }
}

/// Normal timer
pub struct Timer<const N: u8>
{
    pub count_mode: CountMode,
    clock: u32,
    is_timerx: bool,
    is_32bit: bool,
}

macro_rules! timer_tuple_type {
    ($N:expr, 1) => { (Channel<$N, 1>) };
    ($N:expr, 2) => { (Channel<$N, 1>, Channel<$N, 2>) };
    ($N:expr, 3) => { (Channel<$N, 1>, Channel<$N, 2>, Channel<$N, 3>) };
    ($N:expr, 4) => { (Channel<$N, 1>, Channel<$N, 2>, Channel<$N, 3>, Channel<$N, 4>) };
}

// 一旦秋田けど、あとで、型の違いとかは、マクロのパラメータにしないと無理そうだから、やる
macro_rules! make_timer {
    ($N:expr, $ChN:tt) => {
        paste::paste! {
        impl Timer<$N> {
            pub fn new(count_mode: CountMode, core_clocks: &rcc::CoreClocks) -> Self 
            {
                let rcc = unsafe { &(*pac::RCC::ptr()) };
                
                match $N {
                    1 => rcc_en_reset!(apb2, tim1, rcc),
                    2 => rcc_en_reset!(apb1, tim2, rcc),
                    3 => rcc_en_reset!(apb1, tim3, rcc),
                    4 => rcc_en_reset!(apb1, tim4, rcc),
                    5 => rcc_en_reset!(apb1, tim5, rcc),
                    6 => rcc_en_reset!(apb1, tim6, rcc),
                    7 => rcc_en_reset!(apb1, tim7, rcc),
                    8 => rcc_en_reset!(apb2, tim8, rcc),
                    12 => rcc_en_reset!(apb1, tim12, rcc),
                    13 => rcc_en_reset!(apb1, tim13, rcc),
                    14 => rcc_en_reset!(apb1, tim14, rcc),
                    15 => rcc_en_reset!(apb2, tim15, rcc),
                    16 => rcc_en_reset!(apb2, tim16, rcc),
                    17 => rcc_en_reset!(apb2, tim17, rcc),
                    _ => unreachable!(),
                }

                let clock = match $N {
                    1 | 8 | 15..17 => core_clocks.timy_ker_ck.raw(),
                    _ => core_clocks.timx_ker_ck.raw(),
                };

                let is_timerx = match $N {
                    1 | 8 | 15..17 => true,
                    _ => false,
                };

                let is_32bit = match $N {
                    2 | 5 => true,
                    _ => false,
                };

                Self {
                    count_mode,
                    clock,
                    is_timerx,
                    is_32bit,
                }
            }

            pub fn split(self, option: ChannelOption) -> timer_tuple_type!($N, $ChN)
            {
                let regs = unsafe { &(*pac::[<TIM $N>]::ptr()) };
                let base_freq = self.clock;
                
                let divisor = if let Some(Alignment::Center) = option.alignment {
                    base_freq * 2
                } else {
                    base_freq
                };

                // Round to the nearest period
                let arr = (base_freq + (divisor >> 1)) / divisor - 1;

                let (period, prescale): (u32, u32) = if self.is_32bit {
                    (arr, 0)
                }
                else {
                    let ideal_period = arr + 1;

                    // Division factor is (PSC + 1)
                    let prescale = (ideal_period - 1) / (1 << 16);
            
                    // This will always fit in a 16-bit value because u32::MAX / (1 << 16) fits in a 16 bit
            
                    // Round to the nearest period
                    let period = (ideal_period + (prescale >> 1)) / (prescale + 1) - 1;
            
                    // It should be impossible to fail these asserts
                    assert!(period <= 0xFFFF);
                    assert!(prescale <= 0xFFFF);

                    (period, prescale)
                };

                // Write prescale
                regs.psc.write(|w| { w.psc().bits(prescale as u16) });

                // Write period
                match self.is_32bit {
                    false => regs.arr.write(|w| { w.arr().bits(period.try_into().unwrap()) }),
                    true => regs.arr.write(|w| { w.arr().bits(period.try_into().unwrap())}),
                }

                // Set CCxP = OCxREF / CCxNP = !OCxREF
                // Refer to RM0433 Rev 6 - Table 324.
                if self.is_timerx {
                    regs.bdtr.write(|w| w.moe().set_bit());
                }

                if let Some(deadtime) = option.deadtime {
                    // tDTS is based on tCK_INT which is before the prescaler
                    // It uses its own separate prescaler CR1.CKD

                    // ticks = ns * GHz = ns * Hz / 1e9
                    // Cortex-M7 has 32x32->64 multiply but no 64-bit divide
                    // Divide by 100000 then 10000 by multiplying and shifting
                    // This can't overflow because both values being multiplied are u32
                    let deadtime_ticks = deadtime.ticks() as u64 * base_freq as u64;
                    // Make sure we won't overflow when multiplying; DTG is max 1008 ticks and CKD is max prescaler of 4
                    // so deadtimes over 4032 ticks are impossible (4032*10^9 before dividing)
                    assert!(deadtime_ticks <= 4_032_000_000_000u64);
                    let deadtime_ticks = deadtime_ticks * 42950;
                    let deadtime_ticks = (deadtime_ticks >> 32) as u32;
                    let deadtime_ticks = deadtime_ticks as u64 * 429497;
                    let deadtime_ticks = (deadtime_ticks >> 32) as u32;

                    // Choose CR1 CKD divider of 1, 2, or 4 to determine tDTS
                    let (deadtime_ticks, ckd) = match deadtime_ticks {
                        t if t <= 1008 => (deadtime_ticks, 1),
                        t if t <= 2016 => (deadtime_ticks / 2, 2),
                        t if t <= 4032 => (deadtime_ticks / 4, 4),
                        _ => {
                            panic!("Deadtime must be less than 4032 ticks of timer base clock.")
                        }
                    };

                    // Choose BDTR DTG bits to match deadtime_ticks
                    // We want the smallest value of DTG that gives a deadtime >= the requested deadtime
                    let (dtg, ckd) = {
                        let mut result = (0, 0);
                        for dtg in 0..=255 {
                            let actual_deadtime: u32 = match dtg {
                                d if d < 128 => d,
                                d if d < 192 => 2 * (64 + (d & 0x3F)),
                                d if d < 224 => 8 * (32 + (d & 0x1F)),
                                _ => 16 * (32 + (dtg & 0x1F)),
                            };

                            if actual_deadtime >= deadtime_ticks {
                                result = (dtg as u8, ckd);
                                break; // ループを終了
                            }
                        }
                        result
                    };

                    match ckd {
                        1 => timer.regs.cr1.modify(|_, w| w.ckd().div1()),
                        2 => timer.regs.cr1.modify(|_, w| w.ckd().div2()),
                        4 => timer.regs.cr1.modify(|_, w| w.ckd().div4()),
                        _ => panic!("Should be unreachable, invalid deadtime prescaler"),
                    }
                        
                    // Safety: the DTG field of BDTR allows any 8-bit deadtime value and the dtg variable is u8
                    unsafe {
                        timer.regs.$bdtr.write(|w| w.dtg().bits(dtg).aoe().clear_bit().moe().$moe_set());
                    }
                }

                match self.alignment {
                    Alignment::Left => { },
                    Alignment::Right => { tim.cr1.modify(|_, w| w.dir().down()); },
                    Alignment::Center => { tim.cr1.modify(|_, w| w.$cms().center_aligned3()); }
                }

                tim.cr1.write(|w| w.cen().enabled());

                (
                    Channel::<pac::$TIMX, 1>::new(timer),
                    Channel::<pac::$TIMX, 2>::new(timer),
                    Channel::<pac::$TIMX, 3>::new(timer),
                    Channel::<pac::$TIMX, 4>::new(timer),
                )
            }

            pub fn start(mut self, value: Hertz) {
                self.pause();

                // UEV event occours on next overflow
                self.urs_counter_only();
                self.clear_irq();
                
                match self.count_mode {
                    CountMode::Loop => self.set_stopwatch_frequency(value),
                    CountMode::Interrupt => self.set_timeout_interval(value),
                }
                // Generate an update event to force an update of the ARR
                // register. This ensures the first timer cycle is of the
                // specified duration.
                self.apply_freq();

                // Start counter
                self.resume();
            }

            pub fn wait(&mut self) -> Result<(), ()> {
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

                let counter_max = match $cntType {
                    16 => u32::from(u16::MAX),
                    _ => u32::MAX
                };
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
        }}
    }
}

// tim_hal! {
//     pac::TIM1: (tim1, Tim1, u16, 16, DIR: cms, BDTR: bdtr, enabled, af1, clear_bit, clear_bit),
//     pac::TIM2: (tim2, Tim2, u32, 32, DIR: cms),
//     pac::TIM3: (tim3, Tim3, u16, 16, DIR: cms),
//     pac::TIM4: (tim4, Tim4, u16, 16, DIR: cms),
//     pac::TIM5: (tim5, Tim5, u32, 32, DIR: cms),
//     pac::TIM8: (tim8, Tim8, u16, 16, DIR: cms, BDTR: bdtr, enabled, af1, clear_bit, clear_bit),
// }
// #[cfg(feature = "rm0468")]
// tim_hal! {
//     pac::TIM23: (tim23, Tim23, u32, 32, DIR: cms),
//     pac::TIM24: (tim24, Tim24, u32, 32, DIR: cms),
// }
// tim_hal! {
//     pac::TIM12: (tim12, Tim12, u16, 16),
//     pac::TIM13: (tim13, Tim13, u16, 16),
//     pac::TIM14: (tim14, Tim14, u16, 16),
// }
// tim_hal! {
//     pac::TIM15: (tim15, Tim15, u16, 16, BDTR: bdtr, set_bit, af1, set_bit),
//     pac::TIM16: (tim16, Tim16, u16, 16, BDTR: bdtr, set_bit, tim16_af1, set_bit),
//     pac::TIM17: (tim17, Tim17, u16, 16, BDTR: bdtr, set_bit, tim17_af1, set_bit),
// }

// Advanced-control
make_timer!(pac::TIM1, 4, u16);
make_timer!(2, 4);
make_timer!(3, 4);
make_timer!(4, 4);
make_timer!(5, 4);
make_timer!(6, 4);
make_timer!(7, 4);
make_timer!(8, 4);
make_timer!(12, 4);
make_timer!(13, 4);
make_timer!(14, 4);
make_timer!(15, 4);
make_timer!(16, 4);
make_timer!(17, 4);
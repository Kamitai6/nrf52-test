//! Timers
//!
//! # Examples
//!
//! - [Blinky using a Timer](https://github.com/stm32-rs/stm32h7xx-hal/blob/master/examples/blinky_timer.rs)
//! - [64 bit microsecond timer](https://github.com/stm32-rs/stm32h7xx-hal/blob/master/examples/tick_timer.rs)

// TODO: on the h7x3 at least, only TIM2, TIM3, TIM4, TIM5 can support 32 bits.
// TIM1 is 16 bit.

use core::marker::PhantomData;

// use crate::stm32::rcc::{d2ccip2r as ccip2r, d3ccipr as srdccipr};

use crate::pac;
use super::rcc::{rec, CoreClocks, ResetEnable};

/// Timer Events
///
/// Each event is a possible interrupt source, if enabled
pub enum Event {
    /// Timer timed out / count down ended
    TimeOut,
}

pub enum CntType {
    U16,
    U32,
}

/// Hardware timers
#[derive(Debug)]
pub struct Timer<const N: u8> {
    cnt_type: CntType,
    clk: u32,
    regs_ptr: *const pac::tim1::RegisterBlock,
}

impl<const N: u8> Timer<N> {
    fn new(frequency or timeout) -> ! {
        let cnt_type = match N {
            2|5 => CntType::U32,
            _ => CntType::U16,
        };

        let regs_ptr: *const pac::tim1::RegisterBlock = match N {
            1 => pac::TIM1::ptr(),
            2 => pac::TIM2::ptr(),
            3 => pac::TIM3::ptr(),
            4 => pac::TIM4::ptr(),
            5 => pac::TIM5::ptr(),
            6 => pac::TIM6::ptr(),
            7 => pac::TIM7::ptr(),
            8 => pac::TIM8::ptr(),
            12 => pac::TIM12::ptr(),
            13 => pac::TIM13::ptr(),
            14 => pac::TIM14::ptr(),
            15 => pac::TIM15::ptr(),
            16 => pac::TIM16::ptr(),
            17 => pac::TIM17::ptr(),
            _ => panic!("Unsupported TIM number"),
        };

        // enable and reset peripheral to a clean state
        let _ = prec.enable().reset(); // drop, can be recreated by free method

        // let clk = (*regs_ptr)::get_clk(clocks)
        // clocks.$ckX()

        let myself = Self {
            cnt_type,
            clk,
            regs_ptr,
        };
    }

    fn start() {
        let mut timer = Timer::$timX(self, prec, clocks);

        timer.pause();

        // UEV event occours on next overflow
        timer.urs_counter_only();
        timer.clear_irq();
        
        match {
            // Set PSC and ARR
            timer.set_tick_freq(frequency);
            // Set PSC and ARR
            self.set_freq(timeout.into());
        }
        // Generate an update event to force an update of the ARR
        // register. This ensures the first timer cycle is of the
        // specified duration.
        timer.apply_freq();

        // Start counter
        timer.resume();

        timer
    }

    fn wait(&mut self) -> nb::Result<(), Void> {
        if self.is_irq_clear() {
            Err(nb::Error::WouldBlock)
        } else {
            self.clear_irq();
            Ok(())
        }
    }

    /// Configures the timer's frequency and counter reload value
    /// so that it underflows at the timeout's frequency
    pub fn set_freq(&mut self, timeout: Hertz) {
        let ticks = self.clk / timeout.raw();

        self.set_timeout_ticks(ticks);
    }

    /// Sets the timer period from a time duration
    ///
    /// ```
    /// use stm32h7xx_hal::time::MilliSeconds;
    ///
    /// // Set timeout to 100ms
    /// let timeout = MilliSeconds::from_ticks(100).into_rate();
    /// timer.set_timeout(timeout);
    /// ```
    ///
    /// Alternatively, the duration can be set using the
    /// core::time::Duration type
    ///
    /// ```
    /// let duration = core::time::Duration::from_nanos(2_500);
    ///
    /// // Set timeout to 2.5µs
    /// timer.set_timeout(duration);
    /// ```
    pub fn set_timeout<T>(&mut self, timeout: T)
    where
        T: Into<core::time::Duration>
    {
        const NANOS_PER_SECOND: u64 = 1_000_000_000;
        let timeout = timeout.into();

        let clk = self.clk as u64;
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
        let (psc, arr) = calculate_timeout_ticks_register_values(ticks);
        self.tim.psc.write(|w| w.psc().bits(psc));
        #[allow(unused_unsafe)] // method is safe for some timers
        self.tim.arr.write(|w| unsafe { w.bits(u32(arr)) });
    }

    /// Configures the timer to count up at the given frequency
    ///
    /// Counts from 0 to the counter's maximum value, then repeats.
    /// Because this only uses the timer prescaler, the frequency
    /// is rounded to a multiple of the timer's kernel clock.
    pub fn set_tick_freq(&mut self, frequency: Hertz) {
        let div = self.clk / frequency.raw();

        let psc = u16(div - 1).unwrap();
        self.tim.psc.write(|w| w.psc().bits(psc));

        let counter_max = u32(<$cntType>::MAX);
        #[allow(unused_unsafe)] // method is safe for some timers
        self.tim.arr.write(|w| unsafe { w.bits(counter_max) });
    }

    /// Applies frequency/timeout changes immediately
    ///
    /// The timer will normally update its prescaler and auto-reload
    /// value when its counter overflows. This function causes
    /// those changes to happen immediately. Also clears the counter.
    pub fn apply_freq(&mut self) {
        self.tim.egr.write(|w| w.ug().set_bit());
    }

    /// Pauses the TIM peripheral
    pub fn pause(&mut self) {
        self.tim.cr1.modify(|_, w| w.cen().clear_bit());
    }

    /// Resume (unpause) the TIM peripheral
    pub fn resume(&mut self) {
        self.tim.cr1.modify(|_, w| w.cen().set_bit());
    }

    /// Set Update Request Source to counter overflow/underflow only
    pub fn urs_counter_only(&mut self) {
        self.tim.cr1.modify(|_, w| w.urs().counter_only());
    }

    /// Reset the counter of the TIM peripheral
    pub fn reset_counter(&mut self) {
        self.tim.cnt.reset();
    }

    /// Read the counter of the TIM peripheral
    pub fn counter(&self) -> u32 {
        self.tim.cnt.read().cnt().bits().into()
    }

    /// Start listening for `event`
    pub fn listen(&mut self, event: Event) {
        match event {
            Event::TimeOut => {
                // Enable update event interrupt
                self.tim.dier.write(|w| w.uie().set_bit());
            }
        }
    }

    /// Stop listening for `event`
    pub fn unlisten(&mut self, event: Event) {
        match event {
            Event::TimeOut => {
                // Disable update event interrupt
                self.tim.dier.write(|w| w.uie().clear_bit());
                let _ = self.tim.dier.read();
                let _ = self.tim.dier.read(); // Delay 2 peripheral clocks
            }
        }
    }

    /// Check if Update Interrupt flag is cleared
    pub fn is_irq_clear(&mut self) -> bool {
        self.tim.sr.read().uif().bit_is_clear()
    }

    /// Clears interrupt flag
    pub fn clear_irq(&mut self) {
        self.tim.sr.modify(|_, w| {
            // Clears timeout event
            w.uif().clear_bit()
        });
        let _ = self.tim.sr.read();
        let _ = self.tim.sr.read(); // Delay 2 peripheral clocks
    }

    /// Releases the TIM peripheral
    pub fn free(mut self) -> ($TIMX, rec::$Rec) {
        // pause counter
        self.pause();

        (self.tim, rec::$Rec { _marker: PhantomData })
    }

    /// Returns a reference to the inner peripheral
    pub fn inner(&self) -> &$TIMX {
        &self.tim
    }

    /// Returns a mutable reference to the inner peripheral
    pub fn inner_mut(&mut self) -> &mut $TIMX {
        &mut self.tim
    }
}

/// We want to have `ticks` amount of timer ticks before it reloads.
/// But `ticks` may have a higher value than what the timer can hold directly.
/// So we'll use the prescaler to extend the range.
///
/// To know how many times we would overflow with a prescaler of 1, we divide `ticks` by 2^16 (the max amount of ticks per overflow).
/// If the result is e.g. 3, then we need to increase our range by 4 times to fit all the ticks.
/// We can increase the range enough by setting the prescaler to 3 (which will divide the clock freq by 4).
/// Because every tick is now 4x as long, we need to divide `ticks` by 4 to keep the same timeout.
///
/// This function returns the prescaler register value and auto reload register value.
fn calculate_timeout_ticks_register_values(ticks: u32) -> (u16, u16) {
    // Note (unwrap): Never panics because 32-bit value is shifted right by 16 bits,
    // resulting in a value that always fits in 16 bits.
    let psc = u16(ticks / (1 << 16)).unwrap();
    // Note (unwrap): Never panics because the divisor is always such that the result fits in 16 bits.
    // Also note that the timer counts `0..=arr`, so subtract 1 to get the correct period.
    let arr = u16(ticks / (u32(psc) + 1)).unwrap().saturating_sub(1);
    (psc, arr)
}

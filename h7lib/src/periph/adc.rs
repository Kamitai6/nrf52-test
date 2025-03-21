//! ADC
//! new関数だけは、実行時オーバーヘッドを許可する
//! 基本的な機能は実装する
//! 読みやすく編集しやすいコードを目指す

use core::ptr;
use cortex_m::{asm, delay::Delay};

use super::{dma, gpio, rcc, pwr};
use crate::{pac, rcc_en_reset, Hertz};

// Address of the ADCinterval voltage reference. This address is found in the User manual. It appears
// to be the same for most STM32s. The voltage this is measured at my vary by variant; eg 3.0 vice 3.3.
// So far, it seems it's always on ADC1, but the channel depends on variant.
// G474 manual implies you can use *any* ADC on ch 18. G491 shows ADC 1 and 3, ch 18 on both.
// L4x2 implies ADC1 only.
mod constants {
    use super::ChannelNum;

    pub const VREFINT_ADDR: u32 = 0x1FF1_E860;
    pub const VREFINT_VOLTAGE: f32 = 3.3;
    pub const VREFINT_CHANNEL: ChannelNum = ChannelNum::C19;
    pub const MAX_ADVREGEN_STARTUP_US: u32 = 10;
}

#[derive(Clone, Copy)]
#[repr(u8)]
/// Select a trigger. Sets CFGR reg, EXTSEL field. See G4 RM, table 163: ADC1/2 - External
/// triggers for regular channels.
pub enum Trigger {
    Tim1Cc1   = 0,
    Tim1Cc2   = 1,
    Tim1Cc3   = 2,
    Tim2Cc2   = 3,
    Tim3Trgo  = 4,
    Tim4Cc4   = 5,
    Exti11    = 6,
    Tim8Trgo  = 7,
    Tim8Trgo2 = 8,
    Tim1Trgo  = 9,
    Tim1Trgo2 = 10,
    Tim2Trgo  = 11,
    Tim4Trgo  = 12,
    Tim6Trgo  = 13,
    Tim15Trgo = 14,
    Tim3Cc4   = 15,
    Tim7Trgo  = 30,
}

#[derive(Clone, Copy)]
#[repr(u8)]
/// Select a trigger. Sets CFGR reg, EXTEN field. See G4 RM, table 161:
/// Configuring the trigger polarity for regular external triggers
/// (Also applies for injected)
pub enum TriggerEdge {
    Software        = 0,
    HardwareRising  = 1,
    HardwareFalling = 2,
    HardwareBoth    = 3,
}

#[derive(Copy, Clone)]
#[repr(u8)]
/// ADC interrupts. See L44 RM, section 16.5: ADC interrupts. Set in the IER register, and cleared
/// in the ISR register.
pub enum AdcInterrupt {
    /// ADC ready (ADRDYIE field)
    Ready = 0,
    /// End of regular conversion interrupt enable (EOCIE field)
    EndOfConversion,
    /// End of regular sequence of conversions (EOSIE field)
    EndOfSequence,
    /// End of injected conversion (JEOCIE field)
    EndofConversionInjected,
    /// End of injected sequence of conversions (JEOSIE field)
    EndOfSequenceInjected,
    /// Analog watchdog 1 interrupt (AWD1IE field)
    Watchdog1,
    /// Analog watchdog 2 interrupt (AWD2IE field)
    Watchdog2,
    /// Analog watchdog 3 interrupt (AWD3IE field)
    Watchdog3,
    /// End of sampling flag interrupt enable for regular conversions (EOSMPIE field)
    EndOfSamplingPhase,
    /// Overrun (OVRIE field)
    Overrun,
    /// Injected Context Queue Overflow (JQOVFIE field)
    InjectedOverflow,
}

// todo: Adc sampling time below depends on the STM32 family. Eg the numbers below
// todo are wrong for L4, but the idea is the same.
/// ADC sampling time. Sets ADC_SMPRx register, SMPy field.
///
/// Each channel can be sampled with a different sample time.
/// There is always an overhead of 13 ADC clock cycles.
/// E.g. For Sampletime T_19 the total conversion time (in ADC clock cycles) is
/// 13 + 19 = 32 ADC Clock Cycles
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum SampleTime {
    T1_5 = 0,
    T2_5,
    T4_5,
    T7_5,
    T19_5,
    T61_5,
    T181_5,
    T601_5,
}

impl Default for SampleTime {
    /// T_1 is the reset value; pick a higher one, as the lower values may cause significantly
    /// lower-than-accurate readings.
    fn default() -> Self {
        SampleTime::T181_5
    }
}

#[derive(Clone, Copy)]
#[repr(u8)]
/// Select single-ended, or differential inputs. Sets bits in the ADC[x]_DIFSEL register.
pub enum InputType {
    SingleEnded = 0,
    Differential = 1,
}

#[derive(Clone, Copy)]
#[repr(u8)]
/// ADC operation mode
pub enum OperationMode {
    /// OneShot Mode
    OneShot = 0,
    Continuous = 1,
}

/// ADC data register alignment
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Align {
    NoShift = 0,
    L1 = 1,
    L2 = 2,
    L3 = 3,
    L4 = 4,
    L5 = 5,
    L6 = 6,
    L7 = 7,
    L8 = 8,
    L9 = 9,
    L10 = 10,
    L11 = 11,
    L12 = 12,
    L13 = 13,
    L14 = 14,
    L15 = 15,
}

impl Default for Align {
    fn default() -> Self {
        Align::NoShift
    }
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum ChannelNum {
    C0 = 0,
    C1,
    C2,
    C3,
    C4,
    C5,
    C6,
    C7,
    C8,
    C9,
    C10,
    C11,
    C12,
    C13,
    C14,
    C15,
    C16,
    C17,
    C18,
    C19,
}

/// ADC data register alignment
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Rank {
    R1 = 0,
    R2,
    R3,
    R4,
    R5,
    R6,
    R7,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
    R16,
}

// todo impl
// /// Ratio options for oversampling.
// #[derive(Clone, Copy)]
// #[repr(u8)]
// pub enum OversamplingRatio {
//     Times2 = 0b000,
//     Times4 = 0b001,
//     Times8 = 0b010,
//     Times16 = 0b011,
//     Times32 = 0b100,
//     Times64 = 0b101,
//     Times128 = 0b110,
//     Times256 = 0b111,
// }
//
// /// Shift options for oversampling.
// #[derive(Clone, Copy)]
// #[repr(u8)]
// pub enum OversamplingShift {
//     None = 0b0000,
//     Bits1 = 0b0001,
//     Bits2 = 0b0010,
//     Bits3 = 0b0011,
//     Bits4 = 0b0100,
//     Bits5 = 0b0101,
//     Bits6 = 0b0110,
//     Bits7 = 0b0111,
//     Bits8 = 0b1000,
// }

/// Initial configuration data for the ADC peripheral.
#[derive(Clone)]
pub struct ChannelCfg {
    pub sample_time: SampleTime,
    pub conversion_rank: Rank,
    pub input_type: InputType,
}

impl Default for ChannelCfg {
    fn default() -> Self {
        Self {
            sample_time: SampleTime::T181_5,
            conversion_rank: Rank::R1,
            input_type: InputType::SingleEnded,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Channel<const ADC_N: u8> {
    pub(super) number: ChannelNum,
}

impl<const ADC_N: u8> Channel<ADC_N> {
    pub fn init<const PORT: char, const PIN: u8>(
        channel_num: ChannelNum, adc_pin: gpio::Gpio<PORT, PIN>, cfg: ChannelCfg
    ) -> Self {
        assert!(matches!(adc_pin.mode, gpio::PinMode::Analog), "Mode is not Analog");

        assert!(
            match ADC_N {
                1 => match PORT {
                    'A' => PIN <= 7,
                    'B' => PIN <= 1,
                    'C' => PIN <= 5,
                    'F' => 11 <= PIN && PIN <= 12,
                    _ => false,
                },
                2 => match PORT {
                    'A' => 2 <= PIN && PIN <= 7,
                    'B' => PIN <= 1,
                    'C' => PIN <= 5,
                    'F' => 13 <= PIN && PIN <= 14,
                    _ => false,
                },
                3 => match PORT {
                    'C' => PIN <= 2,
                    'F' => 3 <= PIN && PIN <= 10,
                    'H' => 2 <= PIN && PIN <= 5,
                    _ => false,
                },
                _ => false,
            }
        );

        let myself = Self {
            number: channel_num,
        };

        Self::set_sample_time(&myself, cfg.sample_time);
        Self::set_sequence(&myself, cfg.conversion_rank);
        Self::set_input_type(&myself, cfg.input_type);

        myself
    }

    pub fn set_sample_time(&self, sample_time: SampleTime) {
        let periph = unsafe { &(*get_periph_regs::<ADC_N>())};
        let channel_u8 = self.number as u8;

        if channel_u8 < 10 {
            periph.smpr1.modify(|r, w| unsafe {
                w.bits((r.bits() & !(0b111 << (channel_u8 * 3))) | ((sample_time as u32) << (channel_u8 * 3)))
            })
        } else {
            periph.smpr2.modify(|r, w| unsafe {
                w.bits((r.bits() & !(0b111 << ((channel_u8 % 10) * 3))) | ((sample_time as u32) << ((channel_u8 % 10) * 3)))
            })
        }
    }

    pub fn get_sample_time(&self) -> SampleTime {
        let periph = unsafe { &(*get_periph_regs::<ADC_N>())};
        let channel_u8 = self.number as u8;

        let time_bit = {
            if channel_u8 < 10 {
                periph.smpr1.read().bits() & !(0b111 << (channel_u8 * 3))
            } else {
                periph.smpr2.read().bits() & !(0b111 << ((channel_u8 % 10) * 3))
            }
        };

        match u8::try_from(time_bit).expect("time-bit is not masked") {
            0 => SampleTime::T1_5,
            1 => SampleTime::T2_5,
            2 => SampleTime::T4_5,
            3 => SampleTime::T7_5,
            4 => SampleTime::T19_5,
            5 => SampleTime::T61_5,
            6 => SampleTime::T181_5,
            7 => SampleTime::T601_5,
            _ => panic!("The time bit should be 0-7")
        }
    }

    pub fn set_sequence(&self, rank: Rank) {
        let periph = unsafe { &(*get_periph_regs::<ADC_N>())};
        match rank {
            Rank::R1 => periph.sqr1.modify(|_, w| unsafe { w.sq1().bits(self.number as u8) }),
            Rank::R2 => periph.sqr1.modify(|_, w| unsafe { w.sq2().bits(self.number as u8) }),
            Rank::R3 => periph.sqr1.modify(|_, w| unsafe { w.sq3().bits(self.number as u8) }),
            Rank::R4 => periph.sqr1.modify(|_, w| unsafe { w.sq4().bits(self.number as u8) }),
            Rank::R5 => periph.sqr2.modify(|_, w| unsafe { w.sq5().bits(self.number as u8) }),
            Rank::R6 => periph.sqr2.modify(|_, w| unsafe { w.sq6().bits(self.number as u8) }),
            Rank::R7 => periph.sqr2.modify(|_, w| unsafe { w.sq7().bits(self.number as u8) }),
            Rank::R8 => periph.sqr2.modify(|_, w| unsafe { w.sq8().bits(self.number as u8) }),
            Rank::R9 => periph.sqr2.modify(|_, w| unsafe { w.sq9().bits(self.number as u8) }),
            Rank::R10 => periph.sqr3.modify(|_, w| unsafe { w.sq10().bits(self.number as u8) }),
            Rank::R11 => periph.sqr3.modify(|_, w| unsafe { w.sq11().bits(self.number as u8) }),
            Rank::R12 => periph.sqr3.modify(|_, w| unsafe { w.sq12().bits(self.number as u8) }),
            Rank::R13 => periph.sqr3.modify(|_, w| unsafe { w.sq13().bits(self.number as u8) }),
            Rank::R14 => periph.sqr3.modify(|_, w| unsafe { w.sq14().bits(self.number as u8) }),
            Rank::R15 => periph.sqr4.modify(|_, w| unsafe { w.sq15().bits(self.number as u8) }),
            Rank::R16 => periph.sqr4.modify(|_, w| unsafe { w.sq16().bits(self.number as u8) })
        }

        periph.pcsel.modify(|r, w| unsafe { w.pcsel().bits(r.pcsel().bits() | (1 << (self.number as u8))) });
    }

    pub fn set_input_type(&self, input_type: InputType) {
        let periph = unsafe { &(*get_periph_regs::<ADC_N>())};

        // Note that we don't use the `difsel` PAC accessor here, due to its varying
        // implementations across different PACs.
        // todo: 1 offset? Experiment in firmware.
        let val = periph.difsel.read().bits();

        let val_new = match input_type {
            InputType::SingleEnded => val & !(1 << (self.number as u8)),
            InputType::Differential => val | (1 << (self.number as u8)),
        };
        periph.difsel.write(|w| unsafe { w.bits(val_new)});
    }
}

/// Initial configuration data for the ADC peripheral.
#[derive(Clone)]
pub struct Config {
    pub clock: Hertz,
    /// One-shot, or continuous measurements. Defaults to one-shot.
    pub operation_mode: OperationMode,

    cal_single_ended: Option<u16>,
    cal_differential: Option<u16>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            clock: Hertz::from_raw(0),
            operation_mode: OperationMode::OneShot,
            cal_single_ended: None,
            cal_differential: None,
        }
    }
}

pub struct Adc<'c, const N: u8> {
    pub cfg: Config,
    vdda_calibrated: f32,
    delay_base_freq: u32,
    pub channels: &'c [Option<Channel<N>>; 20],
}

impl<'c, const N: u8> Adc<'c, N> {
    const CHECK: () = {
        assert!(1 <= N && N <= 3, "Adc must be 1 - 3.");
    };
    pub fn new(cfg: Config, clocks: &rcc::Rcc, channels: &'c [Option<Channel<N>>; 20]) -> Self {
        let _ = Self::CHECK;

        let some_count = channels.iter().filter(|option| option.is_some()).count() as u8;

        let mut myself = Self {
            cfg,
            vdda_calibrated: 0.,
            delay_base_freq: clocks.sys_ck.raw(),
            channels,
        };

        let rcc = unsafe { &(*pac::RCC::ptr()) };
        let periph = unsafe { &(*get_periph_regs::<N>())};

        match N {
            1 | 2 => rcc_en_reset!(ahb1, adc12, rcc),
                3 => rcc_en_reset!(ahb4, adc3, rcc),
            _ => unreachable!(),
        }

        Self::set_clock(&myself.cfg.clock, clocks);

        Self::set_align(&myself, Align::default());
        Self::advregen_enable(&mut myself);
        Self::calibrate(&mut myself, InputType::SingleEnded);
        Self::calibrate(&mut myself, InputType::Differential);

        #[cfg(all(not(any(feature = "h743", feature = "h753"))))]
        periph.cr.modify(|_, w| w.boost().bits(1));
        #[cfg(any(feature = "h743", feature = "h753"))]
        periph.cr.modify(|_, w| w.boost().bit(true));

        Self::enable(&mut myself);
        Self::setup_vdda(&mut myself);

        // Don't set continuous mode until after configuring VDDA, since it needs
        // to take a oneshot reading.
        periph.cfgr.modify(|_, w| w.cont().bit(myself.cfg.operation_mode as u8 != 0));

        Self::set_sequence_len(&mut myself, some_count);

        myself
    }

    /// Take a single reading; return a raw integer value.
    pub fn read_raw<const C_LEN: usize>(&self, channel_num: &[ChannelNum; C_LEN]) -> [u16; C_LEN] {
        let periph = unsafe { &(*get_periph_regs::<N>())};
        
        // After the regular sequence is complete, after each conversion is complete,
        // the EOC (end of regular conversion) flag is set.
        // After the regular sequence is complete: The EOS (end of regular sequence) flag is set.
        
        self.start_conversion();
        while periph.isr.read().eos().bit_is_clear() {}  // wait until complete.

        let mut adc_values = [0u16; C_LEN];
        for i in 0..C_LEN {
            periph.pcsel.modify(|r, w| unsafe { w.pcsel().bits(r.pcsel().bits() & !(1 << channel_num[i] as u8)) });
            adc_values[i] = periph.dr.read().rdata().bits();
        }
        adc_values
    }

    /// Take a single reading; return a voltage.
    pub fn read<const C_LEN: usize>(&self, channel_num: &[ChannelNum; C_LEN]) -> [f32; C_LEN] {
        let reading = self.read_raw(channel_num);
        // RM:
        // Converting a supply-relative ADC measurement to an absolute voltage value
        // The ADC is designed to deliver a digital value corresponding to the ratio between the analog
        // power supply and the voltage applied on the converted channel. For most application use
        // cases, it is necessary to convert this ratio into a voltage independent of VDDA. For
        // applications where VDDA is known and ADC converted values are right-aligned you can use
        // the following formula to get this absolute value:

        // V_CHANNELx = V_DDA / FULL_SCALE x ADCx_DATA

        // Where:
        // • VREFINT_CAL is the VREFINT calibration value
        // • ADC_DATA is the value measured by the ADC on channel x (right-aligned)
        // • VREFINT_DATA is the actual VREFINT output value converted by the ADC
        // • FULL_SCALE is the maximum digital value of the ADC output. For example with 12-bit
        // resolution, it will be 212 − 1 = 4095 or with 8-bit resolution, 28 − 1 = 255
        // todo: FULL_SCALE will be different for 16-bit. And differential?

        let mut adc_values = [0.; C_LEN];
        for i in 0..C_LEN {
            adc_values[i] = self.vdda_calibrated / 4_096. * reading[i] as f32;
        }
        adc_values
    }

    pub fn enable(&self) {
        let periph = unsafe { &(*get_periph_regs::<N>())};

        // 1. Clear the ADRDY bit in the ADC_ISR register by writing ‘1’.
        periph.isr.modify(|_, w| w.adrdy().set_bit());
        // 2. Set ADEN=1.
        periph.cr.modify(|_, w| w.aden().set_bit());  // Enable
        // 3. Wait until ADRDY=1 (ADRDY is set after the ADC startup time). This can be done
        // using the associated interrupt (setting ADRDYIE=1).
        while periph.isr.read().adrdy().bit_is_clear() {}  // Wait until ready
        // 4. Clear the ADRDY bit in the ADC_ISR register by writing ‘1’ (optional).
        periph.isr.modify(|_, w| w.adrdy().set_bit());
    }

    pub fn disable(&self) {
        let periph = unsafe { &(*get_periph_regs::<N>())};

        // 1. Check that both ADSTART=0 and JADSTART=0 to ensure that no conversion is
        // ongoing. If required, stop any regular and injected conversion ongoing by setting
        // ADSTP=1 and JADSTP=1 and then wait until ADSTP=0 and JADSTP=0.
        self.stop_conversions();

        // 2. Set ADDIS=1.
        periph.cr.modify(|_, w| w.addis().set_bit()); // Disable

        // 3. If required by the application, wait until ADEN=0, until the analog
        // ADC is effectively disabled (ADDIS will automatically be reset once ADEN=0)
        while periph.cr.read().aden().bit_is_set() {}
    }

    pub fn is_enabled(&self) -> bool {
        let periph = unsafe { &(*get_periph_regs::<N>())};
        periph.cr.read().aden().bit_is_set()
    }

    pub fn advregen_enable(&mut self) {
        let periph = unsafe { &(*get_periph_regs::<N>())};
        // L443 RM, 16.4.6; G4 RM, section 21.4.6: Deep-power-down mode (DEEPPWD) and ADC voltage
        // regulator (ADVREGEN)
        //
        // "By default, the ADC is in Deep-power-down mode where its supply is internally switched off
        // to reduce the leakage currents (the reset state of bit DEEPPWD is 1 in the ADC_CR
        // register).
        // To start ADC operations, it is first needed to exit Deep-power-down mode by setting bit
        // DEEPPWD=0.""
        periph.cr.modify(|_, w| {
            w.deeppwd().clear_bit();   // Exit deep sleep mode.
            w.advregen().set_bit()   // Enable voltage regulator.

        });

        self.wait_advregen_startup();
    }

    pub fn advregen_disable(&mut self) {
        let periph = unsafe { &(*get_periph_regs::<N>())};
        // L4 RM, 16.4.6: Writing DEEPPWD=1 automatically disables the ADC voltage
        // regulator and bit ADVREGEN is automatically cleared.
        // When the internal voltage regulator is disabled (ADVREGEN=0), the internal analog
        // calibration is kept.
        // In ADC Deep-power-down mode (DEEPPWD=1), the internal analog calibration is lost and
        // it is necessary to either relaunch a calibration or re-apply the calibration factor which was
        // previously saved (
        periph.cr.modify(|_, w| w.deeppwd().set_bit());
        // todo: We could offer an option to disable advregen without setting deeppwd,
        // todo, which would keep calibration.
    }

    pub fn is_advregen_enabled(&self) -> bool {
        let periph = unsafe { &(*get_periph_regs::<N>())};
        periph.cr.read().advregen().bit_is_set()
    }

    fn wait_advregen_startup(&self) {
        let cp = unsafe { cortex_m::Peripherals::steal() };
        let mut delay = Delay::new(cp.SYST, self.delay_base_freq);
        delay.delay_us(constants::MAX_ADVREGEN_STARTUP_US);
    }

    /// Calibrate. See L4 RM, 16.5.8, or F404 RM, section 15.3.8.
    /// Stores calibration values, which can be re-inserted later,
    /// eg after entering ADC deep sleep mode, or MCU STANDBY or VBAT.
    pub fn calibrate(&mut self, input_type: InputType) {
        let periph = unsafe { &(*get_periph_regs::<N>())};

        // 1. Ensure DEEPPWD=0, ADVREGEN=1 and that ADC voltage regulator startup time has
        // elapsed.
        if !self.is_advregen_enabled() {
            self.advregen_enable();
        }

        let was_enabled = self.is_enabled();
        // Calibration can only be initiated when the ADC is disabled (when ADEN=0).
        // 2. Ensure that ADEN=0
        if was_enabled {
            self.disable();
        }

        periph.cr.modify(|_, w| w
            // RM:
            // The calibration factor to be applied for single-ended input conversions is different from the
            // factor to be applied for differential input conversions:
            // • Write ADCALDIF=0 before launching a calibration which will be applied for singleended input conversions.
            // • Write ADCALDIF=1 before launching a calibration which will be applied for differential
            // input conversions.
            // 3. Select the input mode for this calibration by setting ADCALDIF=0 (single-ended input)
            // or ADCALDIF=1 (differential input).
            .adcaldif().bit(input_type as u8 != 0)
            // The calibration is then initiated by software by setting bit ADCAL=1.
            // 4. Set ADCAL=1.
            .adcal().set_bit()); // start calibration.

        // ADCAL bit stays at 1 during all the
        // calibration sequence. It is then cleared by hardware as soon the calibration completes. At
        // this time, the associated calibration factor is stored internally in the analog ADC and also in
        // the bits CALFACT_S[6:0] or CALFACT_D[6:0] of ADC_CALFACT register (depending on
        // single-ended or differential input calibration)
        // 5. Wait until ADCAL=0.
        while periph.cr.read().adcal().bit_is_set() {}

        // 6. The calibration factor can be read from ADC_CALFACT register.
        match input_type {
            InputType::SingleEnded => {
                let val = periph.calfact.read().calfact_s().bits();
                self.cfg.cal_single_ended = Some(val);
            }
            InputType::Differential => {
                let val = periph.calfact.read().calfact_d().bits();
                self.cfg.cal_differential = Some(val);
            }
        }

        if was_enabled {
            self.enable();
        }
    }

    /// Insert a previously-saved calibration value into the ADC.
    /// Se L4 RM, 16.4.8.
    pub fn inject_calibration(&mut self) {
        let periph = unsafe { &(*get_periph_regs::<N>())};

        // 1. Ensure ADEN=1 and ADSTART=0 and JADSTART=0 (ADC enabled and no
        // conversion is ongoing).
        if !self.is_enabled() {
            self.enable();
        }
        self.stop_conversions();

        // 2. Write CALFACT_S and CALFACT_D with the new calibration factors.
        if let Some(cal) = self.cfg.cal_single_ended {
            periph.calfact.modify(|_, w| unsafe { w.calfact_s().bits(cal) });
        }
        if let Some(cal) = self.cfg.cal_differential {
            periph.calfact.modify(|_, w| unsafe { w.calfact_d().bits(cal) });
        }

        // 3. When a conversion is launched, the calibration factor will be injected into the analog
        // ADC only if the internal analog calibration factor differs from the one stored in bits
        // CALFACT_S for single-ended input channel or bits CALFACT_D for differential input
        // channel.
    }

    /// Start a conversion: Either a single measurement, or continuous conversions.
    /// Blocks until the conversion is complete.
    /// See L4 RM 16.4.15 for details.
    pub fn start_conversion(&self) {
        let periph = unsafe { &(*get_periph_regs::<N>())};

        periph.cr.modify(|_, w| w.adstart().set_bit());  // Start
    }

    /// If any conversions are in progress, stop them. This is a step listed in the RMs
    /// for disable, and calibration procedures. See L4 RM: 16.4.17.
    /// When the ADSTP bit is set by software, any ongoing regular conversion is aborted with
    /// partial result discarded (ADC_DR register is not updated with the current conversion).
    /// When the JADSTP bit is set by software, any ongoing injected conversion is aborted with
    /// partial result discarded (ADC_JDRy register is not updated with the current conversion).
    /// The scan sequence is also aborted and reset (meaning that relaunching the ADC would
    /// restart a new sequence).
    pub fn stop_conversions(&self) {
        let periph = unsafe { &(*get_periph_regs::<N>())};

        // The software can decide to stop regular conversions ongoing by setting ADSTP=1 and
        // injected conversions ongoing by setting JADSTP=1.
        // Stopping conversions will reset the ongoing ADC operation. Then the ADC can be
        // reconfigured (ex: changing the channel selection or the trigger) ready for a new operation.
        if periph.cr.read().adstart().bit_is_set() || periph.cr.read().jadstart().bit_is_set() {
            periph.cr.modify(|_, w| {
                w.adstp().set_bit();
                w.jadstp().set_bit()
            });

            while periph.cr.read().adstart().bit_is_set() || periph.cr.read().jadstart().bit_is_set() {}
        }
    }

    pub fn set_sequence_len(&self, len: u8) {
        assert!(1 <= len && len <= 16, "len must be 1 ~ 16");
        let periph = unsafe { &(*get_periph_regs::<N>())};
        periph.sqr1.modify(|_, w| unsafe { w.l().bits(len - 1) });
    }

    /// Find and store the internal voltage reference, to improve conversion from reading
    /// to voltage accuracy. See L44 RM, section 16.4.34: "Monitoring the internal voltage reference"
    fn setup_vdda(&mut self) {
        let common = unsafe { &(*get_common_regs::<N>())};
        // RM: It is possible to monitor the internal voltage reference (VREFINT) to have a reference point for
        // evaluating the ADC VREF+ voltage level.
        // The internal voltage reference is internally connected to the input channel 0 of the ADC1
        // (ADC1_INP0).

        // todo: On H7, you may need to use ADC3 for this...

        // Regardless of which ADC we're on, we take this reading using ADC1.
        self.vdda_calibrated = if N != 1 {
            // todo: What if ADC1 is already enabled and configured differently?
            // todo: Either way, if you're also using ADC1, this will screw things up⋅.

            // let dp = unsafe { pac::Peripherals::steal() };
            //
            // #[cfg(feature = "l5")]
            // let dp_adc = dp.ADC;
            // #[cfg(not(feature = "l5"))]
            // let dp_adc = dp.ADC1;

            // If we're currently using ADC1 (and this is a different ADC), skip this step for now;
            // VDDA will be wrong,
            // and all readings using voltage conversion will be wrong.
            // todo: Take an ADC1 reading if this is the case, or let the user pass in VDDA from there.
            // if dp_adc.cr.read().aden().bit_is_set() {
            //     self.vdda_calibrated = 3.3; // A guess.
            //     return
            // }

            // todo: Get this working.
            // let mut adc1 = Adc::new_adc1(
            //     dp_adc,
            //     AdcDevice::One,
            //     // We use self cfg, in case ADC1 is on the same common regs as this; we don't
            //     // want it overwriting prescaler and clock cfg.
            //     self.cfg.clone(),
            // );
            // adc1.disable();

            // This fn will be called recursively for ADC1, generating the vdda value we need.
            // adc1.vdda_calibrated
            3.3
        }
        else {
            // "Table 24. Embedded internal voltage reference" states that the sample time needs to be
            // at a minimum 4 us. With 640.5 ADC cycles we have a minimum of 8 us at 80 MHz, leaving
            // some headroom.

            common.ccr.modify(|_, w| w.vrefen().set_bit());
            // User manual table: "Embedded internal voltage reference" states that it takes a maximum of 12 us
            // to stabilize the internal voltage reference, we wait a little more.

            // todo: Not sure what to set this delay to and how to change it based on variant, so picking
            // todo something conservative.
            let cp = unsafe { cortex_m::Peripherals::steal() };
            let mut delay = Delay::new(cp.SYST, self.delay_base_freq);
            delay.delay_us(100);

            let pre_sampletime = self.channels[0].unwrap().get_sample_time();

            self.channels[0].unwrap().set_sample_time(SampleTime::T181_5);
            let read_chs = [constants::VREFINT_CHANNEL];
            let reading = self.read(&read_chs);
            self.stop_conversions();

            common.ccr.modify(|_, w| w.vrefen().clear_bit());

            self.channels[0].unwrap().set_sample_time(pre_sampletime);

            // The VDDA power supply voltage applied to the microcontroller may be subject to variation or
            // not precisely known. The embedded internal voltage reference (VREFINT) and its calibration
            // data acquired by the ADC during the manufacturing process at VDDA = 3.0 V can be used to
            // evaluate the actual VDDA voltage level.
            // The following formula gives the actual VDDA voltage supplying the device:
            // VDDA = 3.0 V x VREFINT_CAL / VREFINT_DATA
            // where:
            // • VREFINT_CAL is the VREFINT calibration value
            // • VREFINT_DATA is the actual VREFINT output value converted by ADC

            // todo: This address may be different on different MCUs, even within the same family.
            // Although, it seems relatively consistent. Check User Manuals.
            let vrefint_cal: u16 = unsafe { ptr::read_volatile(&*(constants::VREFINT_ADDR as *const _)) };
            constants::VREFINT_VOLTAGE * vrefint_cal as f32 / reading[0] as f32
        };
    }

    pub fn set_align(&self, align: Align) {
        let periph = unsafe { &(*get_periph_regs::<N>())};
        periph.cfgr2.modify(|_, w| w.lshift().bits(align as u8));
    }

    /// Select and activate a trigger. See G4 RM, section 21.4.18:
    /// Conversion on external trigger and trigger polarit
    pub fn set_trigger(&mut self, trigger: Trigger, edge: TriggerEdge) {
        let periph = unsafe { &(*get_periph_regs::<N>())};
        periph.cfgr.modify(|_, w| unsafe {
            w.exten().bits(edge as u8);
            w.extsel().bits(trigger as u8)
        });
    }

    /// Enable a specific type of ADC interrupt.
    pub fn enable_interrupt(&mut self, interrupt: AdcInterrupt) {
        let periph = unsafe { &(*get_periph_regs::<N>())};
        periph.ier.modify(|_, w| match interrupt {
            AdcInterrupt::Ready => w.adrdyie().set_bit(),
            AdcInterrupt::EndOfConversion => w.eocie().set_bit(),
            AdcInterrupt::EndOfSequence => w.eosie().set_bit(),
            AdcInterrupt::EndofConversionInjected => w.jeocie().set_bit(),
            AdcInterrupt::EndOfSequenceInjected => w.jeosie().set_bit(),
            AdcInterrupt::Watchdog1 => w.awd1ie().set_bit(),
            AdcInterrupt::Watchdog2 => w.awd2ie().set_bit(),
            AdcInterrupt::Watchdog3 => w.awd3ie().set_bit(),
            AdcInterrupt::EndOfSamplingPhase => w.eosmpie().set_bit(),
            AdcInterrupt::Overrun => w.ovrie().set_bit(),
            AdcInterrupt::InjectedOverflow => w.jqovfie().set_bit(),
        });
    }

    /// Clear an interrupt flag of the specified type. Consider running this in the
    /// corresponding ISR.
    pub fn clear_interrupt(&mut self, interrupt: AdcInterrupt) {
        let periph = unsafe { &(*get_periph_regs::<N>())};
        periph.isr.write(|w| match interrupt {
            AdcInterrupt::Ready => w.adrdy().set_bit(),
            AdcInterrupt::EndOfConversion => w.eoc().set_bit(),
            AdcInterrupt::EndOfSequence => w.eos().set_bit(),
            AdcInterrupt::EndofConversionInjected => w.jeoc().set_bit(),
            AdcInterrupt::EndOfSequenceInjected => w.jeos().set_bit(),
            AdcInterrupt::Watchdog1 => w.awd1().set_bit(),
            AdcInterrupt::Watchdog2 => w.awd2().set_bit(),
            AdcInterrupt::Watchdog3 => w.awd3().set_bit(),
            AdcInterrupt::EndOfSamplingPhase => w.eosmp().set_bit(),
            AdcInterrupt::Overrun => w.ovr().set_bit(),
            AdcInterrupt::InjectedOverflow => w.jqovf().set_bit(),
        });
        // match interrupt {
        //     AdcInterrupt::Ready => self.regs.icr.write(|_w| w.adrdy().set_bit()),
        //     AdcInterrupt::EndOfConversion => self.regs.icr.write(|w| w.eoc().set_bit()),
        //     AdcInterrupt::EndOfSequence => self.regs.icr.write(|_w| w.eos().set_bit()),
        //     AdcInterrupt::EndofConversionInjected => self.regs.icr.write(|_w| w.jeoc().set_bit()),
        //     AdcInterrupt::EndOfSequenceInjected => self.regs.icr.write(|_w| w.jeos().set_bit()),
        //     AdcInterrupt::Watchdog1 => self.regs.icr.write(|_w| w.awd1().set_bit()),
        //     AdcInterrupt::Watchdog2 => self.regs.icr.write(|_w| w.awd2().set_bit()),
        //     AdcInterrupt::Watchdog3 => self.regs.icr.write(|_w| w.awd3().set_bit()),
        //     AdcInterrupt::EndOfSamplingPhase => self.regs.icr.write(|_w| w.eosmp().set_bit()),
        //     AdcInterrupt::Overrun => self.regs.icr.write(|_w| w.ovr().set_bit()),
        //     AdcInterrupt::InjectedOverflow => self.regs.icr.write(|_w| w.jqovf().set_bit()),
        // }
    }

    /// Take a reading, using DMA. Sets conversion sequence; no need to set it directly.
    /// Note that the `channel` argument is unused on F3 and L4, since it is hard-coded,
    /// and can't be configured using the DMAMUX peripheral. (`dma::mux()` fn).
    pub unsafe fn read_dma(
        &mut self, 
        buf: &mut [u16],
        dma_channel: dma::DmaChannel,
        dma_channel_cfg: dma::ChannelCfg,
        dma_periph: &mut dma::Dma<1>,
    ) {
        let periph = unsafe { &(*get_periph_regs::<N>())};
        let (ptr, len) = (buf.as_mut_ptr(), buf.len());
        // The software is allowed to write (dmaen and dmacfg) only when ADSTART=0 and JADSTART=0 (which
        // ensures that no conversion is ongoing)
        self.stop_conversions();

        periph.cfgr.modify(|_, w| {
            // Note: To use non-DMA after this has been set, need to configure manually.
            // ie set back to 0b00.
            w.dmngt().bits(if dma_channel_cfg.circular == dma::Circular::Enabled { 0b11 } else { 0b01 })
        });

        periph.cr.modify(|_, w| w.adstart().set_bit());  // Start

        // Since converted channel values are stored into a unique data register, it is useful to use
        // DMA for conversion of more than one channel. This avoids the loss of the data already
        // stored in the ADC_DR register.
        // When the DMA mode is enabled (DMAEN bit set to 1 in the ADC_CFGR register in single
        // ADC mode or MDMA different from 0b00 in dual ADC mode), a DMA request is generated
        // after each conversion of a channel. This allows the transfer of the converted data from the
        // ADC_DR register to the destination location selected by the software.
        // Despite this, if an overrun occurs (OVR=1) because the DMA could not serve the DMA
        // transfer request in time, the ADC stops generating DMA requests and the data
        // corresponding to the new conversion is not transferred by the DMA. Which means that all
        // the data transferred to the RAM can be considered as valid.
        // Depending on the configuration of OVRMOD bit, the data is either preserved or overwritten
        // (refer to Section : ADC overrun (OVR, OVRMOD)).
        // The DMA transfer requests are blocked until the software clears the OVR bit.
        // Two different DMA modes are proposed depending on the application use and are
        // configured with bit DMACFG of the ADC_CFGR register in single ADC mode, or with bit
        // DMACFG of the ADC_CCR register in dual ADC mode:
        // • DMA one shot mode (DMACFG=0).
        // This mode is suitable when the DMA is programmed to transfer a fixed number of data.
        // • DMA circular mode (DMACFG=1)
        // This mode is suitable when programming the DMA in circular mode.


        // In [DMA one shot mode], the ADC generates a DMA transfer request each time a new conversion data
        // is available and stops generating DMA requests once the DMA has reached the last DMA
        // transfer (when DMA_EOT interrupt occurs - refer to DMA paragraph) even if a conversion
        // has been started again.
        // When the DMA transfer is complete (all the transfers configured in the DMA controller have
        // been done):
        // • The content of the ADC data register is frozen.
        // • Any ongoing conversion is aborted with partial result discarded.
        // • No new DMA request is issued to the DMA controller. This avoids generating an
        // overrun error if there are still conversions which are started.
        // • Scan sequence is stopped and reset.
        // • The DMA is stopped.

        let num_data = len as u32;

        dma_periph.cfg_channel(
            dma_channel,
            &periph.dr as *const _ as u32,
            ptr as u32,
            num_data,
            dma::Direction::ReadFromPeriph,
            dma::DataSize::S16,
            dma::DataSize::S16,
            dma_channel_cfg,
        );
    }

    // For pll2_p_ck
    fn set_clock(target: &Hertz, clocks: &rcc::Rcc) {
        let ker_ck = Self::kernel_clk_unwrap(clocks);

        let max_ker_ck = match clocks.vos {
            pwr::VoltageScale::Scale0 | pwr::VoltageScale::Scale1 => 80_000_000,
            pwr::VoltageScale::Scale2 | pwr::VoltageScale::Scale3 => 40_000_000
        };
        assert!(ker_ck.raw() <= max_ker_ck,
                "Kernel clock violates maximum frequency defined in Reference Manual. \
                    Can result in erroneous ADC readings");

        // Target mux output. See RM0433 Rev 7 - Figure 136.
        #[cfg(feature = "revision_v")]
        let f_target = target.raw() * 2;

        #[cfg(not(feature = "revision_v"))]
        let f_target = target.raw();

        use crate::pac::adc3_common::ccr::PRESC_A;
        let (divider, presc) = match ker_ck.raw().div_ceil(f_target) {
            1 => (1, PRESC_A::Div1),
            2 => (2, PRESC_A::Div2),
            3..=4 => (4, PRESC_A::Div4),
            5..=6 => (6, PRESC_A::Div6),
            7..=8 => (8, PRESC_A::Div8),
            9..=10 => (10, PRESC_A::Div10),
            11..=12 => (12, PRESC_A::Div12),
            13..=16 => (16, PRESC_A::Div16),
            17..=32 => (32, PRESC_A::Div32),
            33..=64 => (64, PRESC_A::Div64),
            65..=128 => (128, PRESC_A::Div128),
            129..=256 => (256, PRESC_A::Div256),
            _ => panic!("Selecting the ADC clock required a prescaler > 256, \
                            which is not possible in hardware. Either increase the ADC \
                            clock frequency or decrease the kernel clock frequency"),
        };
        let common = unsafe { &(*get_common_regs::<N>())};
        common.ccr.modify(|_, w| w.presc().variant(presc));

        // Calculate actual value. See RM0433 Rev 7 - Figure 136.
        #[cfg(feature = "revision_v")]
        let f_adc = Hertz::from_raw(ker_ck.raw() / (divider * 2));

        // Calculate actual value Revison Y. See RM0433 Rev 7 - Figure 137.
        #[cfg(not(feature = "revision_v"))]
        let f_adc = Hertz::from_raw(ker_ck.raw() / divider);

        // Maximum ADC clock speed. With BOOST = 0 there is a no
        // minimum frequency given in part datasheets
        assert!(f_adc.raw() <= 50_000_000);
    }

    fn kernel_clk_unwrap(clocks: &rcc::Rcc) -> Hertz {
        let ccipr = unsafe { (*pac::RCC::ptr()).d3ccipr.read() };
        use crate::pac::rcc::d3ccipr::ADCSEL_A;
        match ccipr.adcsel().variant() {
            Some(ADCSEL_A::Pll2P) => {
                clocks.pll2_p_ck.expect("ADC: PLL2_P must be enabled")
            }
            Some(ADCSEL_A::Pll3R) => {
                clocks.pll3_r_ck.expect("ADC: PLL3_R must be enabled")
            }
            Some(ADCSEL_A::Per) => {
                clocks.per_ck.expect("ADC: PER clock must be enabled")
            }
            _ => unreachable!(),
        }
    }
}

const fn get_periph_regs<const N: u8>() -> *const pac::adc3::RegisterBlock {
    match N {
        1 => pac::ADC1::ptr(),
        2 => pac::ADC2::ptr(),
        3 => pac::ADC3::ptr(),
        _ => panic!("Unsupported ADC number"),
    }
}

const fn get_common_regs<const N: u8>() -> *const pac::adc3_common::RegisterBlock {
    match N {
        1 | 2 => pac::ADC12_COMMON::ptr(),
            3 => pac::ADC3_COMMON::ptr(),
        _ => panic!("Unsupported ADC number"),
    }
}
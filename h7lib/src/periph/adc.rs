//! ADC
//! new関数だけは、実行時オーバーヘッドを許可する
//! 基本的な機能は実装する
//! 読みやすく編集しやすいコードを目指す

use core::ptr;
use cortex_m::{asm, delay::Delay};

use super::dma;
use super::gpio;
use crate::pac;

// Address of the ADCinterval voltage reference. This address is found in the User manual. It appears
// to be the same for most STM32s. The voltage this is measured at my vary by variant; eg 3.0 vice 3.3.
// So far, it seems it's always on ADC1, but the channel depends on variant.
// G474 manual implies you can use *any* ADC on ch 18. G491 shows ADC 1 and 3, ch 18 on both.
// L4x2 implies ADC1 only.
mod constants {
    pub const VREFINT_ADDR: u32 = 0x1FF1_E860;
    pub const VREFINT_VOLTAGE: f32 = 3.3;
    pub const VREFINT_CH: u8 = 0; // todo: Unknown. What is it?
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
pub enum Channel {
    C1 = 1,
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

impl Channel {
    pub fn from(num: u8) -> Option<Channel> {
        match num {
            1 => Some(Channel::C1),
            2 => Some(Channel::C2),
            3 => Some(Channel::C3),
            4 => Some(Channel::C4),
            5 => Some(Channel::C5),
            6 => Some(Channel::C6),
            7 => Some(Channel::C7),
            8 => Some(Channel::C8),
            9 => Some(Channel::C9),
            10 => Some(Channel::C10),
            11 => Some(Channel::C11),
            12 => Some(Channel::C12),
            13 => Some(Channel::C13),
            14 => Some(Channel::C14),
            15 => Some(Channel::C15),
            16 => Some(Channel::C16),
            17 => Some(Channel::C17),
            18 => Some(Channel::C18),
            19 => Some(Channel::C19),
            _ => None, // 1〜19以外の値には対応しない
        }
    }
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Position {
    P1 = 1,
    P2,
    P3,
    P4,
    P5,
    P6,
    P7,
    P8,
    P9,
    P10,
    P11,
    P12,
    P13,
    P14,
    P15,
    P16,
}

impl Position {
    pub fn from(num: u8) -> Option<Position> {
        match num {
            1 => Some(Position::P1),
            2 => Some(Position::P2),
            3 => Some(Position::P3),
            4 => Some(Position::P4),
            5 => Some(Position::P5),
            6 => Some(Position::P6),
            7 => Some(Position::P7),
            8 => Some(Position::P8),
            9 => Some(Position::P9),
            10 => Some(Position::P10),
            11 => Some(Position::P11),
            12 => Some(Position::P12),
            13 => Some(Position::P13),
            14 => Some(Position::P14),
            15 => Some(Position::P15),
            16 => Some(Position::P16),
            _ => None, // 1〜16以外の値には対応しない
        }
    }
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
pub enum SequeLen {
    S1 = 1,
    S2,
    S3,
    S4,
    S5,
    S6,
    S7,
    S8,
    S9,
    S10,
    S11,
    S12,
    S13,
    S14,
    S15,
    S16,
}

impl SequeLen {
    pub fn from(num: u8) -> Option<SequeLen> {
        match num {
            1 => Some(SequeLen::S1),
            2 => Some(SequeLen::S2),
            3 => Some(SequeLen::S3),
            4 => Some(SequeLen::S4),
            5 => Some(SequeLen::S5),
            6 => Some(SequeLen::S6),
            7 => Some(SequeLen::S7),
            8 => Some(SequeLen::S8),
            9 => Some(SequeLen::S9),
            10 => Some(SequeLen::S10),
            11 => Some(SequeLen::S11),
            12 => Some(SequeLen::S12),
            13 => Some(SequeLen::S13),
            14 => Some(SequeLen::S14),
            15 => Some(SequeLen::S15),
            16 => Some(SequeLen::S16),
            _ => None, // 1〜16以外の値には対応しない
        }
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

#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
/// ADC Clock mode
/// (L44 RM, Section 16.4.3) The input clock is the same for the three ADCs and can be selected between two different
/// clock sources (see Figure 40: ADC clock scheme):
/// 1. The ADC clock can be a specific clock source. It can be derived from the following
/// clock sources:
/// – The system clock
/// – PLLSAI1 (single ADC implementation)
/// Refer to RCC Section for more information on how to generate ADC dedicated clock.
/// To select this scheme, bits CKMODE[1:0] of the ADCx_CCR register must be reset.
/// 2. The ADC clock can be derived from the AHB clock of the ADC bus interface, divided by
/// a programmable factor (1, 2 or 4). In this mode, a programmable divider factor can be
/// selected (/1, 2 or 4 according to bits CKMODE[1:0]).
/// To select this scheme, bits CKMODE[1:0] of the ADCx_CCR register must be different
/// from “00”.
pub enum ClockMode {
    /// Use Kernel Clock adc_ker_ck_input divided by PRESC. Asynchronous to AHB clock
    Async = 0,
    /// Use AHB clock rcc_hclk3 (or just hclk depending on variant).
    /// "For option 2), a prescaling factor of 1 (CKMODE[1:0]=01) can be used only if the AHB
    /// prescaler is set to 1 (HPRE[3:0] = 0xxx in RCC_CFGR register)."
    SyncDiv1 = 1,
    /// Use AHB clock rcc_hclk3 (or just hclk depending on variant) divided by 2
    SyncDiv2 = 2,
    /// Use AHB clock rcc_hclk3 (or just hclk depending on variant) divided by 4
    SyncDiv4 = 3,
}

/// Sets ADC clock prescaler; ADCx_CCR register, PRESC field.
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Prescaler {
    D1 = 1,
    D2,
    D4,
    D6,
    D8,
    D10,
    D12,
    D16,
    D32,
    D64,
    D128,
    D256,
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
pub struct Config {
    /// ADC clock mode. Defaults to AHB clock rcc_hclk3 (or hclk) divided by 2.
    pub clock_mode: ClockMode,
    /// ADC sample time. See the `SampleTime` enum for details. Higher values
    ///  result in more accurate readings.
    pub sample_time: SampleTime,
    /// ADC clock prescaler. Defaults to no division.
    pub prescaler: Prescaler,
    /// One-shot, or continuous measurements. Defaults to one-shot.
    pub operation_mode: OperationMode,
    // Most families use u8 values for calibration, but H7 uses u16.
    /// Optional calibration data for single-ended measurements.
    pub cal_single_ended: Option<u16>,
    /// Optional calibration data for differential measurements.
    pub cal_differential: Option<u16>,

    pub ahb_freq: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            clock_mode: ClockMode::SyncDiv1,
            sample_time: Default::default(),
            prescaler: Prescaler::D1,
            operation_mode: OperationMode::OneShot,
            cal_single_ended: None,
            cal_differential: None,
            ahb_freq: 0,
        }
    }
}

pub struct Adc<const N: u8> {
    pub cfg: Config,
    pub vdda_calibrated: f32,

    periph_regs_ptr: *const pac::adc3::RegisterBlock,
    common_regs_ptr: *const pac::adc3_common::RegisterBlock,
}

impl<const N: u8> Adc<N> {
    const CHECK: () = {
        assert!(1 <= N && N <= 3, "Adc must be 1 - 3.");
    };
    pub fn init<const PORT: char, const PIN: u8>(adc_pin: gpio::GPIO<PORT, PIN>, cfg: Config) -> Self {
        let _ = Self::CHECK;

        assert!(matches!(adc_pin.mode, gpio::PinMode::Analog), "Mode is not Analog");

        assert!(
            match N {
                1 => match PORT {
                    'A' => 0 <= PIN && PIN <= 7,
                    'B' => 0 <= PIN && PIN <= 1,
                    'C' => 0 <= PIN && PIN <= 5,
                    'F' => 11 <= PIN && PIN <= 12,
                    _ => false,
                },
                2 => match PORT {
                    'A' => 2 <= PIN && PIN <= 7,
                    'B' => 0 <= PIN && PIN <= 1,
                    'C' => 0 <= PIN && PIN <= 5,
                    'F' => 13 <= PIN && PIN <= 14,
                    _ => false,
                },
                3 => match PORT {
                    'C' => 0 <= PIN && PIN <= 2,
                    'F' => 3 <= PIN && PIN <= 10,
                    'H' => 2 <= PIN && PIN <= 5,
                    _ => false,
                },
                _ => false,
            }
        );

        let periph_regs_ptr: *const pac::adc3::RegisterBlock = match N {
            1 => pac::ADC1::ptr(),
            2 => pac::ADC2::ptr(),
            3 => pac::ADC3::ptr(),
            _ => panic!("Unsupported ADC number"),
        };
        let common_regs_ptr: *const pac::adc3_common::RegisterBlock = match N {
            1 | 2 => pac::ADC12_COMMON::ptr(),
                3 => pac::ADC3_COMMON::ptr(),
            _ => panic!("Unsupported ADC number"),
        };

        let mut myself = Self {
            periph_regs_ptr,
            common_regs_ptr,
            cfg,
            vdda_calibrated: 0.
        };
        
        let rcc = unsafe { &(*pac::RCC::ptr()) };
        let periph = unsafe { &(*periph_regs_ptr)};
        let common = unsafe { &(*common_regs_ptr)};

        match N {
            1 | 2 => rcc.ahb1enr.modify(|_, w| w.adc12en().set_bit()),
                3 => rcc.ahb4enr.modify(|_, w| w.adc3en().set_bit()),
            _ => unreachable!(),
        }

        common.ccr.modify(|_, w| unsafe {
            w.presc().bits(myself.cfg.prescaler as u8);
            return w.ckmode().bits(myself.cfg.clock_mode as u8);
        });
        Self::set_align(&myself, Align::default());
        Self::advregen_enable(&mut myself);
        Self::calibrate(&mut myself, InputType::SingleEnded);
        Self::calibrate(&mut myself, InputType::Differential);
        asm::delay(myself.cfg.clock_mode as u32 * 4 * 2); // additional x2 is a pad;

        #[cfg(all(not(any(feature = "h743", feature = "h753"))))]
        periph.cr.modify(|_, w| w.boost().bits(1));
        #[cfg(any(feature = "h743", feature = "h753"))]
        periph.cr.modify(|_, w| w.boost().bit(true));

        Self::enable(&mut myself);
        Self::setup_vdda(&mut myself);

        // Don't set continuous mode until after configuring VDDA, since it needs
        // to take a oneshot reading.
        periph.cfgr.modify(|_, w| w.cont().bit(myself.cfg.operation_mode as u8 != 0));

        for ch in 1..10 {
            Self::set_sample_time(&mut myself, Channel::from(ch).unwrap());
        }
        // Note: We are getting a hardfault on G431 when setting this for channel 10.
        for ch in 11..19 {
            Self::set_sample_time(&mut myself, Channel::from(ch).unwrap());
        }

        myself
    }

    pub fn set_sequence_len(&mut self, len: SequeLen) {
        let periph = unsafe { &(*self.periph_regs_ptr)};
        periph.sqr1.modify(|_, w| unsafe { w.l().bits((len as u8) - 1) });
    }

    pub fn set_align(&self, align: Align) {
        let periph = unsafe { &(*self.periph_regs_ptr)};
        periph.cfgr2.modify(|_, w| w.lshift().bits(align as u8));
    }

    pub fn enable(&mut self) {
        let periph = unsafe { &(*self.periph_regs_ptr)};

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

    pub fn disable(&mut self) {
        let periph = unsafe { &(*self.periph_regs_ptr)};

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

    /// If any conversions are in progress, stop them. This is a step listed in the RMs
    /// for disable, and calibration procedures. See L4 RM: 16.4.17.
    /// When the ADSTP bit is set by software, any ongoing regular conversion is aborted with
    /// partial result discarded (ADC_DR register is not updated with the current conversion).
    /// When the JADSTP bit is set by software, any ongoing injected conversion is aborted with
    /// partial result discarded (ADC_JDRy register is not updated with the current conversion).
    /// The scan sequence is also aborted and reset (meaning that relaunching the ADC would
    /// restart a new sequence).
    pub fn stop_conversions(&mut self) {
        let periph = unsafe { &(*self.periph_regs_ptr)};

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

    pub fn is_enabled(&self) -> bool {
        let periph = unsafe { &(*self.periph_regs_ptr)};
        periph.cr.read().aden().bit_is_set()
    }

    pub fn is_advregen_enabled(&self) -> bool {
        let periph = unsafe { &(*self.periph_regs_ptr)};
        periph.cr.read().advregen().bit_is_set()
    }

    pub fn advregen_enable(&mut self) {
        let periph = unsafe { &(*self.periph_regs_ptr)};
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
        let periph = unsafe { &(*self.periph_regs_ptr)};
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

    fn wait_advregen_startup(&self) {
        self.delay_us(constants::MAX_ADVREGEN_STARTUP_US);
    }

    /// Calibrate. See L4 RM, 16.5.8, or F404 RM, section 15.3.8.
    /// Stores calibration values, which can be re-inserted later,
    /// eg after entering ADC deep sleep mode, or MCU STANDBY or VBAT.
    pub fn calibrate(&mut self, input_type: InputType) {
        let periph = unsafe { &(*self.periph_regs_ptr)};

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
        let periph = unsafe { &(*self.periph_regs_ptr)};

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

    pub fn set_input_type(&mut self, channel: Channel, input_type: InputType) {
        let periph = unsafe { &(*self.periph_regs_ptr)};

        // L44 RM, 16.4.7:
        // Channels can be configured to be either single-ended input or differential input by writing
        // into bits DIFSEL[15:1] in the ADC_DIFSEL register. This configuration must be written while
        // the ADC is disabled (ADEN=0). Note that DIFSEL[18:16,0] are fixed to single ended
        // channels and are always read as 0.
        let was_enabled = self.is_enabled();
        if was_enabled {
            self.disable();
        }

        // Note that we don't use the `difsel` PAC accessor here, due to its varying
        // implementations across different PACs.
        // todo: 1 offset? Experiment in firmware.
        let val = periph.difsel.read().bits();

        let val_new = match input_type {
            InputType::SingleEnded => val & !(1 << (channel as u8)),
            InputType::Differential => val | (1 << (channel as u8)),
        };
        periph.difsel.write(|w| unsafe { w.bits(val_new)});

        if was_enabled {
            self.enable();
        }
    }

    pub fn set_sequence(&mut self, channel: Channel, position: Position) {
        let periph = unsafe { &(*self.periph_regs_ptr)};
        match position {
            Position::P1 => periph.sqr1.modify(|_, w| unsafe { w.sq1().bits(channel as u8) }),
            Position::P2 => periph.sqr1.modify(|_, w| unsafe { w.sq2().bits(channel as u8) }),
            Position::P3 => periph.sqr1.modify(|_, w| unsafe { w.sq3().bits(channel as u8) }),
            Position::P4 => periph.sqr1.modify(|_, w| unsafe { w.sq4().bits(channel as u8) }),
            Position::P5 => periph.sqr2.modify(|_, w| unsafe { w.sq5().bits(channel as u8) }),
            Position::P6 => periph.sqr2.modify(|_, w| unsafe { w.sq6().bits(channel as u8) }),
            Position::P7 => periph.sqr2.modify(|_, w| unsafe { w.sq7().bits(channel as u8) }),
            Position::P8 => periph.sqr2.modify(|_, w| unsafe { w.sq8().bits(channel as u8) }),
            Position::P9 => periph.sqr2.modify(|_, w| unsafe { w.sq9().bits(channel as u8) }),
            Position::P10 => periph.sqr3.modify(|_, w| unsafe { w.sq10().bits(channel as u8) }),
            Position::P11 => periph.sqr3.modify(|_, w| unsafe { w.sq11().bits(channel as u8) }),
            Position::P12 => periph.sqr3.modify(|_, w| unsafe { w.sq12().bits(channel as u8) }),
            Position::P13 => periph.sqr3.modify(|_, w| unsafe { w.sq13().bits(channel as u8) }),
            Position::P14 => periph.sqr3.modify(|_, w| unsafe { w.sq14().bits(channel as u8) }),
            Position::P15 => periph.sqr4.modify(|_, w| unsafe { w.sq15().bits(channel as u8) }),
            Position::P16 => periph.sqr4.modify(|_, w| unsafe { w.sq16().bits(channel as u8) })
        }

        periph.pcsel.modify(|r, w| unsafe { w.pcsel().bits(r.pcsel().bits() | (1 << (channel as u8))) });
    }

    pub fn set_sample_time(&mut self, channel: Channel) {
        let periph = unsafe { &(*self.periph_regs_ptr)};

        let smp: SampleTime = self.cfg.sample_time;
        
        // RM: Note: only allowed when ADSTART = 0 and JADSTART = 0.
        self.stop_conversions();

        // self.disable();
        // while periph.cr.read().adstart().bit_is_set() || periph.cr.read().jadstart().bit_is_set() {}

        let channel_u8 = channel as u8;

        if channel_u8 < 10 {
            periph.smpr1.modify(|r, w| unsafe {
                // 現在の値を読み出して、指定されたチャンネルに対応する部分だけを変更する
                let mask = !(0b111 << (channel_u8 * 3)); // 3ビットのマスク
                let new_value = (smp as u32) << (channel_u8 * 3);
            
                // 現在のビット値を保持しつつ、新しい値を設定
                w.bits((r.bits() & mask) | new_value)
            })
        } else {
            periph.smpr2.modify(|r, w| unsafe {
                // 現在の値を読み出して、指定されたチャンネルに対応する部分だけを変更する
                let mask = !(0b111 << ((channel_u8 % 10) * 3)); // 3ビットのマスク
                let new_value = (smp as u32) << ((channel_u8 % 10) * 3);
            
                // 現在のビット値を保持しつつ、新しい値を設定
                w.bits((r.bits() & mask) | new_value)
            })
        }

        // self.enable();
    }

    /// Find and store the internal voltage reference, to improve conversion from reading
    /// to voltage accuracy. See L44 RM, section 16.4.34: "Monitoring the internal voltage reference"
    fn setup_vdda(&mut self) {
        let common = unsafe { &(*self.common_regs_ptr)};
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
        } else {
            // "Table 24. Embedded internal voltage reference" states that the sample time needs to be
            // at a minimum 4 us. With 640.5 ADC cycles we have a minimum of 8 us at 80 MHz, leaving
            // some headroom.

            common.ccr.modify(|_, w| w.vrefen().set_bit());
            // User manual table: "Embedded internal voltage reference" states that it takes a maximum of 12 us
            // to stabilize the internal voltage reference, we wait a little more.

            // todo: Not sure what to set this delay to and how to change it based on variant, so picking
            // todo something conservative.
            self.delay_us(100);

            // This sample time is overkill.
            // Note that you will need to reset the sample time if you use this channel on this
            // ADC for something other than reading vref later.
            self.set_sample_time(Channel::from(constants::VREFINT_CH).unwrap());
            let reading = self.read(Channel::from(constants::VREFINT_CH).unwrap());
            self.stop_conversions();

            common.ccr.modify(|_, w| w.vrefen().clear_bit());

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
            constants::VREFINT_VOLTAGE * vrefint_cal as f32 / reading as f32
        };
    }

    /// Convert a raw measurement into a voltage in Volts, using the calibrated VDDA.
    /// See RM0394, section 16.4.34
    pub fn reading_to_voltage(&self, reading: u16) -> f32 {
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

        self.vdda_calibrated / 4_096. * reading as f32
    }

    /// Start a conversion: Either a single measurement, or continuous conversions.
    /// Blocks until the conversion is complete.
    /// See L4 RM 16.4.15 for details.
    pub fn start_conversion(&mut self, sequence: &[SequeLen]) {
        let periph = unsafe { &(*self.periph_regs_ptr)};

        // todo: You should call this elsewhere, once, to prevent unneded reg writes.
        for (i, channel) in sequence.iter().enumerate() {
            self.set_sequence(Channel::from(*channel as u8).unwrap(), Position::from(i as u8 + 1).unwrap()); // + 1, since sequences start at 1.
        }

        // L4 RM: In Single conversion mode, the ADC performs once all the conversions of the channels.
        // This mode is started with the CONT bit at 0 by either:
        // • Setting the ADSTART bit in the ADC_CR register (for a regular channel)
        // • Setting the JADSTART bit in the ADC_CR register (for an injected channel)
        // • External hardware trigger event (for a regular or injected channel)
        // (Here, we assume a regular channel)
        periph.cr.modify(|_, w| w.adstart().set_bit());  // Start

        // After the regular sequence is complete, after each conversion is complete,
        // the EOC (end of regular conversion) flag is set.
        // After the regular sequence is complete: The EOS (end of regular sequence) flag is set.
        while periph.isr.read().eos().bit_is_clear() {}  // wait until complete.
    }

    /// Read data from a conversion. In OneShot mode, this will generally be run right
    /// after `start_conversion`.
    pub fn read_result(&mut self) -> u16 {
        let periph = unsafe { &(*self.periph_regs_ptr)};
        let ch = 18; // todo temp!!

        periph.pcsel.modify(|r, w| unsafe { w.pcsel().bits(r.pcsel().bits() & !(1 << ch)) });
        return periph.dr.read().rdata().bits() as u16;
    }

    /// Take a single reading; return a raw integer value.
    pub fn read(&mut self, channel: Channel) -> u16 {
        self.start_conversion(&[channel]);
        self.read_result()
    }

    /// Take a single reading; return a voltage.
    pub fn read_voltage(&mut self, channel: Channel) -> f32 {
        let reading = self.read(channel);
        self.reading_to_voltage(reading)
    }

    /// Select and activate a trigger. See G4 RM, section 21.4.18:
    /// Conversion on external trigger and trigger polarit
    pub fn set_trigger(&mut self, trigger: Trigger, edge: TriggerEdge) {
        let periph = unsafe { &(*self.periph_regs_ptr)};
        periph.cfgr.modify(|_, w| unsafe {
            w.exten().bits(edge as u8);
            w.extsel().bits(trigger as u8)
        });
    }

    /// Take a reading, using DMA. Sets conversion sequence; no need to set it directly.
    /// Note that the `channel` argument is unused on F3 and L4, since it is hard-coded,
    /// and can't be configured using the DMAMUX peripheral. (`dma::mux()` fn).
    pub unsafe fn read_dma(
        &mut self, buf: &mut [u16],
        adc_channels: &[u8],
        dma_channel: dma::DmaChannel,
        channel_cfg: dma::ChannelCfg,
        dma_periph: &mut dma::Dma<1>,
    ) {
        let periph = unsafe { &(*self.periph_regs_ptr)};
        let (ptr, len) = (buf.as_mut_ptr(), buf.len());
        // The software is allowed to write (dmaen and dmacfg) only when ADSTART=0 and JADSTART=0 (which
        // ensures that no conversion is ongoing)
        self.stop_conversions();

        periph.cfgr.modify(|_, w| {
            // Note: To use non-DMA after this has been set, need to configure manually.
            // ie set back to 0b00.
            w.dmngt().bits(if channel_cfg.circular == dma::Circular::Enabled { 0b11 } else { 0b01 })
        });

        let mut seq_len = 0;
        for (i, ch) in adc_channels.iter().enumerate() {
            self.set_sequence(Channel::from(*ch).unwrap(), Position::from(i as u8 + 1).unwrap());
            seq_len += 1;
        }
        self.set_sequence_len(SequeLen::from(seq_len).unwrap());

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
            channel_cfg,
        );
    }

    /// Enable a specific type of ADC interrupt.
    pub fn enable_interrupt(&mut self, interrupt: AdcInterrupt) {
        let periph = unsafe { &(*self.periph_regs_ptr)};
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
        let periph = unsafe { &(*self.periph_regs_ptr)};
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

    /// A blocking delay, for a specified time in μs.
    pub fn delay_us(&self, num_us: u32) {
        let cp = unsafe { cortex_m::Peripherals::steal() };
        let mut delay = Delay::new(cp.SYST, self.cfg.ahb_freq);
        delay.delay_us(num_us);
    }
}

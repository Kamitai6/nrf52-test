use crate::*;

use crate::pac::rcc::cfgr::SW_A as SW;
use crate::pac::rcc::cfgr::TIMPRE_A as TIMPRE;
use crate::pac::rcc::pllckselr::PLLSRC_A as PLLSRC;
use crate::pac::rcc::d1cfgr::HPRE_A as HPRE;
use crate::pac::rcc::d1ccipr::CKPERSEL_A as CKPERSEL;

use super::pwr;

mod constants {
    pub const FRACN_DIVISOR: f32 = 8192.0; // 2 ** 13
    pub const FRACN_MAX: f32 = 8192.0 - 1.0;
    pub const HSI: u32 = 64_000_000; // Hz
    pub const CSI: u32 = 4_000_000; // Hz
    pub const HSI48: u32 = 48_000_000; // Hz
    pub const LSI: u32 = 32_000; // Hz
}

/// Strategies for configuring a Phase Locked Loop (PLL)
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PllConfigStrategy {
    /// VCOL, highest PFD frequency, highest VCO frequency
    Normal,
    /// VCOH, choose PFD frequency for accuracy, highest VCO frequency
    Iterative,
    /// VCOH, choose PFD frequency for accuracy, highest VCO frequency
    /// Uses fractional mode to precisely set the P clock
    Fractional,
    /// VCOH, choose PFD frequency for accuracy, highest VCO frequency
    /// Uses fractional mode to precisely set the P clock not less than target frequency
    FractionalNotLess,
}

/// Configuration of a Phase Locked Loop (PLL)
pub struct PllConfig {
    pub strategy: PllConfigStrategy,
    pub p_ck: Option<Hertz>,
    pub q_ck: Option<Hertz>,
    pub r_ck: Option<Hertz>,
}
impl Default for PllConfig {
    fn default() -> PllConfig {
        PllConfig {
            strategy: PllConfigStrategy::Normal,
            p_ck: None,
            q_ck: None,
            r_ck: None,
        }
    }
}

/// Gives the reason why the mcu was reset
#[derive(Debug, Copy, Clone)]
pub enum ResetReason {
    /// The mcu went from not having power to having power and resetting
    PowerOnReset,
    /// The reset pin was asserted
    PinReset,
    /// The brownout detector triggered
    BrownoutReset,
    /// The software did a soft reset through the SCB peripheral
    SystemReset,
    /// The software did a soft reset through the RCC periperal
    CpuReset,
    /// The window watchdog triggered
    WindowWatchdogReset,
    /// The independent watchdog triggered
    IndependentWatchdogReset,
    /// Either of the two watchdogs triggered (but we don't know which one)
    GenericWatchdogReset,
    /// The DStandby mode was exited
    D1ExitsDStandbyMode,
    /// The DStandby mode was exited
    D2ExitsDStandbyMode,
    /// A state has been entered erroneously
    D1EntersDStandbyErroneouslyOrCpuEntersCStopErroneously,
    /// The reason could not be determined
    Unknown {
        /// The raw register value
        rcc_rsr: u32,
    },
}

/// Configuration of the core clocks
pub struct Config {
    pub hse: Option<Hertz>,
    pub bypass_hse: bool,
    pub sys_ck: Option<Hertz>,
    pub per_ck: Option<Hertz>,
    pub rcc_hclk: Option<Hertz>,
    pub rcc_pclk1: Option<Hertz>,
    pub rcc_pclk2: Option<Hertz>,
    pub rcc_pclk3: Option<Hertz>,
    pub rcc_pclk4: Option<Hertz>,
    pub pll1: PllConfig,
    pub pll2: PllConfig,
    pub pll3: PllConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hse: None,
            bypass_hse: false,
            sys_ck: Some(100.MHz()),
            per_ck: None,
            rcc_hclk: None,
            rcc_pclk1: None,
            rcc_pclk2: None,
            rcc_pclk3: None,
            rcc_pclk4: None,
            pll1: PllConfig::default(),
            pll2: PllConfig::default(),
            pll3: PllConfig::default(),
        }
    }
}

pub struct Rcc {
    pub hclk: Hertz,
    pub pclk1: Hertz,
    pub pclk2: Hertz,
    pub pclk3: Hertz,
    pub pclk4: Hertz,
    pub ppre1: u8,
    pub ppre2: u8,
    pub ppre3: u8,
    pub ppre4: u8,
    pub csi_ck: Option<Hertz>,
    pub hsi_ck: Option<Hertz>,
    pub hsi48_ck: Option<Hertz>,
    pub lsi_ck: Option<Hertz>,
    pub per_ck: Option<Hertz>,
    pub hse_ck: Option<Hertz>,
    pub pll1_p_ck: Option<Hertz>,
    pub pll1_q_ck: Option<Hertz>,
    pub pll1_r_ck: Option<Hertz>,
    pub pll2_p_ck: Option<Hertz>,
    pub pll2_q_ck: Option<Hertz>,
    pub pll2_r_ck: Option<Hertz>,
    pub pll3_p_ck: Option<Hertz>,
    pub pll3_q_ck: Option<Hertz>,
    pub pll3_r_ck: Option<Hertz>,
    pub timx_ker_ck: Hertz,
    pub timy_ker_ck: Hertz,
    pub sys_ck: Hertz,
    pub c_ck: Hertz,
}

impl Rcc {
    pub fn init(power: pwr::Power, mut config: Config) -> Self
    {
        // We do not reset RCC here. This routine must assert when
        // the previous state of the RCC peripheral is unacceptable.

        // config modifications ----------------------------------------
        // (required for self-consistency and usability)

        // sys_ck from PLL if needed, else HSE or HSI
        let (sys_ck, sys_use_pll1_p) = Self::sys_ck_setup(&mut config);

        // Configure traceclk from PLL if needed
        Self::traceclk_setup(&mut config, sys_use_pll1_p);

        // self is now immutable ----------------------------------------
        let rcc = unsafe { &(*pac::RCC::ptr()) };

        // Configure PLL1
        let (pll1_p_ck, pll1_q_ck, pll1_r_ck) =
            Self::pll_setup::<1>(&mut config);
        // Configure PLL2
        let (pll2_p_ck, pll2_q_ck, pll2_r_ck) =
            Self::pll_setup::<2>(&mut config);
        // Configure PLL3
        let (pll3_p_ck, pll3_q_ck, pll3_r_ck) =
            Self::pll_setup::<3>(&mut config);

        let sys_ck = if sys_use_pll1_p {
            pll1_p_ck.unwrap() // Must have been set by sys_ck_setup
        } else {
            sys_ck
        };

        // hsi_ck = HSI. This routine does not support HSIDIV != 1. To
        // do so it would need to ensure all PLLxON bits are clear
        // before changing the value of HSIDIV
        let hsi = constants::HSI;
        assert!(rcc.cr.read().hsion().is_on(), "HSI oscillator must be on!");
        assert!(rcc.cr.read().hsidiv().is_div1());

        let csi = constants::CSI;
        let hsi48 = constants::HSI48;

        // Enable LSI for RTC, IWDG, AWU, or MCO2
        let lsi = constants::LSI;
        rcc.csr.modify(|_, w| w.lsion().on());
        while rcc.csr.read().lsirdy().is_not_ready() {}

        // per_ck from HSI by default
        let (per_ck, ckpersel) =
            match (config.per_ck == config.hse, config.per_ck) {
                (true, Some(hse)) => (hse, CKPERSEL::Hse), // HSE
                (_, Some(csi)) => (csi, CKPERSEL::Csi),    // CSI
                _ => (hsi.Hz(), CKPERSEL::Hsi),                 // HSI
            };

        // D1 Core Prescaler
        // Set to 1
        let d1cpre_bits = 0;
        let d1cpre_div = 1;
        let sys_d1cpre_ck = sys_ck.raw() / d1cpre_div;

        // Timer prescaler selection
        let timpre = TIMPRE::DefaultX2;

        // Refer to part datasheet "General operating conditions"
        // table for (rev V). We do not assert checks for earlier
        // revisions which may have lower limits.
        let (sys_d1cpre_ck_max, rcc_hclk_max, pclk_max) = match power.vos {
            pwr::VoltageScale::Scale0 => (480_000_000, 240_000_000, 120_000_000),
            pwr::VoltageScale::Scale1 => (400_000_000, 200_000_000, 100_000_000),
            pwr::VoltageScale::Scale2 => (300_000_000, 150_000_000, 75_000_000),
            _ => (200_000_000, 100_000_000, 50_000_000),
        };

        // Check resulting sys_d1cpre_ck
        assert!(sys_d1cpre_ck <= sys_d1cpre_ck_max);

        // Get AHB clock or sensible default
        let rcc_hclk = config.rcc_hclk.map(|rate| rate.raw()).unwrap_or(sys_d1cpre_ck / 2);

        assert!(rcc_hclk <= rcc_hclk_max);

        // Estimate divisor
        let (hpre_bits, hpre_div) = match sys_d1cpre_ck.div_ceil(rcc_hclk) {
            0 => unreachable!(),
            1 => (HPRE::Div1, 1),
            2 => (HPRE::Div2, 2),
            3..=5 => (HPRE::Div4, 4),
            6..=11 => (HPRE::Div8, 8),
            12..=39 => (HPRE::Div16, 16),
            40..=95 => (HPRE::Div64, 64),
            96..=191 => (HPRE::Div128, 128),
            192..=383 => (HPRE::Div256, 256),
            _ => (HPRE::Div512, 512),
        };

        // Calculate real AXI and AHB clock
        let rcc_hclk = sys_d1cpre_ck / hpre_div;
        assert!(rcc_hclk <= rcc_hclk_max);
        let rcc_aclk = rcc_hclk; // AXI clock is always equal to AHB clock on H7

        // Calculate ppreN dividers and real rcc_pclkN frequencies
        // Get intended rcc_pclk1 frequency
            let rcc_pclk1: u32 = config
            .rcc_pclk1
            .map(|rate| rate.raw())
            .unwrap_or_else(|| core::cmp::min(pclk_max, rcc_hclk / 2));

            // Calculate suitable divider
            let (ppre1_bits, ppre1) = match rcc_hclk.div_ceil(rcc_pclk1) {
            0 => unreachable!(),
            1 => (0b000, 1 as u8),
            2 => (0b100, 2),
            3..=5 => (0b101, 4),
            6..=11 => (0b110, 8),
            _ => (0b111, 16),
            };

            // Calculate real APB1 clock
            let rcc_pclk1 = rcc_hclk / u32::from(ppre1);

            // Check in range
            assert!(rcc_pclk1 <= pclk_max);

            let rcc_timx_ker_ck = match (ppre1_bits, &timpre) {
            (0b101, TIMPRE::DefaultX2) => rcc_hclk / 2,
            (0b110, TIMPRE::DefaultX4) => rcc_hclk / 2,
            (0b110, TIMPRE::DefaultX2) => rcc_hclk / 4,
            (0b111, TIMPRE::DefaultX4) => rcc_hclk / 4,
            (0b111, TIMPRE::DefaultX2) => rcc_hclk / 8,
            _ => rcc_hclk,
            };

            // Get intended rcc_pclk2 frequency
            let rcc_pclk2: u32 = config
            .rcc_pclk2
            .map(|rate| rate.raw())
            .unwrap_or_else(|| core::cmp::min(pclk_max, rcc_hclk / 2));

            // Calculate suitable divider
            let (ppre2_bits, ppre2) = match rcc_hclk.div_ceil(rcc_pclk2) {
            0 => unreachable!(),
            1 => (0b000, 1 as u8),
            2 => (0b100, 2),
            3..=5 => (0b101, 4),
            6..=11 => (0b110, 8),
            _ => (0b111, 16),
            };

            // Calculate real APB2 clock
            let rcc_pclk2 = rcc_hclk / u32::from(ppre2);

            // Check in range
            assert!(rcc_pclk2 <= pclk_max);

            let rcc_timy_ker_ck = match (ppre2_bits, &timpre) {
            (0b101, TIMPRE::DefaultX2) => rcc_hclk / 2,
            (0b110, TIMPRE::DefaultX4) => rcc_hclk / 2,
            (0b110, TIMPRE::DefaultX2) => rcc_hclk / 4,
            (0b111, TIMPRE::DefaultX4) => rcc_hclk / 4,
            (0b111, TIMPRE::DefaultX2) => rcc_hclk / 8,
            _ => rcc_hclk,
            };

            // Get intended rcc_pclk3 frequency
            let rcc_pclk3: u32 = config
            .rcc_pclk3
            .map(|rate| rate.raw())
            .unwrap_or_else(|| core::cmp::min(pclk_max, rcc_hclk / 2));

            // Calculate suitable divider
            let (ppre3_bits, ppre3) = match rcc_hclk.div_ceil(rcc_pclk3) {
            0 => unreachable!(),
            1 => (0b000, 1 as u8),
            2 => (0b100, 2),
            3..=5 => (0b101, 4),
            6..=11 => (0b110, 8),
            _ => (0b111, 16),
            };

            // Calculate real APB3 clock
            let rcc_pclk3 = rcc_hclk / u32::from(ppre3);

            // Check in range
            assert!(rcc_pclk3 <= pclk_max);

            // Get intended rcc_pclk4 frequency
            let rcc_pclk4: u32 = config
            .rcc_pclk4
            .map(|rate| rate.raw())
            .unwrap_or_else(|| core::cmp::min(pclk_max, rcc_hclk / 2));

            // Calculate suitable divider
            let (ppre4_bits, ppre4) = match rcc_hclk.div_ceil(rcc_pclk4) {
            0 => unreachable!(),
            1 => (0b000, 1 as u8),
            2 => (0b100, 2),
            3..=5 => (0b101, 4),
            6..=11 => (0b110, 8),
            _ => (0b111, 16),
            };

            // Calculate real APB4 clock
            let rcc_pclk4 = rcc_hclk / u32::from(ppre4);

            // Check in range
            assert!(rcc_pclk4 <= pclk_max);

        // Start switching clocks here! ----------------------------------------
        
        // Flash setup
        Self::flash_setup(rcc_aclk, power.vos);
        
        // Ensure CSI is on and stable
        rcc.cr.modify(|_, w| w.csion().on());
        while rcc.cr.read().csirdy().is_not_ready() {}

        // Ensure HSI48 is on and stable
        rcc.cr.modify(|_, w| w.hsi48on().on());
        while rcc.cr.read().hsi48rdy().is_not_ready() {}

        // HSE
        let hse_ck = match config.hse {
            Some(hse) => {
                // Ensure HSE is on and stable
                rcc.cr.modify(|_, w| {
                    w.hseon().on().hsebyp().bit(config.bypass_hse)
                });
                while rcc.cr.read().hserdy().is_not_ready() {}

                Some(hse)
            }
            None => None,
        };

        // PLL
        let pllsrc = if config.hse.is_some() {
            PLLSRC::Hse
        } else {
            PLLSRC::Hsi
        };
        rcc.pllckselr.modify(|_, w| w.pllsrc().variant(pllsrc));

        // PLL1
        if pll1_p_ck.is_some() {
            // Enable PLL and wait for it to stabilise
            rcc.cr.modify(|_, w| w.pll1on().on());
            while rcc.cr.read().pll1rdy().is_not_ready() {}
        }

        // PLL2
        if pll2_p_ck.is_some() {
            // Enable PLL and wait for it to stabilise
            rcc.cr.modify(|_, w| w.pll2on().on());
            while rcc.cr.read().pll2rdy().is_not_ready() {}
        }

        // PLL3
        if pll3_p_ck.is_some() {
            // Enable PLL and wait for it to stabilise
            rcc.cr.modify(|_, w| w.pll3on().on());
            while rcc.cr.read().pll3rdy().is_not_ready() {}
        }

        // Core Prescaler / AHB Prescaler / APB3 Prescaler
        rcc.d1cfgr.modify(|_, w| unsafe {
            w.d1cpre()
                .bits(d1cpre_bits)
                .d1ppre() // D1 contains APB3
                .bits(ppre3_bits)
                .hpre()
                .variant(hpre_bits)
        });
        // Ensure core prescaler value is valid before future lower
        // core voltage
        while rcc.d1cfgr.read().d1cpre().bits() != d1cpre_bits {}

        // APB1 / APB2 Prescaler
        rcc.d2cfgr.modify(|_, w| unsafe {
            w.d2ppre1() // D2 contains APB1
                .bits(ppre1_bits)
                .d2ppre2() // D2 also contains APB2
                .bits(ppre2_bits)
        });

        // APB4 Prescaler
        rcc.d3cfgr.modify(|_, w| unsafe {
            w.d3ppre() // D3 contains APB4
                .bits(ppre4_bits)
        });

        // Peripheral Clock (per_ck)
        rcc.d1ccipr.modify(|_, w| w.ckpersel().variant(ckpersel));

        // Set timer clocks prescaler setting
        rcc.cfgr.modify(|_, w| w.timpre().variant(timpre));

        // Select system clock source
        let swbits = match (sys_use_pll1_p, config.hse.is_some()) {
            (true, _) => SW::Pll1 as u8,
            (false, true) => SW::Hse as u8,
            _ => SW::Hsi as u8,
        };
        rcc.cfgr.modify(|_, w| unsafe { w.sw().bits(swbits) });
        while rcc.cfgr.read().sws().bits() != swbits {}

        // IO compensation cell - Requires CSI clock and SYSCFG
        assert!(rcc.cr.read().csirdy().is_ready());
        rcc.apb4enr.modify(|_, w| w.syscfgen().enabled());

        // Enable the compensation cell, using back-bias voltage code
        // provide by the cell.
        let syscfg = unsafe {&(*pac::SYSCFG::ptr())};
        syscfg.cccsr.modify(|_, w| {
            w.en().set_bit().cs().clear_bit().hslv().clear_bit()
        });
        while syscfg.cccsr.read().ready().bit_is_clear() {}

        // Return frozen clock configuration
        Self {
            hclk: rcc_hclk.Hz(),
            pclk1: rcc_pclk1.Hz(),
            pclk2: rcc_pclk2.Hz(),
            pclk3: rcc_pclk3.Hz(),
            pclk4: rcc_pclk4.Hz(),
            ppre1,
            ppre2,
            ppre3,
            ppre4,
            csi_ck: Some(csi.Hz()),
            hsi_ck: Some(hsi.Hz()),
            hsi48_ck: Some(hsi48.Hz()),
            lsi_ck: Some(lsi.Hz()),
            per_ck: Some(per_ck),
            hse_ck,
            pll1_p_ck,
            pll1_q_ck,
            pll1_r_ck,
            pll2_p_ck,
            pll2_q_ck,
            pll2_r_ck,
            pll3_p_ck,
            pll3_q_ck,
            pll3_r_ck,
            timx_ker_ck: rcc_timx_ker_ck.Hz(),
            timy_ker_ck: rcc_timy_ker_ck.Hz(),
            sys_ck,
            c_ck: sys_d1cpre_ck.Hz(),
        }
    }

    fn vco_setup<const PLL: u8>(
        strategy: PllConfigStrategy,
        pllsrc: u32,
        output: u32,
    ) -> (u32, u32, u32, u32)
    {
        assert!(1 <= PLL && PLL <= 3, "PLL must be 1 - 3.");

        let rcc = unsafe { &(*pac::RCC::ptr()) };
        let (vco_min, vco_max) = match strategy {
            PllConfigStrategy::Normal => (150_000_000, 420_000_000),
            _ => {
                #[cfg(not(feature = "revision_v"))]
                let vco_limits = (192_000_000, 836_000_000);
                #[cfg(feature = "revision_v")]
                let vco_limits = (192_000_000, 960_000_000);
                vco_limits
            }
        };
    
        let (vco_ck_target, pll_x_p) = {
            let pll_x_p = match PLL {
                1 => {
                    if output > vco_max / 2 {
                        1
                    } else {
                        ((vco_max / output) | 1) - 1 // 偶数または1でなければならない
                    }
                }
                _ => {
                    if output > vco_max / 2 {
                        1
                    } else {
                        vco_max / output
                    }
                }
            };
        
            let vco_ck = output * pll_x_p;
        
            assert!(pll_x_p <= 128, "Cannot achieve output frequency this low: Maximum PLL divider is 128");
            assert!(vco_ck >= vco_min);
            assert!(vco_ck <= vco_max);
        
            (vco_ck, pll_x_p)
        };
    
        let pll_x_m = match strategy {
            PllConfigStrategy::Normal => (pllsrc + 1_999_999) / 2_000_000,
            _ => {
                let pll_x_m_min = (pllsrc + 15_999_999) / 16_000_000;
                let pll_x_m_max = if pllsrc <= 127_999_999 { pllsrc / 2_000_000 } else { 63 };
                (pll_x_m_min..=pll_x_m_max).min_by_key(|&pll_x_m| {
                    let ref_x_ck = pllsrc / pll_x_m;
                    let pll_x_n = vco_ck_target / ref_x_ck;
                    (vco_ck_target as i32 - (ref_x_ck * pll_x_n) as i32).abs()
                }).unwrap()
            }
        };
    
        assert!(pll_x_m < 64);
        let ref_x_ck = pllsrc / pll_x_m;
    
        match strategy {
            PllConfigStrategy::Normal => {
                assert!((1_000_000..=2_000_000).contains(&ref_x_ck));
            },
            _ => {
                assert!((2_000_000..=16_000_000).contains(&ref_x_ck));
            }
        };
    
        match PLL {
            1 => {
                rcc.pllcfgr.modify(|_, w| {
                    w.pll1vcosel().medium_vco();
                    w.pll1rge().range1();
                    w
                });
            }
            2 => {
                rcc.pllcfgr.modify(|_, w| {
                    w.pll2vcosel().medium_vco();
                    w.pll2rge().range1();
                    w
                });
            }
            3 => {
                rcc.pllcfgr.modify(|_, w| {
                    w.pll3vcosel().medium_vco();
                    w.pll3rge().range1();
                    w
                });
            }
            _ => unreachable!(),
        }
    
        if strategy == PllConfigStrategy::Iterative {
            rcc.pllcfgr.modify(|_, w| {
                match ref_x_ck {
                    2_000_000..=3_999_999 => {
                        match PLL {
                            1 => w.pll1rge().range2(),
                            2 => w.pll2rge().range2(),
                            3 => w.pll3rge().range2(),
                            _ => unreachable!(),
                        }
                    }
                    4_000_000..=7_999_999 => {
                        match PLL {
                            1 => w.pll1rge().range4(),
                            2 => w.pll2rge().range4(),
                            3 => w.pll3rge().range4(),
                            _ => unreachable!(),
                        }
                    }
                    _ => {
                        match PLL {
                            1 => w.pll1rge().range8(),
                            2 => w.pll2rge().range8(),
                            3 => w.pll3rge().range8(),
                            _ => unreachable!(),
                        }
                    }
                }
            });
        }
    
        (ref_x_ck, pll_x_m, pll_x_p, vco_ck_target)
    }
    
    /// PLL 設定のジェネリック関数
    fn pll_setup<const N: u8>(config: &mut Config) -> (Option<Hertz>, Option<Hertz>, Option<Hertz>)
    {
        assert!(1 <= N && N <= 3, "PLL must be 1 - 3.");

        let pll = match N {
            1 => &config.pll1,
            2 => &config.pll2,
            3 => &config.pll3,
            _ => unreachable!(),
        };

        let rcc = unsafe { &(*pac::RCC::ptr()) };
        // PLLの入力クロック（HSI または HSE）
        let pllsrc = config.hse.map(|rate| rate.raw()).unwrap_or(constants::HSI);
        assert!(pllsrc > 0);
    
        if let Some(output) = pll.p_ck.or(pll.q_ck.or(pll.r_ck)) {
            let (ref_x_ck, pll_x_m, pll_x, vco_ck_target) = if N == 1 {
                Self::vco_setup::<N>(pll.strategy, pllsrc, output.raw())
            } else {
                Self::vco_setup::<N>(pll.strategy, pllsrc, output.raw())
            };
    
            let pll_x_n = vco_ck_target / ref_x_ck;
            assert!((4..=512).contains(&pll_x_n));
    
            match N {
                1 => {
                    rcc.pllckselr.modify(|_, w| {w.divm1().bits(pll_x_m as u8)});
                    rcc.pll1divr.modify(|_, w| unsafe { w.divn1().bits((pll_x_n - 1) as u16) });
                }
                2 => {
                    rcc.pllckselr.modify(|_, w| {w.divm2().bits(pll_x_m as u8)});
                    rcc.pll2divr.modify(|_, w| unsafe { w.divn2().bits((pll_x_n - 1) as u16) });
                }
                3 => {
                    rcc.pllckselr.modify(|_, w| {w.divm3().bits(pll_x_m as u8)});
                    rcc.pll3divr.modify(|_, w| unsafe { w.divn3().bits((pll_x_n - 1) as u16) });
                }
                _ => unreachable!(),
            }
            
            let vco_ck = match pll.strategy {
                PllConfigStrategy::Fractional => {
                    let pll_x_fracn = Self::calc_fracn(ref_x_ck as f32, pll_x_n as f32, pll_x as f32, output.raw() as f32);
                    match N {
                        1 => {
                            rcc.pll1fracr.modify(|_, w| {w.fracn1().bits(pll_x_fracn)});
                            rcc.pllcfgr.modify(|_, w| {w.pll1fracen().set()});
                        }
                        2 => {
                            rcc.pll2fracr.modify(|_, w| {w.fracn2().bits(pll_x_fracn)});
                            rcc.pllcfgr.modify(|_, w| {w.pll2fracen().set()});
                        }
                        3 => {
                            rcc.pll3fracr.modify(|_, w| {w.fracn3().bits(pll_x_fracn)});
                            rcc.pllcfgr.modify(|_, w| {w.pll3fracen().set()});
                        }
                        _ => unreachable!(),
                    }
                    (ref_x_ck as f32 * (pll_x_n as f32 + (pll_x_fracn as f32 / constants::FRACN_DIVISOR))) as u32
                },
                PllConfigStrategy::FractionalNotLess => {
                    let mut pll_x_fracn = Self::calc_fracn(ref_x_ck as f32, pll_x_n as f32, pll_x as f32, output.raw() as f32);
                    pll_x_fracn += 1;
                    match N {
                        1 => {
                            rcc.pll1fracr.modify(|_, w| {w.fracn1().bits(pll_x_fracn)});
                            rcc.pllcfgr.modify(|_, w| {w.pll1fracen().set()});
                        }
                        2 => {
                            rcc.pll2fracr.modify(|_, w| {w.fracn2().bits(pll_x_fracn)});
                            rcc.pllcfgr.modify(|_, w| {w.pll2fracen().set()});
                        }
                        3 => {
                            rcc.pll3fracr.modify(|_, w| {w.fracn3().bits(pll_x_fracn)});
                            rcc.pllcfgr.modify(|_, w| {w.pll3fracen().set()});
                        }
                        _ => unreachable!(),
                    }
                    (ref_x_ck as f32 * (pll_x_n as f32 + (pll_x_fracn as f32 / constants::FRACN_DIVISOR))) as u32
                },
                _ => {
                    match N {
                        1 => rcc.pllcfgr.modify(|_, w| {w.pll1fracen().reset()}),
                        2 => rcc.pllcfgr.modify(|_, w| {w.pll2fracen().reset()}),
                        3 => rcc.pllcfgr.modify(|_, w| {w.pll3fracen().reset()}),
                        _ => unreachable!(),
                    }
                    ref_x_ck * pll_x_n
                },
            };
    
            let pll_x_q = pll.q_ck.map(|ck| Self::calc_ck_div(pll.strategy, vco_ck, ck.raw())).unwrap_or(0);
            let pll_x_r = pll.r_ck.map(|ck| Self::calc_ck_div(pll.strategy, vco_ck, ck.raw())).unwrap_or(0);
    
            let dividers = (pll_x, pll_x_q, pll_x_r);
    
            let p_ck = pll.p_ck.map(|_| {
                assert!(dividers.0 <= 128);
                match N {
                    1 => {
                        rcc.pll1divr.modify(|_, w| unsafe {w.divp1().bits((dividers.0 - 1) as u8)});
                        rcc.pllcfgr.modify(|_, w| w.divp1en().enabled());
                    }
                    2 => {
                        rcc.pll2divr.modify(|_, w| unsafe {w.divp2().bits((dividers.0 - 1) as u8)});
                        rcc.pllcfgr.modify(|_, w| w.divp2en().enabled());
                    }
                    3 => {
                        rcc.pll3divr.modify(|_, w| unsafe {w.divp3().bits((dividers.0 - 1) as u8)});
                        rcc.pllcfgr.modify(|_, w| w.divp3en().enabled());
                    }
                    _ => unreachable!(),
                }
                Some(Hertz::from_raw(vco_ck / dividers.0))
            }).flatten();
    
            let q_ck = pll.q_ck.map(|_| {
                assert!(dividers.1 <= 128);
                match N {
                    1 => {
                        rcc.pll1divr.modify(|_, w| unsafe {w.divq1().bits((dividers.1 - 1) as u8)});
                        rcc.pllcfgr.modify(|_, w| w.divq1en().enabled());
                    }
                    2 => {
                        rcc.pll2divr.modify(|_, w| unsafe {w.divq2().bits((dividers.1 - 1) as u8)});
                        rcc.pllcfgr.modify(|_, w| w.divq2en().enabled());
                    }
                    3 => {
                        rcc.pll3divr.modify(|_, w| unsafe {w.divq3().bits((dividers.1 - 1) as u8)});
                        rcc.pllcfgr.modify(|_, w| w.divq3en().enabled());
                    }
                    _ => unreachable!(),
                }
                Some(Hertz::from_raw(vco_ck / dividers.1))
            }).flatten();
    
            let r_ck = pll.r_ck.map(|_| {
                assert!(dividers.2 <= 128);
                match N {
                    1 => {
                        rcc.pll1divr.modify(|_, w| unsafe {w.divr1().bits((dividers.2 - 1) as u8)});
                        rcc.pllcfgr.modify(|_, w| w.divr1en().enabled());
                    }
                    2 => {
                        rcc.pll2divr.modify(|_, w| unsafe {w.divr2().bits((dividers.2 - 1) as u8)});
                        rcc.pllcfgr.modify(|_, w| w.divr2en().enabled());
                    }
                    3 => {
                        rcc.pll3divr.modify(|_, w| unsafe {w.divr3().bits((dividers.2 - 1) as u8)});
                        rcc.pllcfgr.modify(|_, w| w.divr3en().enabled());
                    }
                    _ => unreachable!(),
                }
                Some(Hertz::from_raw(vco_ck / dividers.2))
            }).flatten();
    
            (p_ck, q_ck, r_ck)
        } else {
            (None, None, None)
        }
    }
    
    /// Calcuate the Fractional-N part of the divider
    ///
    /// ref_clk - Frequency at the PFD input
    /// pll_n - Integer-N part of the divider
    /// pll_p - P-divider
    /// output - Wanted output frequency
    fn calc_fracn(ref_clk: f32, pll_n: f32, pll_p: f32, output: f32) -> u16 {
        // VCO output frequency = Fref1_ck x (DIVN1 + (FRACN1 / 2^13)),
        let pll_fracn = constants::FRACN_DIVISOR * (((output * pll_p) / ref_clk) - pll_n);
        assert!(pll_fracn >= 0.0);
        assert!(pll_fracn <= constants::FRACN_MAX);
        // Rounding down by casting gives up the lowest without going over
        pll_fracn as u16
    }
    
    /// Calculates the {Q,R}-divider. Must NOT be used for the P-divider, as this
    /// has additional restrictions on PLL1.
    ///
    /// vco_ck - VCO output frequency
    /// target_ck - Target {Q,R} output frequency
    fn calc_ck_div(
        strategy: PllConfigStrategy,
        vco_ck: u32,
        target_ck: u32,
    ) -> u32 {
        let mut div = vco_ck.div_ceil(target_ck);
        // If the divider takes us under the target clock, then increase it
        if strategy == PllConfigStrategy::FractionalNotLess
            && target_ck * div > vco_ck
        {
            div -= 1;
        }
        div
    }
    /// Gets and clears the reason of why the mcu was reset
    pub fn get_reset_reason() -> ResetReason {
        let rcc = unsafe {&(*pac::RCC::ptr())};
        let reset_reason = rcc.rsr.read();
    
        // Clear the register
        rcc.rsr.modify(|_, w| w.rmvf().clear());
    
        // See R0433 Rev 7 Section 8.4.4 Reset source identification
        match (
            reset_reason.lpwrrstf().is_reset_occourred(),
            reset_reason.wwdg1rstf().is_reset_occourred(),
            reset_reason.iwdg1rstf().is_reset_occourred(),
            reset_reason.sftrstf().is_reset_occourred(),
            reset_reason.porrstf().is_reset_occourred(),
            reset_reason.pinrstf().is_reset_occourred(),
            reset_reason.borrstf().is_reset_occourred(),
            reset_reason.d2rstf().is_reset_occourred(),
            reset_reason.d1rstf().is_reset_occourred(),
            reset_reason.cpurstf().is_reset_occourred(),
        ) {
            (false, false, false, false, true, true, true, true, true, true) => {
                ResetReason::PowerOnReset
            }
            (false, false, false, false, false, true, false, false, false, true) => {
                ResetReason::PinReset
            }
            (false, false, false, false, false, true, true, false, false, true) => {
                ResetReason::BrownoutReset
            }
            (false, false, false, true, false, true, false, false, false, true) => {
                ResetReason::SystemReset
            }
            (false, false, false, false, false, false, false, false, false, true) => {
                ResetReason::CpuReset
            }
            (false, true, false, false, false, false, false, false, false, false) | (false, true, false, false, false, true, false, false, false, true) => {
                ResetReason::WindowWatchdogReset
            }
            (false, false, true, false, false, true, false, false, false, true) => {
                ResetReason::IndependentWatchdogReset
            }
            (false, true, true, false, false, true, false, false, false, true) => {
                ResetReason::GenericWatchdogReset
            }
            (false, false, false, false, false, false, false, false, true, false) => {
                ResetReason::D1ExitsDStandbyMode
            }
            (false, false, false, false, false, false, false, true, false, false) => {
                ResetReason::D2ExitsDStandbyMode
            }
            (true, false, false, false, false, true, false, false, false, true) => {
                ResetReason::D1EntersDStandbyErroneouslyOrCpuEntersCStopErroneously
            }
            _ => ResetReason::Unknown {
                rcc_rsr: reset_reason.bits(),
            },
        }
    }

    fn flash_setup(rcc_aclk: u32, vos: pwr::VoltageScale) {
        use crate::pac::FLASH;
        // ACLK in MHz, round down and subtract 1 from integers. eg.
        // 61_999_999 -> 61MHz
        // 62_000_000 -> 61MHz
        // 62_000_001 -> 62MHz
        let rcc_aclk_mhz = (rcc_aclk - 1) / 1_000_000;

        // See RM0433 Rev 7 Table 17. FLASH recommended number of wait
        // states and programming delay
        let (wait_states, progr_delay) = match vos {
            // VOS 0 range VCORE 1.26V - 1.40V
            pwr::VoltageScale::Scale0 => match rcc_aclk_mhz {
                0..=69 => (0, 0),
                70..=139 => (1, 1),
                140..=184 => (2, 1),
                185..=209 => (2, 2),
                210..=224 => (3, 2),
                225..=239 => (4, 2),
                _ => (7, 3),
            },
            // VOS 1 range VCORE 1.15V - 1.26V
            pwr::VoltageScale::Scale1 => match rcc_aclk_mhz {
                0..=69 => (0, 0),
                70..=139 => (1, 1),
                140..=184 => (2, 1),
                185..=209 => (2, 2),
                210..=224 => (3, 2),
                _ => (7, 3),
            },
            // VOS 2 range VCORE 1.05V - 1.15V
            pwr::VoltageScale::Scale2 => match rcc_aclk_mhz {
                0..=54 => (0, 0),
                55..=109 => (1, 1),
                110..=164 => (2, 1),
                165..=224 => (3, 2),
                _ => (7, 3),
            },
            // VOS 3 range VCORE 0.95V - 1.05V
            pwr::VoltageScale::Scale3 => match rcc_aclk_mhz {
                0..=44 => (0, 0),
                45..=89 => (1, 1),
                90..=134 => (2, 1),
                135..=179 => (3, 2),
                180..=224 => (4, 2),
                _ => (7, 3),
            },
        };

        let flash = unsafe { &(*FLASH::ptr()) };
        // Adjust flash wait states
        flash.acr.write(|w| unsafe {
            w.wrhighfreq().bits(progr_delay).latency().bits(wait_states)
        });
        while flash.acr.read().latency().bits() != wait_states {}
    }

    /// Setup sys_ck
    /// Returns sys_ck frequency, and a pll1_p_ck
    fn sys_ck_setup(config: &mut Config) -> (Hertz, bool) {
        // Compare available with wanted clocks
        let srcclk = config.hse.map(|rate| rate.raw()).unwrap_or(constants::HSI); // Available clocks
        let sys_ck = config.sys_ck.map(|rate| rate.raw()).unwrap_or(srcclk);

        if sys_ck != srcclk {
            // The requested system clock is not the immediately available
            // HSE/HSI clock. Perhaps there are other ways of obtaining
            // the requested system clock (such as `HSIDIV`) but we will
            // ignore those for now.
            //
            // Therefore we must use pll1_p_ck
            let pll1_p_ck = match config.pll1.p_ck {
                Some(p_ck) => {
                    assert!(p_ck.raw() == sys_ck,
                            "Error: Cannot set pll1_p_ck independently as it must be used to generate sys_ck");
                    Some(p_ck)
                }
                None => Some(sys_ck.Hz()),
            };
            config.pll1.p_ck = pll1_p_ck;

            (sys_ck.Hz(), true)
        } else {
            // sys_ck is derived directly from a source clock
            // (HSE/HSI). pll1_p_ck can be as requested
            (sys_ck.Hz(), false)
        }
    }

    /// Setup traceclk
    /// Returns a pll1_r_ck
    fn traceclk_setup(config: &mut Config, sys_use_pll1_p: bool) {
        let pll1_r_ck = match (sys_use_pll1_p, config.pll1.r_ck) {
            // pll1_p_ck selected as system clock but pll1_r_ck not
            // set. The traceclk mux is synchronous with the system
            // clock mux, but has pll1_r_ck as an input. In order to
            // keep traceclk running, we force a pll1_r_ck.
            (true, None) => Some(config.pll1.p_ck.unwrap() / 2),
            // Either pll1 not selected as system clock, free choice
            // of pll1_r_ck. Or pll1 is selected, assume user has set
            // a suitable pll1_r_ck frequency.
            _ => config.pll1.r_ck,
        };
        config.pll1.r_ck = pll1_r_ck;
    }
}

/// Enables and resets peripheral clocks on various RCC registesr.
/// The first argument is a `apb1`, `ahb2` etc to specify the reg block. The second is something like
/// `tim1`, and the third is a `pac::RCC`.
#[macro_export]
macro_rules! rcc_en_reset {
    (apb1, $periph:expr, $rcc:expr) => {
        paste::paste! {
            if $rcc.apb1lenr.read().[<$periph en>]().bit_is_clear() {
                $rcc.apb1lenr.modify(|_, w| w.[<$periph en>]().set_bit());
                $rcc.apb1lrstr.modify(|_, w| w.[<$periph rst>]().set_bit());
                $rcc.apb1lrstr.modify(|_, w| w.[<$periph rst>]().clear_bit());
            }
        }
    };
    (apb2, $periph:expr, $rcc:expr) => {
        paste::paste! {
            if $rcc.apb2enr.read().[<$periph en>]().bit_is_clear() {
                $rcc.apb2enr.modify(|_, w| w.[<$periph en>]().set_bit());
                $rcc.apb2rstr.modify(|_, w| w.[<$periph rst>]().set_bit());
                $rcc.apb2rstr.modify(|_, w| w.[<$periph rst>]().clear_bit());
            }
        }
    };
    (apb4, $periph:expr, $rcc:expr) => {
        paste::paste! {
            if $rcc.apb4enr.read().[<$periph en>]().bit_is_clear() {
                $rcc.apb4enr.modify(|_, w| w.[<$periph en>]().set_bit());
                $rcc.apb4rstr.modify(|_, w| w.[<$periph rst>]().set_bit());
                $rcc.apb4rstr.modify(|_, w| w.[<$periph rst>]().clear_bit());
            }
        }
    };
    (ahb1, $periph:expr, $rcc:expr) => {
        paste::paste! {
            if $rcc.ahb1enr.read().[<$periph en>]().bit_is_clear() {
                $rcc.ahb1enr.modify(|_, w| w.[<$periph en>]().set_bit());
                $rcc.ahb1rstr.modify(|_, w| w.[<$periph rst>]().set_bit());
                $rcc.ahb1rstr.modify(|_, w| w.[<$periph rst>]().clear_bit());
            }
        }
    };
    (ahb2, $periph:expr, $rcc:expr) => {
        paste::paste! {
            if $rcc.ahb2enr.read().[<$periph en>]().bit_is_clear() {
                $rcc.ahb2enr.modify(|_, w| w.[<$periph en>]().set_bit());
                $rcc.ahb2rstr.modify(|_, w| w.[<$periph rst>]().set_bit());
                $rcc.ahb2rstr.modify(|_, w| w.[<$periph rst>]().clear_bit());
            }
        }
    };
    (ahb3, $periph:expr, $rcc:expr) => {
        paste::paste! {
            if $rcc.ahb3enr.read().[<$periph en>]().bit_is_clear() {
                $rcc.ahb3enr.modify(|_, w| w.[<$periph en>]().set_bit());
                $rcc.ahb3rstr.modify(|_, w| w.[<$periph rst>]().set_bit());
                $rcc.ahb3rstr.modify(|_, w| w.[<$periph rst>]().clear_bit());
            }
        }
    };
    (ahb4, $periph:expr, $rcc:expr) => {
        paste::paste! {
            if $rcc.ahb4enr.read().[<$periph en>]().bit_is_clear() {
                $rcc.ahb4enr.modify(|_, w| w.[<$periph en>]().set_bit());
                $rcc.ahb4rstr.modify(|_, w| w.[<$periph rst>]().set_bit());
                $rcc.ahb4rstr.modify(|_, w| w.[<$periph rst>]().clear_bit());
            }
        }
    };
}

//! Manage STM32H7 supply configuration. This is required on some H7 variants, to specify
//! which regulator to use. This must match the way the MCU power pins are wired on the hardware design.

use crate::pac;

/// Voltage Scale
///
/// Represents the voltage range feeding the CPU core. The maximum core
/// clock frequency depends on this value.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum VoltageScale {
    /// VOS 0 range VCORE 1.26V - 1.40V
    Scale0 = 0,
    /// VOS 1 range VCORE 1.15V - 1.26V
    Scale1,
    /// VOS 2 range VCORE 1.05V - 1.15V
    Scale2,
    /// VOS 3 range VCORE 0.95V - 1.05V
    Scale3,
}

/// SMPS Supply Configuration - Dual Core parts
///
/// Refer to RM0399 Rev 3 Table 32.
#[cfg(feature = "smps")]
pub enum SupplyConfiguration {
    /// Default configuration
    Default = 0,
    /// LDO supply
    Ldo,
    /// Direct SMPS supply
    DirectSmps,
    /// SMPS supplies LDO
    SmpsLdo1V8,
    /// SMPS supplies LDO
    SmpsLdo2V5,
    /// supplies external and LDO Bypass
    Bypass,
}

/// See RM0399, Table 32. Supply configuration control, for available configurations.
/// Sets the PWR_CR3 register, LDOEN, SDEN, SDEXTHP, SDLEVEL, and BYPASS fields.
pub struct PwrConfig {
    pub target_vos: VoltageScale,
    #[cfg(feature = "smps")]
    supply_configuration: SupplyConfiguration,
}

impl Default for PwrConfig {
    fn default() -> Self {
        Self {
            target_vos: VoltageScale::Scale1,
            #[cfg(feature = "smps")]
            supply_configuration: SupplyConfiguration::Default,
        }
    }
}

pub struct Power {
    pub vos: VoltageScale
}

impl Power {
    /// Apply a given supply config. `voltage_level` only affects certain variants.
    pub fn init(cfg: PwrConfig) -> Self {
        let pwr = unsafe {&(*pac::PWR::ptr())};
        let rcc = unsafe {&(*pac::RCC::ptr())};
        let syscfg = unsafe {&(*pac::SYSCFG::ptr())};
        
        // SMPSを使わないときはロックする
        #[cfg(not(feature = "smps"))]
        pwr.cr3.modify(|_, w| {
            w.scuen().set_bit().ldoen().set_bit().bypass().clear_bit()
        });

        #[cfg(feature = "smps")]
        match cfg.supply_configuration {
            PwrConfig::Default => pwr.cr3.modify(|_, w| unsafe {
                w
            }),
            PwrConfig::Ldo => pwr.cr3.modify(|_, w| unsafe {
                w.sden().clear_bit();
                w.ldoen().set_bit()
            }),
            PwrConfig::DirectSmps => pwr.cr3.modify(|_, w| unsafe {
                w.sden().set_bit();
                w.ldoen().clear_bit()
            }),
            PwrConfig::SmpsLdo1V8 => pwr.cr3.modify(|_, w| unsafe {
                w.sdlevel().bits(1);
                w.sden().set_bit();
                w.ldoen().set_bit()
            }),
            PwrConfig::SmpsLdo2V5 => pwr.cr3.modify(|_, w| unsafe {
                w.sdlevel().bits(2);
                w.sden().set_bit();
                w.ldoen().set_bit()
            }),
            PwrConfig::Bypass => pwr.cr3.modify(|_, w| unsafe {
                w.sden().clear_bit();
                w.ldoen().clear_bit();
                w.bypass().set_bit()
            }),
        }

        // Validate the supply configuration. If you are stuck here, it is
        // because the voltages on your board do not match those specified
        // in the D3CR.VOS and CR3.SDLEVEL fields.  By default after reset
        // VOS = Scale 3, so check that the voltage on the VCAP pins =
        // 1.0V.
        while pwr.csr1.read().actvosrdy().bit_is_clear() {}

        // Transition to configured voltage scale. VOS0 cannot be entered
        // directly, instead transition to VOS1 initially and then VOS0 later
        let mut vos = match cfg.target_vos {
            VoltageScale::Scale0 => VoltageScale::Scale1,
            x => x,
        };
        pwr.d3cr.write(|w| unsafe {
            // Manually set field values for each family
            w.vos().bits(
                match vos {
                    // RM0433 Rev 7 6.8.6
                    VoltageScale::Scale3 => 0b01,
                    VoltageScale::Scale2 => 0b10,
                    VoltageScale::Scale1 => 0b11,
                    _ => unimplemented!(),
                },
            )
        });
        while pwr.d3cr.read().vosrdy().bit_is_clear() {}

        // Enable overdrive for maximum clock
        // Syscfgen required to set enable overdrive
        #[cfg(feature = "revision_v")]
        if matches!(vos, VoltageScale::Scale0) {
            rcc.apb4enr.modify(|_, w| w.syscfgen().enabled());
            syscfg.pwrcr.modify(|_, w| w.oden().set_bit());
            while pwr.d3cr.read().vosrdy().bit_is_clear() {}
            vos = VoltageScale::Scale0;
        }

        Self {
            vos,
        }
    }
}

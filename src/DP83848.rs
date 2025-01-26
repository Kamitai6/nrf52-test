//! SMSC DP83848 Ethernet PHY

use crate::ethernet::{StationManagement, PHY};

mod phy_consts {
    pub const REG_BMCR: u8 = 0x00;
    pub const REG_BMSR: u8 = 0x01;
    pub const REG_PHYIDR1: u8 = 0x02; //not use
    pub const REG_PHYIDR2: u8 = 0x03; //not use
    pub const REG_ANAR: u8 = 0x04; //not use
    pub const REG_ANLPAR: u8 = 0x05; //not use
    pub const REG_ANER: u8 = 0x06; //not use
    pub const REG_ANNPTR: u8 = 0x07; //not use
    pub const REG_PHYCR: u8 = 0x19;
    pub const REG_MISR: u8 = 0x12;
    pub const REG_MICR: u8 = 0x11;
    pub const REG_PHYSTS: u8 = 0x10;

    pub const BMCR_SOFT_RESET: u16 = 0x8000;
    pub const BMCR_LOOPBACK: u16 = 0x4000;
    pub const BMCR_SPEED_SELECT: u16 = 0x2000;
    pub const BMCR_AUTONEGO_EN: u16 = 0x1000;
    pub const BMCR_POWER_DOWN: u16 = 0x0800;
    pub const BMCR_ISOLATE: u16 = 0x0400;
    pub const BMCR_RESTART_AUTONEGO: u16 = 0x0200;
    pub const BMCR_DUPLEX_MODE: u16 = 0x0100;

    pub const BMSR_100BASE_T4: u16 = 0x8000;
    pub const BMSR_100BASE_TX_FD: u16 = 0x4000;
    pub const BMSR_100BASE_TX_HD: u16 = 0x2000;
    pub const BMSR_10BASE_T_FD: u16 = 0x1000;
    pub const BMSR_10BASE_T_HD: u16 = 0x0800;
    pub const BMSR_MF_PREAMBLE: u16 = 0x0040;
    pub const BMSR_AUTONEGO_CPLT: u16 = 0x0020;
    pub const BMSR_REMOTE_FAULT: u16 = 0x0010;
    pub const BMSR_AUTONEGO_ABILITY: u16 = 0x0008;
    pub const BMSR_LINK_STATUS: u16 = 0x0004;
    pub const BMSR_JABBER_DETECT: u16 = 0x0002;
    pub const BMSR_EXTENDED_CAP: u16 = 0x0001;

    pub const PHYIDR1_OUI_3_18: u16 = 0xFFFF;

    pub const PHYIDR2_OUI_19_24: u16 = 0xFC00;
    pub const PHYIDR2_MODEL_NBR: u16 = 0x03F0;
    pub const PHYIDR2_REVISION_NBR: u16 = 0x000F;

    // TODO: Auto-Negotiation xxx Register

    pub const PHYCR_MODE: u16 = 0x00E0;
    pub const PHYCR_PHY_ADDR: u16 = 0x001F;

    pub const MIXR_WOL_IT: u16 = 0x0100;
    pub const MIXR_ENERGYON_IT: u16 = 0x0080;
    pub const MIXR_AUTONEGO_COMPLETE_IT: u16 = 0x0040;
    pub const MIXR_REMOTE_FAULT_IT: u16 = 0x0020;
    pub const MIXR_LINK_DOWN_IT: u16 = 0x0010;
    pub const MIXR_AUTONEGO_LP_ACK_IT: u16 = 0x0008;
    pub const MIXR_PARALLEL_DETECTION_FAULT_IT: u16 = 0x0004;
    pub const MIXR_AUTONEGO_PAGE_RECEIVED_IT: u16 = 0x0002;

    pub const PHYSTS_AUTONEGO_DONE: u16 = 0x010;
    pub const PHYSTS_HCDSPEEDMASK: u16 = 0x006;
    pub const PHYSTS_10BT_HD: u16 = 0x002;
    pub const PHYSTS_10BT_FD: u16 = 0x006;
    pub const PHYSTS_100BTX_HD: u16 = 0x000;
    pub const PHYSTS_100BTX_FD: u16 = 0x004;

    pub const STATUS_READ_ERROR: i32 = -5;
    pub const STATUS_WRITE_ERROR: i32 = -4;
    pub const STATUS_ADDRESS_ERROR: i32 = -3;
    pub const STATUS_RESET_TIMEOUT: i32 = -2;
    pub const STATUS_ERROR: i32 = -1;
    pub const STATUS_OK: i32 = 0;
    pub const STATUS_LINK_DOWN: i32 = 1;
    pub const STATUS_100MBITS_FULLDUPLEX: i32 = 2;
    pub const STATUS_100MBITS_HALFDUPLEX: i32 = 3;
    pub const STATUS_10MBITS_FULLDUPLEX: i32 = 4;
    pub const STATUS_10MBITS_HALFDUPLEX: i32 = 5;
    pub const STATUS_AUTONEGO_NOTDONE: i32 = 6;
}
use self::phy_consts::*;

/// SMSC DP83848 Ethernet PHY
pub struct DP83848<MAC: StationManagement> {
    mac: MAC,
}

impl<MAC: StationManagement> PHY for DP83848<MAC> {
    /// Reset PHY and wait for it to come out of reset.
    fn phy_reset(&mut self) {
        self.mac.smi_write(REG_BMCR, BMCR_SOFT_RESET);
        while (self.mac.smi_read(REG_BMCR) & BMCR_SOFT_RESET) == BMCR_SOFT_RESET
        {}
    }

    /// PHY initialisation.
    fn phy_init(&mut self) {
        // let mut micr = self.mac.smi_read(REG_MICR);
        // micr |= PHY_MICR_INT_EN as u32 | PHY_MICR_INT_OE;
        // if(!self.mac.smi_write(REG_MICR, micr))
        // {
        //     /* Return ERROR in case of write timeout */
        //     return ETH_ERROR;
        // }

        // let mut misr = self.mac.smi_read(REG_MISR);
        // misr |= PHY_MISR_LINK_INT_EN as u32;
        // if(!self.mac.smi_write(REG_MISR, misr))
        // {
        //     /* Return ERROR in case of write timeout */
        //     return ETH_ERROR;
        // }

        //enable powerdown
        // BMCR_POWER_DOWN === 0 -> normal, 1 -> power down

        // manual select
        // BMCR_DUPLEX_MODE === 0 -> Half, 1 -> Full
        // BMCR_SPEED_SELECT === 0 -> 10M, 1 -> 100M
        self.mac.smi_write(
            REG_BMCR,
            BMCR_DUPLEX_MODE | BMCR_SPEED_SELECT
        );

        // Enable auto-negotiation
        // self.mac.smi_write(
        //     REG_BMCR,
        //     BMCR_AUTONEGO_EN | BMCR_RESTART_AUTONEGO
        // );
    }
}

/// Public functions for the DP83848
impl<MAC: StationManagement> DP83848<MAC> {
    /// Create DP83848 instance from ETHMAC peripheral
    pub fn new(mac: MAC) -> Self {
        DP83848 { mac }
    }
    /// Returns a reference to the inner ETHMAC peripheral
    pub fn inner(&self) -> &MAC {
        &self.mac
    }
    /// Returns a mutable reference to the inner ETHMAC peripheral
    pub fn inner_mut(&mut self) -> &mut MAC {
        &mut self.mac
    }
    /// Releases the ETHMAC peripheral
    pub fn free(self) -> MAC {
        self.mac
    }

    /// Poll PHY to determine link status.
    pub fn poll_link(&mut self) -> i32 {
        let bmsr = self.mac.smi_read(REG_BMSR);

        // No link if link is down
        if bmsr & BMSR_LINK_STATUS == 0 {
            return STATUS_LINK_DOWN;
        }
        // No link if other side isn't 100Mbps full duplex
        if bmsr & BMSR_100BASE_TX_FD == 0 {
            return STATUS_ERROR;
        }

        // Got link
        STATUS_OK
    }

    pub fn link_established(&mut self) -> i32 {
        self.poll_link()
    }

    pub fn block_until_link(&mut self) {
        while self.link_established() != STATUS_OK {}
    }

    pub fn check_phy_status(&mut self) -> u16 {
        self.mac.smi_read(REG_BMSR)
    }
}
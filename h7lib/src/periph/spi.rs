// Note: This module contains lots of C+P from stm32h7xx-hal.

use core::{cell::UnsafeCell, ops::Deref, ptr};

// Used for while loops, to allow returning an error instead of hanging.
const MAX_ITERS: u32 = 300_000; // todo: What should this be?

use crate::pac;

#[derive(Copy, Clone, Debug)]
pub enum Error {
    /// Overrun occurred
    Overrun,
    /// Mode fault occurred
    ModeFault,
    /// CRC error
    Crc,
    Hardware,
    DuplexFailed, // todo temp?
}

#[derive(Clone, Copy, PartialEq)]
/// Select the duplex communication mode between the 2 devices. Sets `CR1` register, `BIDIMODE`,
/// and `RXONLY` fields.
pub enum CommMode {
    FullDuplex,
    HalfDuplex,
    /// Simplex Transmit only. (Cfg same as Full Duplex, but ignores input)
    TransmitOnly,
    /// Simplex Receive only.
    ReceiveOnly,
}

#[derive(Clone, Copy, PartialEq)]
/// Used for managing NSS / CS pin. Sets CR1 register, SSM field.
/// On H7, sets CFG2 register, `SSOE` field.
pub enum SlaveSelect {
    ///  In this configuration, slave select information
    /// is driven internally by the SSI bit value in register SPIx_CR1. The external NSS pin is
    /// free for other application uses.
    Software,
    /// This configuration is only used when the
    /// MCU is set as master. The NSS pin is managed by the hardware. The NSS signal
    /// is driven low as soon as the SPI is enabled in master mode (SPE=1), and is kept
    /// low until the SPI is disabled (SPE =0). A pulse can be generated between
    /// continuous communications if NSS pulse mode is activated (NSSP=1). The SPI
    /// cannot work in multimaster configuration with this NSS setting.
    HardwareOutEnable,
    /// If the microcontroller is acting as the
    /// master on the bus, this configuration allows multimaster capability. If the NSS pin
    /// is pulled low in this mode, the SPI enters master mode fault state and the device is
    /// automatically reconfigured in slave mode. In slave mode, the NSS pin works as a
    /// standard “chip select” input and the slave is selected while NSS line is at low level.
    HardwareOutDisable,
}

#[derive(Clone, Copy)]
#[repr(u8)]
/// Clock polarity. Sets CFGR2 register, CPOL field. Stored in the config as a field of `SpiMode`.
pub enum Polarity {
    /// Clock signal low when idle
    IdleLow = 0,
    /// Clock signal high when idle
    IdleHigh = 1,
}

#[derive(Clone, Copy)]
#[repr(u8)]
/// Clock phase. Sets CFGR2 register, CPHA field. Stored in the config as a field of `SpiMode`.
pub enum Phase {
    /// Data in "captured" on the first clock transition
    CaptureOnFirstTransition = 0,
    /// Data in "captured" on the second clock transition
    CaptureOnSecondTransition = 1,
}

#[derive(Clone, Copy)]
/// SPI mode. Sets CFGR2 reigster, CPOL and CPHA fields.
pub struct SpiMode {
    /// Clock polarity
    pub polarity: Polarity,
    /// Clock phase
    pub phase: Phase,
}

impl SpiMode {
    /// Set Spi Mode 0: Idle low, capture on first transition.
    /// Data sampled on rising edge and shifted out on the falling edge
    pub fn mode0() -> Self {
        Self {
            polarity: Polarity::IdleLow,
            phase: Phase::CaptureOnFirstTransition,
        }
    }

    /// Set Spi Mode 1: Idle low, capture on second transition.
    /// Data sampled on the falling edge and shifted out on the rising edge
    pub fn mode1() -> Self {
        Self {
            polarity: Polarity::IdleLow,
            phase: Phase::CaptureOnSecondTransition,
        }
    }

    /// Set Spi Mode 2: Idle high, capture on first transition.
    /// Data sampled on the rising edge and shifted out on the falling edge
    pub fn mode2() -> Self {
        Self {
            polarity: Polarity::IdleHigh,
            phase: Phase::CaptureOnFirstTransition,
        }
    }

    /// Set Spi Mode 3: Idle high, capture on second transition.
    /// Data sampled on the falling edge and shifted out on the rising edge
    pub fn mode3() -> Self {
        Self {
            polarity: Polarity::IdleHigh,
            phase: Phase::CaptureOnSecondTransition,
        }
    }
}

type SpiModeType = SpiMode;

/// Set the factor to divide the APB clock by to set baud rate. Sets `SPI_CR1` register, `BR` field.
/// On H7, sets CFG1 register, `MBR` field.
#[derive(Copy, Clone)]
#[repr(u8)]
pub enum BaudRate {
    Div2 = 0b000,
    Div4 = 0b001,
    Div8 = 0b010,
    Div16 = 0b011,
    Div32 = 0b100,
    Div64 = 0b101,
    Div128 = 0b110,
    Div256 = 0b111,
}

#[derive(Copy, Clone)]
pub enum Interrupt {
    /// Additional number of transactions reload interrupt enable (TSERFIE)
    NumberOfTransactionsReload,
    ModeFault,
    Tifre,
    CrcError,
    Overrun,
    Underrun,
    Txtfie,
    /// EOT, SUSP, and TXC (EOTIE)
    EotSuspTxc,
    Dxp,
    Txp,
    Rxp,
}

#[derive(Clone, Copy)]
#[repr(u8)]
/// FIFO reception threshold Sets `SPI_CR2` register, `FRXTH` field.
pub enum ReceptionThresh {
    /// RXNE event is generated if the FIFO level is greater than or equal to 1/2 (16-bit)
    D16 = 0,
    /// RXNE event is generated if the FIFO level is greater than or equal to 1/4 (8-bit)
    D8 = 1,
}

#[derive(Clone)]
/// Configuration data for SPI.
pub struct SpiConfig {
    /// SPI mode associated with Polarity and Phase. Defaults to Mode0: Idle low, capture on first transition.
    pub mode: SpiModeType,
    /// Sets the (duplex) communication mode between the devices. Defaults to full duplex.
    pub comm_mode: CommMode,
    /// Controls use of hardware vs software CS/NSS pin. Defaults to software.
    pub slave_select: SlaveSelect,
    /// Data size. Defaults to 8 bits.
    pub data_size: u8,
    /// FIFO reception threshhold. Defaults to 8 bits.
    pub fifo_reception_thresh: ReceptionThresh,
    // pub cs_delay: f32,
    // pub swap_miso_mosi: bool,
    // pub suspend_when_inactive: bool,
    pub band_rate: BaudRate,
}

impl Default for SpiConfig {
    fn default() -> Self {
        Self {
            mode: SpiModeType::mode0(),
            comm_mode: CommMode::FullDuplex,
            slave_select: SlaveSelect::Software,
            data_size: 8,
            fifo_reception_thresh: ReceptionThresh::D8,
            band_rate: BaudRate::Div128,
        }
    }
}

// Depth of FIFO to use. See RM0433 Rev 7, Table 409. Note that 16 is acceptable on this MCU,
// for SPI 1-3
const FIFO_LEN: usize = 8;

pub struct Spi<const N: usize> {
    periph_reg: *const pac::spi1::RegisterBlock,
    pub cfg: SpiConfig,
}

impl<const N: usize> Spi<N> {
    /// Initialize an SPI peripheral, including configuration register writes, and enabling and resetting
    /// its RCC peripheral clock.
    pub fn new(cfg: SpiConfig) -> Self {
        let periph_reg: *const pac::spi1::RegisterBlock = match N {
            1 => pac::SPI1::ptr(),
            2 => pac::SPI2::ptr(),
            3 => pac::SPI3::ptr(),
            _ => panic!("Unsupported SPI number"),
        };
        let rcc = unsafe { &(*pac::RCC::ptr()) };
        let periph = unsafe { &(*periph_reg)};

        // Enable clock for SPI
        let _ = prec.enable(); // drop, can be recreated by free method

        // Disable SS output
        spi.cfg2.write(|w| w.ssoe().disabled());

        // H743 RM, section 50.4.8: Configuration of SPI.
        // 1. Write the proper GPIO registers: Configure GPIO for MOSI, MISO and SCK pins.
        // (Handled in application code)

        // 2. Write to the SPI_CFG1 and SPI_CFG2 registers to set up proper values of all not
        // reserved bits and bit fields included there with next exceptions:
        // a) SSOM, SSOE, MBR[2:0], MIDI[3:0] and MSSI[3:0] are required at master mode
        // only, the MSSI bits take effect when SSOE is set, MBR setting is required for slave
        // at TI mode, too
        // b) UDRDET[1:0] and UDRCFG[1:0] are required at slave mode only,
        // c) CRCSIZE[4:0] is required if CRCEN is set,
        // d) CPOL, CPHA, LSBFRST, SSOM, SSOE, SSIOP and SSM are not required at TI
        // mode.
        // e) Once the AFCNTR bit is set at SPI_CFG2 register, all the SPI outputs start to be
        // propagated onto the associated GPIO pins regardless the peripheral enable so
        // any later configurations changes of the SPI_CFG1 and SPI_CFG2 registers can
        // affect level of signals at these pins.
        // f) The I2SMOD bit at SPI_I2SCFGR register has to be kept cleared to prevent any
        // unexpected influence of occasional I2S configuration.

        // [St forum thread on how to set up SPI in master mode avoiding mode faults:
        // https://community.st.com/s/question/0D50X0000AFrHS6SQN/stm32h7-what-is-the-proper-
        // way-to-make-spi-work-in-master-mode
        regs.cr1
            .modify(|_, w| w.ssi().bit(cfg.slave_select == SlaveSelect::Software));

        regs.cfg1.modify(|_, w| {
            w.mbr().bits(baud_rate as u8);
            w.dsize().bits((cfg.data_size - 1) as u8);
            w.crcen().clear_bit()
        });

        // Specifies minimum time delay (expressed in SPI clock cycles periods) inserted between two
        // consecutive data frames in master mode. In clock cycles; 0 - 15. (hardware CS)
        let inter_word_delay = 0;

        regs.cfg2.modify(|_, w| {
            w.cpol().bit(cfg.mode.polarity as u8 != 0);
            w.cpha().bit(cfg.mode.phase as u8 != 0);
            w.master().set_bit();
            w.ssm().bit(cfg.slave_select == SlaveSelect::Software);
            w.ssoe().bit(cfg.slave_select != SlaveSelect::Software);
            w.midi().bits(inter_word_delay);
            w.master().set_bit();
            w.comm().bits(0b00) // Full-duplex mode
        });

        // 3. Write to the SPI_CR2 register to select length of the transfer, if it is not known TSIZE
        // has to be programmed to zero.
        // Resetting this here; will be set to the appropriate value at each transaction.
        regs.cr2.modify(|_, w| w.tsize().bits(0));

        // 4. Write to SPI_CRCPOLY and into TCRCINI, RCRCINI and CRC33_17 bits at
        // SPI2S_CR1 register to configure the CRC polynomial and CRC calculation if needed.

        // 5. Configure DMA streams dedicated for the SPI Tx and Rx in DMA registers if the DMA
        // streams are used (see chapter Communication using DMA).

        // 6. Program the IOLOCK bit in the SPI_CFG1 register if the configuration protection is
        // required (for safety).

        regs.cr1.modify(|_, w| w.spe().set_bit());

        Self { regs, cfg }
    }

    /// Change the SPI baud rate.
    pub fn reclock(&mut self, baud_rate: BaudRate) {
        self.regs.cr1.modify(|_, w| w.spe().clear_bit());

        self.regs
            .cfg1
            .modify(|_, w| unsafe { w.mbr().bits(baud_rate as u8) });

        self.regs.cr1.modify(|_, w| w.spe().set_bit());
    }

    /// L44 RM, section 40.4.9: "Procedure for disabling the SPI"
    /// When SPI is disabled, it is mandatory to follow the disable procedures described in this
    /// paragraph. It is important to do this before the system enters a low-power mode when the
    /// peripheral clock is stopped. Ongoing transactions can be corrupted in this case. In some
    /// modes the disable procedure is the only way to stop continuous communication running.
    pub fn disable(&mut self) {
        // The correct disable procedure is (except when receive only mode is used):
        // 1. Wait until TXC=1 and/or EOT=1 (no more data to transmit and last data frame sent).
        // When CRC is used, it is sent automatically after the last data in the block is processed.
        // TXC/EOT is set when CRC frame is completed in this case. When a transmission is
        // suspended the software has to wait till CSTART bit is cleared.
        while self.regs.sr.read().txc().bit_is_clear() {}
        while self.regs.sr.read().eot().bit_is_clear() {}
        // 2. Read all RxFIFO data (until RXWNE=0 and RXPLVL=00)
        while self.regs.sr.read().rxwne().bit_is_set() || self.regs.sr.read().rxplvl().bits() != 0 {
            unsafe { ptr::read_volatile(&self.regs.rxdr as *const _ as *const u8) };
        }
        // 3. Disable the SPI (SPE=0).
        self.regs.cr1.modify(|_, w| w.spe().clear_bit());
    }

    // todo: Temp C+P from h7xx hal while troubleshooting.
    /// Internal implementation for exchanging a word
    ///
    /// * Assumes the transaction has started (CSTART handled externally)
    /// * Assumes at least one word has already been written to the Tx FIFO
    fn exchange(&mut self, word: u8) -> Result<u8, SpiError> {
        let status = self.regs.sr.read();
        check_errors!(status);

        let mut i = 0;
        while !self.regs.sr.read().dxp().is_available() {
            i += 1;
            if i >= MAX_ITERS {
                return Err(SpiError::Hardware);
            }
        }

        // NOTE(write_volatile/read_volatile) write/read only 1 word
        unsafe {
            let txdr = &self.regs.txdr as *const _ as *const UnsafeCell<u8>;
            ptr::write_volatile(UnsafeCell::raw_get(txdr), word);
            return Ok(ptr::read_volatile(&self.regs.rxdr as *const _ as *const u8));
        }
    }
    /// Read a single byte if available, or block until it's available.
    ///
    /// Assumes the transaction has started (CSTART handled externally)
    /// Assumes at least one word has already been written to the Tx FIFO
    pub fn read(&mut self) -> Result<u8, SpiError> {
        check_errors!(self.regs.sr.read());

        let mut i = 0;
        while !self.regs.sr.read().rxp().is_not_empty() {
            i += 1;
            if i >= MAX_ITERS {
                return Err(SpiError::Hardware);
            }
        }

        // NOTE(read_volatile) read only 1 word
        return Ok(unsafe { ptr::read_volatile(&self.regs.rxdr as *const _ as *const u8) });
    }

    /// Write multiple bytes on the SPI line, blocking until complete.
    pub fn write(&mut self, write_words: &[u8]) -> Result<(), SpiError> {
        // both buffers are the same length
        if write_words.is_empty() {
            return Ok(());
        }

        // Fill the first half of the write FIFO
        let len = write_words.len();
        let mut write = write_words.iter();
        for _ in 0..core::cmp::min(FIFO_LEN, len) {
            self.send(*write.next().unwrap());
        }

        // Continue filling write FIFO and emptying read FIFO
        for word in write {
            let _ = self.exchange(*word);
        }

        // Dummy read from the read FIFO
        for _ in 0..core::cmp::min(FIFO_LEN, len) {
            let _ = self.read();
        }

        Ok(())
    }

    /// Read multiple bytes to a buffer, blocking until complete.
    pub fn transfer(&mut self, words: &mut [u8]) -> Result<(), SpiError> {
        if words.is_empty() {
            return Ok(());
        }

        // Fill the first half of the write FIFO
        let len = words.len();
        for i in 0..core::cmp::min(FIFO_LEN, len) {
            self.send(words[i]);
        }

        for i in FIFO_LEN..len + FIFO_LEN {
            if i < len {
                // Continue filling write FIFO and emptying read FIFO
                let read_value = self.exchange(words[i])?;

                words[i - FIFO_LEN] = read_value;
            } else {
                // Finish emptying the read FIFO
                words[i - FIFO_LEN] = self.read()?;
            }
        }

        Ok(())
    }

    fn send(&mut self, word: u8) -> Result<(), SpiError> {
        check_errors!(self.regs.sr.read());

        // NOTE(write_volatile) see note above
        unsafe {
            let txdr = &self.regs.txdr as *const _ as *const UnsafeCell<u8>;
            ptr::write_volatile(UnsafeCell::raw_get(txdr), word)
        }
        // write CSTART to start a transaction in
        // master mode
        self.regs.cr1.modify(|_, w| w.cstart().started());

        return Ok(());
    }

    /// Receive data using DMA. See H743 RM, section 50.4.14: Communication using DMA
    pub unsafe fn read_dma(
        &mut self,
        buf: &mut [u8],
        channel: DmaChannel,
        channel_cfg: ChannelCfg,
        dma_periph: dma::DmaPeriph,
    ) {
        // todo: Accept u16 and u32 words too.
        let (ptr, len) = (buf.as_mut_ptr(), buf.len());

        self.regs.cr1.modify(|_, w| w.spe().clear_bit());
        self.regs.cfg1.modify(|_, w| w.rxdmaen().set_bit());

        let periph_addr = &self.regs.rxdr as *const _ as u32;
        let num_data = len as u32;

        match dma_periph {
            dma::DmaPeriph::Dma1 => {
                let mut regs = unsafe { &(*DMA1::ptr()) };
                dma::cfg_channel(
                    &mut regs,
                    channel,
                    periph_addr,
                    ptr as u32,
                    num_data,
                    dma::Direction::ReadFromPeriph,
                    dma::DataSize::S8,
                    dma::DataSize::S8,
                    channel_cfg,
                );
            }

            dma::DmaPeriph::Dma2 => {
                let mut regs = unsafe { &(*pac::DMA2::ptr()) };
                dma::cfg_channel(
                    &mut regs,
                    channel,
                    periph_addr,
                    ptr as u32,
                    num_data,
                    dma::Direction::ReadFromPeriph,
                    dma::DataSize::S8,
                    dma::DataSize::S8,
                    channel_cfg,
                );
            }
        }

        self.regs.cr1.modify(|_, w| w.spe().set_bit());
        self.regs.cr1.modify(|_, w| w.cstart().set_bit()); // Must be separate from SPE enable.
    }

    pub unsafe fn write_dma(
        &mut self,
        buf: &[u8],
        channel: DmaChannel,
        channel_cfg: ChannelCfg,
        dma_periph: dma::DmaPeriph,
    ) {
        // Static write and read buffers?
        let (ptr, len) = (buf.as_ptr(), buf.len());

        self.regs.cr1.modify(|_, w| w.spe().clear_bit());

        // todo: Accept u16 words too.

        // A DMA access is requested when the TXE or RXNE enable bit in the SPIx_CR2 register is
        // set. Separate requests must be issued to the Tx and Rx buffers.
        // In transmission, a DMA request is issued each time TXE is set to 1. The DMA then
        // writes to the SPIx_DR register.

        // When starting communication using DMA, to prevent DMA channel management raising
        // error events, these steps must be followed in order:
        //
        // 1. Enable DMA Rx buffer in the RXDMAEN bit in the SPI_CR2 register, if DMA Rx is
        // used.
        // (N/A)

        // 2. Enable DMA streams for Tx and Rx in DMA registers, if the streams are used.
        let periph_addr = &self.regs.txdr as *const _ as u32;
        let num_data = len as u32;

        match dma_periph {
            dma::DmaPeriph::Dma1 => {
                let mut regs = unsafe { &(*DMA1::ptr()) };
                dma::cfg_channel(
                    &mut regs,
                    channel,
                    periph_addr,
                    ptr as u32,
                    num_data,
                    dma::Direction::ReadFromMem,
                    dma::DataSize::S8,
                    dma::DataSize::S8,
                    channel_cfg,
                );
            }
            dma::DmaPeriph::Dma2 => {
                let mut regs = unsafe { &(*pac::DMA2::ptr()) };
                dma::cfg_channel(
                    &mut regs,
                    channel,
                    periph_addr,
                    ptr as u32,
                    num_data,
                    dma::Direction::ReadFromMem,
                    dma::DataSize::S8,
                    dma::DataSize::S8,
                    channel_cfg,
                );
            }
        }

        // 3. Enable DMA Tx buffer in the TXDMAEN bit in the SPI_CR2 register, if DMA Tx is used.
        self.regs.cfg1.modify(|_, w| w.txdmaen().set_bit());

        // 4. Enable the SPI by setting the SPE bit.
        self.regs.cr1.modify(|_, w| w.spe().set_bit());
        self.regs.cr1.modify(|_, w| w.cstart().set_bit()); // Must be separate from SPE enable.
    }

    /// Transfer data from DMA; this is the basic reading API, using both write and read transfers:
    /// It performs a write with register data, and reads to a buffer.
    pub unsafe fn transfer_dma(
        &mut self,
        buf_write: &[u8],
        buf_read: &mut [u8],
        channel_write: DmaChannel,
        channel_read: DmaChannel,
        channel_cfg_write: ChannelCfg,
        channel_cfg_read: ChannelCfg,
        dma_periph: dma::DmaPeriph,
    ) {
        // todo: Accept u16 and u32 words too.
        let (ptr_write, len_write) = (buf_write.as_ptr(), buf_write.len());
        let (ptr_read, len_read) = (buf_read.as_mut_ptr(), buf_read.len());

        self.regs.cr1.modify(|_, w| w.spe().clear_bit());

        // todo: DRY here, with `write_dma`, and `read_dma`.

        let periph_addr_write = &self.regs.txdr as *const _ as u32;
        let periph_addr_read = &self.regs.rxdr as *const _ as u32;

        let num_data_write = len_write as u32;
        let num_data_read = len_read as u32;

        // Be careful - order of enabling Rx and Tx may matter, along with other things like when we
        // enable the channels, and the SPI periph.
        self.regs.cfg1.modify(|_, w| w.rxdmaen().set_bit());

        match dma_periph {
            dma::DmaPeriph::Dma1 => {
                let mut regs = unsafe { &(*DMA1::ptr()) };
                dma::cfg_channel(
                    &mut regs,
                    channel_write,
                    periph_addr_write,
                    ptr_write as u32,
                    num_data_write,
                    dma::Direction::ReadFromMem,
                    dma::DataSize::S8,
                    dma::DataSize::S8,
                    channel_cfg_write,
                );

                dma::cfg_channel(
                    &mut regs,
                    channel_read,
                    periph_addr_read,
                    ptr_read as u32,
                    num_data_read,
                    dma::Direction::ReadFromPeriph,
                    dma::DataSize::S8,
                    dma::DataSize::S8,
                    channel_cfg_read,
                );
            }

            dma::DmaPeriph::Dma2 => {
                let mut regs = unsafe { &(*pac::DMA2::ptr()) };
                dma::cfg_channel(
                    &mut regs,
                    channel_write,
                    periph_addr_write,
                    ptr_write as u32,
                    num_data_write,
                    dma::Direction::ReadFromMem,
                    dma::DataSize::S8,
                    dma::DataSize::S8,
                    channel_cfg_write,
                );

                dma::cfg_channel(
                    &mut regs,
                    channel_read,
                    periph_addr_read,
                    ptr_read as u32,
                    num_data_read,
                    dma::Direction::ReadFromPeriph,
                    dma::DataSize::S8,
                    dma::DataSize::S8,
                    channel_cfg_read,
                );
            }
        }

        self.regs.cfg1.modify(|_, w| w.txdmaen().set_bit());

        self.regs.cr1.modify(|_, w| w.spe().set_bit());
        self.regs.cr1.modify(|_, w| w.cstart().set_bit()); // Must be separate from SPE enable.
    }

    /// Enable an interrupt.
    pub fn enable_interrupt(&mut self, interrupt_type: SpiInterrupt) {
        self.regs.ier.modify(|_, w| match interrupt_type {
            SpiInterrupt::NumberOfTransactionsReload => w.tserfie().set_bit(),
            SpiInterrupt::ModeFault => w.modfie().set_bit(),
            SpiInterrupt::Tifre => w.tifreie().set_bit(),
            SpiInterrupt::CrcError => w.crceie().set_bit(),
            SpiInterrupt::Overrun => w.ovrie().set_bit(),
            SpiInterrupt::Underrun => w.udrie().set_bit(),
            SpiInterrupt::Txtfie => w.txtfie().set_bit(),
            SpiInterrupt::EotSuspTxc => w.eotie().set_bit(),
            // SpiInterrupt::Dxp => w.dxpie().set_bit(),
            // SpiInterrupt::Txp => w.txpie().set_bit(),
            // SpiInterrupt::Rxp => w.rxpie().set_bit(),
            _ => w.eotie().set_bit(), // todo: PAC ommission?
        });
    }

    /// Clear an interrupt.
    pub fn clear_interrupt(&mut self, interrupt_type: SpiInterrupt) {
        self.regs.ifcr.write(|w| match interrupt_type {
            SpiInterrupt::NumberOfTransactionsReload => w.tserfc().set_bit(),
            SpiInterrupt::ModeFault => w.modfc().set_bit(),
            SpiInterrupt::Tifre => w.tifrec().set_bit(),
            SpiInterrupt::CrcError => w.crcec().set_bit(),
            SpiInterrupt::Overrun => w.ovrc().set_bit(),
            SpiInterrupt::Underrun => w.udrc().set_bit(),
            SpiInterrupt::Txtfie => w.txtfc().set_bit(),
            SpiInterrupt::EotSuspTxc => w.eotc().set_bit(),
            // SpiInterrupt::Dxp => w.dxpc().set_bit(),
            // SpiInterrupt::Txp => w.txpc().set_bit(),
            // SpiInterrupt::Rxp => w.rxpc().set_bit(),
            _ => w.eotc().set_bit(), // todo: PAC ommission?
        });
    }

    /// Stop a DMA transfer. Stops the channel, and disables the `txdmaen` and `rxdmaen` bits.
    /// Run this after each transfer completes - you may wish to do this in an interrupt
    /// (eg DMA transfer complete) instead of blocking. `channel2` is an optional second channel
    /// to stop; eg if you have both a tx and rx channel.
    pub fn stop_dma(
        &mut self,
        channel: DmaChannel,
        channel2: Option<DmaChannel>,
        dma_periph: dma::DmaPeriph,
    ) {
        // (RM:) To close communication it is mandatory to follow these steps in order:
        // 1. Disable DMA streams for Tx and Rx in the DMA registers, if the streams are used.

        dma::stop(dma_periph, channel);
        if let Some(ch2) = channel2 {
            dma::stop(dma_periph, ch2);
        };

        // 2. Disable the SPI by following the SPI disable procedure:
        // self.disable();

        // 3. Disable DMA Tx and Rx buffers by clearing the TXDMAEN and RXDMAEN bits in the
        // SPI_CR2 register, if DMA Tx and/or DMA Rx are used.

        #[cfg(not(feature = "h7"))]
        self.regs.cr2.modify(|_, w| {
            w.txdmaen().clear_bit();
            w.rxdmaen().clear_bit()
        });

        #[cfg(feature = "h7")]
        self.regs.cfg1.modify(|_, w| {
            w.txdmaen().clear_bit();
            w.rxdmaen().clear_bit()
        });
    }

    /// Convenience function that clears the interrupt, and stops the transfer. For use with the TC
    /// interrupt only.
    pub fn cleanup_dma(
        &mut self,
        dma_periph: dma::DmaPeriph,
        channel_tx: DmaChannel,
        channel_rx: Option<DmaChannel>,
    ) {
        // The hardware seems to automatically enable Tx too; and we use it when transmitting.
        dma::clear_interrupt(dma_periph, channel_tx, dma::DmaInterrupt::TransferComplete);

        if let Some(ch_rx) = channel_rx {
            dma::clear_interrupt(dma_periph, ch_rx, dma::DmaInterrupt::TransferComplete);
        }

        self.stop_dma(channel_tx, channel_rx, dma_periph);
    }

    /// Print the (raw) contents of the status register.
    pub fn read_status(&self) -> u32 {
        unsafe { self.regs.sr.read().bits() }
    }
}

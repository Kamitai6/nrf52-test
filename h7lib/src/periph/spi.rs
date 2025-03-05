// Note: This module contains lots of C+P from stm32h7xx-hal.

use core::{cell::UnsafeCell, ptr};

// Used for while loops, to allow returning an error instead of hanging.
const MAX_ITERS: u32 = 300_000; // todo: What should this be?

use crate::{pac, rcc_en_reset};

use super::dma;
use super::gpio;

#[derive(Copy, Clone, Debug)]
#[repr(u8)]
pub enum Error {
    Overrun = 0,
    ModeFault,
    Crc,
    Hardware,
    DuplexFailed, // todo temp?
}

macro_rules! check_errors {
    ($sr:expr) => {
        let crc_error = $sr.crce().bit_is_set();

        if $sr.ovr().bit_is_set() {
            return Err(Error::Overrun);
        } else if $sr.modf().bit_is_set() {
            return Err(Error::ModeFault);
        } else if crc_error {
            return Err(Error::Crc);
        }
    };
}

#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
/// Select the duplex communication mode between the 2 devices. Sets `CR1` register, `BIDIMODE`,
/// and `RXONLY` fields.
pub enum CommMode {
    FullDuplex = 0,
    HalfDuplex,
    TransmitOnly,
    ReceiveOnly,
}

#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
/// Used for managing NSS / CS pin. Sets CR1 register, SSM field.
/// On H7, sets CFG2 register, `SSOE` field.
pub enum SlaveSelect {
    ///  In this configuration, slave select information
    /// is driven internally by the SSI bit value in register SPIx_CR1. The external NSS pin is
    /// free for other application uses.
    Software = 0,
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
    IdleLow = 0,
    IdleHigh = 1,
}

#[derive(Clone, Copy)]
#[repr(u8)]
/// Clock phase. Sets CFGR2 register, CPHA field. Stored in the config as a field of `SpiMode`.
pub enum Phase {
    CaptureOnFirstTransition = 0,
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
#[repr(u8)]
pub enum Interrupt {
    /// Additional number of transactions reload interrupt enable (TSERFIE)
    NumberOfTransactionsReload = 0,
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

/// Number of bits in at single SPI data frame. Sets `CFGR1` register, `DSIZE` field.
#[derive(Copy, Clone)]
#[repr(u8)]
pub enum DataSize {
    D4 = 3,
    D5 = 4,
    D6 = 5,
    D7 = 6,
    D8 = 7,
    D9 = 8,
    D10 = 9,
    D11 = 10,
    D12 = 11,
    D13 = 12,
    D14 = 13,
    D15 = 14,
    D16 = 15,
    D17 = 16,
    D18 = 17,
    D19 = 18,
    D20 = 19,
    D21 = 20,
    D22 = 21,
    D23 = 22,
    D24 = 23,
    D25 = 24,
    D26 = 25,
    D27 = 26,
    D28 = 27,
    D29 = 28,
    D30 = 29,
    D31 = 30,
    D32 = 31,
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
    pub mode: SpiMode,
    /// Sets the (duplex) communication mode between the devices. Defaults to full duplex.
    pub comm_mode: CommMode,
    /// Controls use of hardware vs software CS/NSS pin. Defaults to software.
    pub slave_select: SlaveSelect,
    /// Data size. Defaults to 8 bits.
    pub data_size: DataSize,
    /// FIFO reception threshhold. Defaults to 8 bits.
    pub fifo_reception_thresh: ReceptionThresh,
    // pub cs_delay: f32,
    // pub swap_miso_mosi: bool,
    // pub suspend_when_inactive: bool,
    pub baud_rate: BaudRate,
}

impl Default for SpiConfig {
    fn default() -> Self {
        Self {
            mode: SpiMode::mode0(),
            comm_mode: CommMode::FullDuplex,
            slave_select: SlaveSelect::Software,
            data_size: DataSize::D8,
            fifo_reception_thresh: ReceptionThresh::D8,
            baud_rate: BaudRate::Div128,
        }
    }
}

// Depth of FIFO to use. See RM0433 Rev 7, Table 409. Note that 16 is acceptable on this MCU,
// for SPI 1-3
const FIFO_LEN: u8 = 8;

pub struct Spi<const N: u8> {
    regs_ptr: *const pac::spi1::RegisterBlock,
    pub cfg: SpiConfig,
}

impl<const N: u8> Spi<N> {
    const CHECK: () = {
        assert!(1 <= N && N <= 6, "Spi must be 1 - 6.");
    };
    /// Initialize an SPI peripheral, including configuration register writes, and enabling and resetting
    /// its RCC peripheral clock.
    pub fn init<
        const SCK_PORT: char, const SCK_PIN: u8, 
        const MISO_PORT: char, const MISO_PIN: u8, 
        const MOSI_PORT: char, const MOSI_PIN: u8
    >(
        sck: gpio::GPIO<SCK_PORT, SCK_PIN>, 
        miso: gpio::GPIO<MISO_PORT, MISO_PIN>, 
        mosi: gpio::GPIO<MOSI_PORT, MOSI_PIN>, 
        cfg: SpiConfig
    ) -> Self {
        let _ = Self::CHECK;

        assert!(
            matches!(sck.mode, gpio::PinMode::AltFn(5, gpio::OutputType::PushPull)) && 
            matches!(miso.mode, gpio::PinMode::AltFn(5, gpio::OutputType::PushPull)) && 
            matches!(mosi.mode, gpio::PinMode::AltFn(5, gpio::OutputType::PushPull)), "Mode is not AltFn 5 push-pull"
        );

        assert!(
            match N {
                1 => {
                    ((SCK_PORT == 'A' && SCK_PIN == 5) || (SCK_PORT == 'B' && SCK_PIN == 3) || (SCK_PORT == 'G' && SCK_PIN == 11)) &&
                    ((MISO_PORT == 'A' && MISO_PIN == 6) || (MISO_PORT == 'B' && MISO_PIN == 4) || (MISO_PORT == 'G' && MISO_PIN == 9)) &&
                    ((MOSI_PORT == 'A' && MOSI_PIN == 7) || (MOSI_PORT == 'B' && MOSI_PIN == 5) || (MOSI_PORT == 'D' && MOSI_PIN == 7))
                },
                2 => {
                    ((SCK_PORT == 'A' && SCK_PIN == 9) || (SCK_PORT == 'A' && SCK_PIN == 12) || (SCK_PORT == 'B' && SCK_PIN == 10) ||
                        (SCK_PORT == 'B' && SCK_PIN == 13) || (SCK_PORT == 'D' && SCK_PIN == 3) || (SCK_PORT == 'I' && SCK_PIN == 1)) &&
                    ((MISO_PORT == 'B' && MISO_PIN == 14) || (MISO_PORT == 'C' && MISO_PIN == 2) || (MISO_PORT == 'I' && MISO_PIN == 2)) &&
                    ((MOSI_PORT == 'C' && MOSI_PIN == 1) || (MOSI_PORT == 'C' && MOSI_PIN == 3) || (MOSI_PORT == 'I' && MOSI_PIN == 3))
                },
                3 => {
                    ((SCK_PORT == 'B' && SCK_PIN == 3) || (SCK_PORT == 'C' && SCK_PIN == 10)) &&
                    ((MISO_PORT == 'B' && MISO_PIN == 4) || (MISO_PORT == 'C' && MISO_PIN == 11)) &&
                    ((MOSI_PORT == 'B' && MOSI_PIN == 2) || (MOSI_PORT == 'B' && MOSI_PIN == 5) || (MOSI_PORT == 'C' && MOSI_PIN == 12) ||
                     (MOSI_PORT == 'D' && MOSI_PIN == 6))
                },
                4 => {
                    ((SCK_PORT == 'E' && SCK_PIN == 2) || (SCK_PORT == 'E' && SCK_PIN == 12)) &&
                    ((MISO_PORT == 'E' && MISO_PIN == 5) || (MISO_PORT == 'E' && MISO_PIN == 13)) &&
                    ((MOSI_PORT == 'E' && MOSI_PIN == 6) || (MOSI_PORT == 'E' && MOSI_PIN == 14))
                },
                5 => {
                    ((SCK_PORT == 'F' && SCK_PIN == 7) || (SCK_PORT == 'H' && SCK_PIN == 6) ||
                     (SCK_PORT == 'K' && SCK_PIN == 0)) &&
                    ((MISO_PORT == 'F' && MISO_PIN == 8) || (MISO_PORT == 'H' && MISO_PIN == 7) ||
                     (MISO_PORT == 'J' && MISO_PIN == 11)) &&
                    ((MOSI_PORT == 'F' && MOSI_PIN == 9) || (MOSI_PORT == 'F' && MOSI_PIN == 11) ||
                     (MOSI_PORT == 'J' && MOSI_PIN == 10))
                },
                6 => {
                    ((SCK_PORT == 'A' && SCK_PIN == 5) || (SCK_PORT == 'B' && SCK_PIN == 3) ||
                     (SCK_PORT == 'C' && SCK_PIN == 12) || (SCK_PORT == 'G' && SCK_PIN == 13)) &&
                    ((MISO_PORT == 'A' && MISO_PIN == 6) || (MISO_PORT == 'B' && MISO_PIN == 4) ||
                     (MISO_PORT == 'G' && MISO_PIN == 12)) &&
                    ((MOSI_PORT == 'A' && MOSI_PIN == 7) || (MOSI_PORT == 'B' && MOSI_PIN == 5) ||
                     (MOSI_PORT == 'G' && MOSI_PIN == 14))
                },
                _ => unreachable!(),
            }
        );
        
        let regs_ptr: *const pac::spi1::RegisterBlock = match N {
            1 => pac::SPI1::ptr(),
            2 => pac::SPI2::ptr(),
            3 => pac::SPI3::ptr(),
            4 => pac::SPI4::ptr(),
            5 => pac::SPI5::ptr(),
            6 => pac::SPI6::ptr(),
            _ => unreachable!(),
        };
        let periph = unsafe { &(*regs_ptr)};
        let rcc = unsafe { &(*pac::RCC::ptr())};

        // Enable clock for SPI
        match N {
            1 => rcc_en_reset!(apb2, spi1, rcc),
            2 => rcc_en_reset!(apb1, spi2, rcc),
            3 => rcc_en_reset!(apb1, spi3, rcc),
            4 => rcc_en_reset!(apb2, spi4, rcc),
            5 => rcc_en_reset!(apb2, spi5, rcc),
            6 => rcc_en_reset!(apb4, spi6, rcc),
            _ => unreachable!()
        };

        // Disable SS output
        periph.cfg2.write(|w| w.ssoe().disabled());

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
        periph.cr1
            .modify(|_, w| w.ssi().bit(cfg.slave_select == SlaveSelect::Software));

            periph.cfg1.modify(|_, w| {
            w.mbr().bits(cfg.baud_rate as u8);
            w.dsize().bits((cfg.data_size as u8) - 1);
            w.crcen().clear_bit()
        });

        // Specifies minimum time delay (expressed in SPI clock cycles periods) inserted between two
        // consecutive data frames in master mode. In clock cycles; 0 - 15. (hardware CS)
        let inter_word_delay = 0;

        periph.cfg2.modify(|_, w| {
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
        periph.cr2.modify(|_, w| w.tsize().bits(0));

        // 4. Write to SPI_CRCPOLY and into TCRCINI, RCRCINI and CRC33_17 bits at
        // SPI2S_CR1 register to configure the CRC polynomial and CRC calculation if needed.

        // 5. Configure DMA streams dedicated for the SPI Tx and Rx in DMA registers if the DMA
        // streams are used (see chapter Communication using DMA).

        // 6. Program the IOLOCK bit in the SPI_CFG1 register if the configuration protection is
        // required (for safety).

        periph.cr1.modify(|_, w| w.spe().set_bit());

        Self { regs_ptr, cfg }
    }

    /// Change the SPI baud rate.
    pub fn reclock(&mut self, baud_rate: BaudRate) {
        let periph = unsafe { &(*self.regs_ptr)};
        periph.cr1.modify(|_, w| w.spe().clear_bit());

        periph
            .cfg1
            .modify(|_, w| unsafe { w.mbr().bits(baud_rate as u8) });

        periph.cr1.modify(|_, w| w.spe().set_bit());
    }

    /// L44 RM, section 40.4.9: "Procedure for disabling the SPI"
    /// When SPI is disabled, it is mandatory to follow the disable procedures described in this
    /// paragraph. It is important to do this before the system enters a low-power mode when the
    /// peripheral clock is stopped. Ongoing transactions can be corrupted in this case. In some
    /// modes the disable procedure is the only way to stop continuous communication running.
    pub fn disable(&mut self) {
        let periph = unsafe { &(*self.regs_ptr)};
        // The correct disable procedure is (except when receive only mode is used):
        // 1. Wait until TXC=1 and/or EOT=1 (no more data to transmit and last data frame sent).
        // When CRC is used, it is sent automatically after the last data in the block is processed.
        // TXC/EOT is set when CRC frame is completed in this case. When a transmission is
        // suspended the software has to wait till CSTART bit is cleared.
        while periph.sr.read().txc().bit_is_clear() {}
        while periph.sr.read().eot().bit_is_clear() {}
        // 2. Read all RxFIFO data (until RXWNE=0 and RXPLVL=00)
        while periph.sr.read().rxwne().bit_is_set() || periph.sr.read().rxplvl().bits() != 0 {
            unsafe { ptr::read_volatile(&periph.rxdr as *const _ as *const u8) };
        }
        // 3. Disable the SPI (SPE=0).
        periph.cr1.modify(|_, w| w.spe().clear_bit());
    }

    // todo: Temp C+P from h7xx hal while troubleshooting.
    /// Internal implementation for exchanging a word
    ///
    /// * Assumes the transaction has started (CSTART handled externally)
    /// * Assumes at least one word has already been written to the Tx FIFO
    fn exchange(&mut self, word: u8) -> Result<u8, Error> {
        let periph = unsafe { &(*self.regs_ptr)};
        let status = periph.sr.read();
        check_errors!(status);

        let mut i = 0;
        while !periph.sr.read().dxp().is_available() {
            i += 1;
            if i >= MAX_ITERS {
                return Err(Error::Hardware);
            }
        }

        // NOTE(write_volatile/read_volatile) write/read only 1 word
        unsafe {
            let txdr = &periph.txdr as *const _ as *const UnsafeCell<u8>;
            ptr::write_volatile(UnsafeCell::raw_get(txdr), word);
            return Ok(ptr::read_volatile(&periph.rxdr as *const _ as *const u8));
        }
    }
    /// Read a single byte if available, or block until it's available.
    ///
    /// Assumes the transaction has started (CSTART handled externally)
    /// Assumes at least one word has already been written to the Tx FIFO
    pub fn read(&mut self) -> Result<u8, Error> {
        let periph = unsafe { &(*self.regs_ptr)};
        check_errors!(periph.sr.read());

        let mut i = 0;
        while !periph.sr.read().rxp().is_not_empty() {
            i += 1;
            if i >= MAX_ITERS {
                return Err(Error::Hardware);
            }
        }

        // NOTE(read_volatile) read only 1 word
        return Ok(unsafe { ptr::read_volatile(&periph.rxdr as *const _ as *const u8) });
    }

    /// Write multiple bytes on the SPI line, blocking until complete.
    pub fn write(&mut self, write_words: &[u8]) -> Result<(), Error> {
        // both buffers are the same length
        if write_words.is_empty() {
            return Ok(());
        }

        // Fill the first half of the write FIFO
        let len = write_words.len();
        let mut write = write_words.iter();
        for _ in 0..core::cmp::min(FIFO_LEN, len as u8) {
            self.send(*write.next().unwrap());
        }

        // Continue filling write FIFO and emptying read FIFO
        for word in write {
            let _ = self.exchange(*word);
        }

        // Dummy read from the read FIFO
        for _ in 0..core::cmp::min(FIFO_LEN, len as u8) {
            let _ = self.read();
        }

        Ok(())
    }

    /// Read multiple bytes to a buffer, blocking until complete.
    pub fn transfer(&mut self, words: &mut [u8]) -> Result<(), Error> {
        if words.is_empty() {
            return Ok(());
        }

        // Fill the first half of the write FIFO
        let len = words.len() as u8;
        for i in 0..core::cmp::min(FIFO_LEN, len) {
            self.send(words[i as usize]);
        }

        for i in FIFO_LEN..len + FIFO_LEN {
            if i < len {
                // Continue filling write FIFO and emptying read FIFO
                let read_value = self.exchange(words[i as usize])?;

                words[(i - FIFO_LEN) as usize] = read_value;
            } else {
                // Finish emptying the read FIFO
                words[(i - FIFO_LEN) as usize] = self.read()?;
            }
        }

        Ok(())
    }

    fn send(&mut self, word: u8) -> Result<(), Error> {
        let periph = unsafe { &(*self.regs_ptr)};
        check_errors!(periph.sr.read());

        // NOTE(write_volatile) see note above
        unsafe {
            let txdr = &periph.txdr as *const _ as *const UnsafeCell<u8>;
            ptr::write_volatile(UnsafeCell::raw_get(txdr), word)
        }
        // write CSTART to start a transaction in
        // master mode
        periph.cr1.modify(|_, w| w.cstart().started());

        return Ok(());
    }

    /// Receive data using DMA. See H743 RM, section 50.4.14: Communication using DMA
    pub unsafe fn read_dma<const DMA_NUM: u8>(
        &mut self,
        buf: &mut [u8],
        channel: dma::DmaChannel,
        channel_cfg: dma::ChannelCfg,
        dma_periph: &mut dma::Dma<DMA_NUM>,
    ) {
        let periph = unsafe { &(*self.regs_ptr)};
        // todo: Accept u16 and u32 words too.
        let (ptr, len) = (buf.as_mut_ptr(), buf.len());

        periph.cr1.modify(|_, w| w.spe().clear_bit());
        periph.cfg1.modify(|_, w| w.rxdmaen().set_bit());

        let periph_addr = &periph.rxdr as *const _ as u32;
        let num_data = len as u32;

        dma_periph.cfg_channel(
            channel,
            periph_addr,
            ptr as u32,
            num_data,
            dma::Direction::ReadFromPeriph,
            dma::DataSize::S8,
            dma::DataSize::S8,
            channel_cfg,
        );

        periph.cr1.modify(|_, w| w.spe().set_bit());
        periph.cr1.modify(|_, w| w.cstart().set_bit()); // Must be separate from SPE enable.
    }

    pub unsafe fn write_dma<const DMA_NUM: u8>(
        &mut self,
        buf: &[u8],
        channel: dma::DmaChannel,
        channel_cfg: dma::ChannelCfg,
        dma_periph: &mut dma::Dma<DMA_NUM>,
    ) {
        let periph = unsafe { &(*self.regs_ptr)};
        // Static write and read buffers?
        let (ptr, len) = (buf.as_ptr(), buf.len());

        periph.cr1.modify(|_, w| w.spe().clear_bit());

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
        let periph_addr = &periph.txdr as *const _ as u32;
        let num_data = len as u32;

        dma_periph.cfg_channel(
            channel,
            periph_addr,
            ptr as u32,
            num_data,
            dma::Direction::ReadFromMem,
            dma::DataSize::S8,
            dma::DataSize::S8,
            channel_cfg,
        );

        // 3. Enable DMA Tx buffer in the TXDMAEN bit in the SPI_CR2 register, if DMA Tx is used.
        periph.cfg1.modify(|_, w| w.txdmaen().set_bit());

        // 4. Enable the SPI by setting the SPE bit.
        periph.cr1.modify(|_, w| w.spe().set_bit());
        periph.cr1.modify(|_, w| w.cstart().set_bit()); // Must be separate from SPE enable.
    }

    /// Transfer data from DMA; this is the basic reading API, using both write and read transfers:
    /// It performs a write with register data, and reads to a buffer.
    pub unsafe fn transfer_dma<const DMA_NUM: u8>(
        &mut self,
        buf_write: &[u8],
        buf_read: &mut [u8],
        channel_write: dma::DmaChannel,
        channel_read: dma::DmaChannel,
        channel_cfg_write: dma::ChannelCfg,
        channel_cfg_read: dma::ChannelCfg,
        dma_periph: &mut dma::Dma<DMA_NUM>,
    ) {
        let periph = unsafe { &(*self.regs_ptr)};
        // todo: Accept u16 and u32 words too.
        let (ptr_write, len_write) = (buf_write.as_ptr(), buf_write.len());
        let (ptr_read, len_read) = (buf_read.as_mut_ptr(), buf_read.len());

        periph.cr1.modify(|_, w| w.spe().clear_bit());

        // todo: DRY here, with `write_dma`, and `read_dma`.

        let periph_addr_write = &periph.txdr as *const _ as u32;
        let periph_addr_read = &periph.rxdr as *const _ as u32;

        let num_data_write = len_write as u32;
        let num_data_read = len_read as u32;

        // Be careful - order of enabling Rx and Tx may matter, along with other things like when we
        // enable the channels, and the SPI periph.
        periph.cfg1.modify(|_, w| w.rxdmaen().set_bit());

        dma_periph.cfg_channel(
            channel_write,
            periph_addr_write,
            ptr_write as u32,
            num_data_write,
            dma::Direction::ReadFromMem,
            dma::DataSize::S8,
            dma::DataSize::S8,
            channel_cfg_write,
        );

        dma_periph.cfg_channel(
            channel_read,
            periph_addr_read,
            ptr_read as u32,
            num_data_read,
            dma::Direction::ReadFromPeriph,
            dma::DataSize::S8,
            dma::DataSize::S8,
            channel_cfg_read,
        );

        periph.cfg1.modify(|_, w| w.txdmaen().set_bit());

        periph.cr1.modify(|_, w| w.spe().set_bit());
        periph.cr1.modify(|_, w| w.cstart().set_bit()); // Must be separate from SPE enable.
    }

    /// Enable an interrupt.
    pub fn enable_interrupt(&mut self, interrupt_type: Interrupt) {
        let periph = unsafe { &(*self.regs_ptr)};
        periph.ier.modify(|_, w| match interrupt_type {
            Interrupt::NumberOfTransactionsReload => w.tserfie().set_bit(),
            Interrupt::ModeFault => w.modfie().set_bit(),
            Interrupt::Tifre => w.tifreie().set_bit(),
            Interrupt::CrcError => w.crceie().set_bit(),
            Interrupt::Overrun => w.ovrie().set_bit(),
            Interrupt::Underrun => w.udrie().set_bit(),
            Interrupt::Txtfie => w.txtfie().set_bit(),
            Interrupt::EotSuspTxc => w.eotie().set_bit(),
            // SpiInterrupt::Dxp => w.dxpie().set_bit(),
            // SpiInterrupt::Txp => w.txpie().set_bit(),
            // SpiInterrupt::Rxp => w.rxpie().set_bit(),
            _ => w.eotie().set_bit(), // todo: PAC ommission?
        });
    }

    /// Clear an interrupt.
    pub fn clear_interrupt(&mut self, interrupt_type: Interrupt) {
        let periph = unsafe { &(*self.regs_ptr)};
        periph.ifcr.write(|w| match interrupt_type {
            Interrupt::NumberOfTransactionsReload => w.tserfc().set_bit(),
            Interrupt::ModeFault => w.modfc().set_bit(),
            Interrupt::Tifre => w.tifrec().set_bit(),
            Interrupt::CrcError => w.crcec().set_bit(),
            Interrupt::Overrun => w.ovrc().set_bit(),
            Interrupt::Underrun => w.udrc().set_bit(),
            Interrupt::Txtfie => w.txtfc().set_bit(),
            Interrupt::EotSuspTxc => w.eotc().set_bit(),
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
    pub fn stop_dma<const DMA_NUM: u8>(
        &mut self,
        channel: dma::DmaChannel,
        channel2: Option<dma::DmaChannel>,
        dma_periph: &mut dma::Dma<DMA_NUM>,
    ) {
        let periph = unsafe { &(*self.regs_ptr)};
        // (RM:) To close communication it is mandatory to follow these steps in order:
        // 1. Disable DMA streams for Tx and Rx in the DMA registers, if the streams are used.

        dma_periph.stop(channel);
        if let Some(ch2) = channel2 {
            dma_periph.stop(ch2);
        };

        // 2. Disable the SPI by following the SPI disable procedure:
        // self.disable();

        // 3. Disable DMA Tx and Rx buffers by clearing the TXDMAEN and RXDMAEN bits in the
        // SPI_CR2 register, if DMA Tx and/or DMA Rx are used.
        
        periph.cfg1.modify(|_, w| {
            w.txdmaen().clear_bit();
            w.rxdmaen().clear_bit()
        });
    }

    /// Convenience function that clears the interrupt, and stops the transfer. For use with the TC
    /// interrupt only.
    pub fn cleanup_dma<const DMA_NUM: u8>(
        &mut self,
        channel: dma::DmaChannel,
        channel2: Option<dma::DmaChannel>,
        dma_periph: &mut dma::Dma<DMA_NUM>,
    ) {
        // The hardware seems to automatically enable Tx too; and we use it when transmitting.
        dma_periph.clear_interrupt(channel, dma::DmaInterrupt::TransferComplete);

        if let Some(ch_rx) = channel2 {
            dma_periph.clear_interrupt(ch_rx, dma::DmaInterrupt::TransferComplete);
        }

        self.stop_dma(channel, channel2, dma_periph);
    }
}

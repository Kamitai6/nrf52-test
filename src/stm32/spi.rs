use stm32h5::stm32h562;

macro_rules! check_errors {
    ($sr:expr) => {
        let crc_error = $sr.crce().bit_is_set();

        if $sr.ovr().bit_is_set() {
            return 2;
        } else if $sr.modf().bit_is_set() {
            return 4;
        } else if crc_error {
            return 8;
        }
    };
}

pub struct SPI<'a> {
    dp: &'a stm32h562::Peripherals,
}

impl<'a> SPI<'a> {
    pub fn new(dp: &stm32h562::Peripherals) -> SPI {
        SPI { dp }
    }

    pub fn spi3_init(&self) {
        /* use pa15 */
        self.dp.RCC.ahb2enr().modify(|_, w| w.gpioaen().enabled());
        self.dp.RCC.ahb2enr().modify(|_, w| w.gpiocen().enabled());
        self.dp.RCC.apb1lenr().modify(|_, w| w.spi3en().enabled());

        self.dp.GPIOA.moder().modify(|_, w| w.mode15().output());
        self.dp.GPIOC.moder().modify(|_, w| w.mode10().alternate());
        self.dp.GPIOC.moder().modify(|_, w| {
            w.mode10().alternate();
            w.mode11().alternate();
            w.mode12().alternate()
        });
        self.dp.GPIOC.afrh().modify(|_, w| {
            w.afsel10().af6();
            w.afsel11().af6();
            w.afsel12().af6()
        });

        self.spi3_end();

        self.dp.SPI3.cr1().modify(|_, w| w.spe().disabled()); // spi disable
        self.dp.SPI3.cr1().modify(|_, w| w.ssi().set_bit());

        self.dp.SPI3.cfg1().modify(|_, w| {
            w.mbr().div32(); // 240MHz/32=7.5MHz
            w.dsize().bits(16 - 1);
            w.crcen().disabled()
        });

        self.dp.SPI3.cfg2().modify(|_, w| {
            w.cpol().clear_bit();
            w.cpha().set_bit();
            w.master().master();
            w.comm().full_duplex();
            w.ssm().enabled();
            w.lsbfrst().msbfirst()
        });

        self.dp.SPI3.cr2().modify(|_, w| w.tsize().bits(0));

        self.dp.SPI3.cr1().modify(|_, w| w.spe().enabled()); // spi enable
    }

    pub fn spi3_begin(&self) {
        self.dp.GPIOA.odr().modify(|_, w| w.od15().low());
    }

    pub fn spi3_end(&self) {
        self.dp.GPIOA.odr().modify(|_, w| w.od15().high());
    }

    pub fn spi3_send(&self, data: u32) -> u32 {
        check_errors!(self.dp.SPI3.sr().read());

        self.dp.SPI3.cr1().modify(|_, w| w.cstart().started());

        while self.dp.SPI3.sr().read().txp().is_full() {
            cortex_m::asm::nop();
        }
        self.dp.SPI3.txdr().write(|w| w.txdr().bits(data.into()));
        while self.dp.SPI3.sr().read().rxp().is_empty() {
            cortex_m::asm::nop();
        }
        let res = self.dp.SPI3.rxdr().read().bits();

        self.dp.SPI3.ifcr().write(|w| w.eotc().set_bit());
        self.dp.SPI3.ifcr().write(|w| w.txtfc().set_bit());
        self.dp.SPI3.ier().reset();

        res
    }
}

// /// Receive data using DMA. See H743 RM, section 50.4.14: Communication using DMA
// pub unsafe fn read_dma(
//     &mut self,
//     buf: &mut [u8],
//     channel: DmaChannel,
//     channel_cfg: ChannelCfg,
//     dma_periph: dma::DmaPeriph,
// ) {
//     // todo: Accept u16 and u32 words too.
//     let (ptr, len) = (buf.as_mut_ptr(), buf.len());

//     self.regs.cr1.modify(|_, w| w.spe().clear_bit());
//     self.regs.cfg1.modify(|_, w| w.rxdmaen().set_bit());

//     let periph_addr = &self.regs.rxdr as *const _ as u32;
//     let num_data = len as u32;

//     match dma_periph {
//         dma::DmaPeriph::Dma1 => {
//             let mut regs = unsafe { &(*DMA1::ptr()) };
//             dma::cfg_channel(
//                 &mut regs,
//                 channel,
//                 periph_addr,
//                 ptr as u32,
//                 num_data,
//                 dma::Direction::ReadFromPeriph,
//                 dma::DataSize::S8,
//                 dma::DataSize::S8,
//                 channel_cfg,
//             );
//         }

//         dma::DmaPeriph::Dma2 => {
//             let mut regs = unsafe { &(*pac::DMA2::ptr()) };
//             dma::cfg_channel(
//                 &mut regs,
//                 channel,
//                 periph_addr,
//                 ptr as u32,
//                 num_data,
//                 dma::Direction::ReadFromPeriph,
//                 dma::DataSize::S8,
//                 dma::DataSize::S8,
//                 channel_cfg,
//             );
//         }
//     }

//     self.regs.cr1.modify(|_, w| w.spe().set_bit());
//     self.regs.cr1.modify(|_, w| w.cstart().set_bit()); // Must be separate from SPE enable.
// }

// pub unsafe fn write_dma(
//     &mut self,
//     buf: &[u8],
//     channel: DmaChannel,
//     channel_cfg: ChannelCfg,
//     dma_periph: dma::DmaPeriph,
// ) {
//     // Static write and read buffers?
//     let (ptr, len) = (buf.as_ptr(), buf.len());

//     self.regs.cr1.modify(|_, w| w.spe().clear_bit());

//     // todo: Accept u16 words too.

//     // A DMA access is requested when the TXE or RXNE enable bit in the SPIx_CR2 register is
//     // set. Separate requests must be issued to the Tx and Rx buffers.
//     // In transmission, a DMA request is issued each time TXE is set to 1. The DMA then
//     // writes to the SPIx_DR register.

//     // When starting communication using DMA, to prevent DMA channel management raising
//     // error events, these steps must be followed in order:
//     //
//     // 1. Enable DMA Rx buffer in the RXDMAEN bit in the SPI_CR2 register, if DMA Rx is
//     // used.
//     // (N/A)

//     // 2. Enable DMA streams for Tx and Rx in DMA registers, if the streams are used.
//     let periph_addr = &self.regs.txdr as *const _ as u32;
//     let num_data = len as u32;

//     match dma_periph {
//         dma::DmaPeriph::Dma1 => {
//             let mut regs = unsafe { &(*DMA1::ptr()) };
//             dma::cfg_channel(
//                 &mut regs,
//                 channel,
//                 periph_addr,
//                 ptr as u32,
//                 num_data,
//                 dma::Direction::ReadFromMem,
//                 dma::DataSize::S8,
//                 dma::DataSize::S8,
//                 channel_cfg,
//             );
//         }
//         dma::DmaPeriph::Dma2 => {
//             let mut regs = unsafe { &(*pac::DMA2::ptr()) };
//             dma::cfg_channel(
//                 &mut regs,
//                 channel,
//                 periph_addr,
//                 ptr as u32,
//                 num_data,
//                 dma::Direction::ReadFromMem,
//                 dma::DataSize::S8,
//                 dma::DataSize::S8,
//                 channel_cfg,
//             );
//         }
//     }

//     // 3. Enable DMA Tx buffer in the TXDMAEN bit in the SPI_CR2 register, if DMA Tx is used.
//     self.regs.cfg1.modify(|_, w| w.txdmaen().set_bit());

//     // 4. Enable the SPI by setting the SPE bit.
//     self.regs.cr1.modify(|_, w| w.spe().set_bit());
//     self.regs.cr1.modify(|_, w| w.cstart().set_bit()); // Must be separate from SPE enable.
// }

// /// Transfer data from DMA; this is the basic reading API, using both write and read transfers:
// /// It performs a write with register data, and reads to a buffer.
// pub unsafe fn transfer_dma(
//     &mut self,
//     buf_write: &[u8],
//     buf_read: &mut [u8],
//     channel_write: DmaChannel,
//     channel_read: DmaChannel,
//     channel_cfg_write: ChannelCfg,
//     channel_cfg_read: ChannelCfg,
//     dma_periph: dma::DmaPeriph,
// ) {
//     // todo: Accept u16 and u32 words too.
//     let (ptr_write, len_write) = (buf_write.as_ptr(), buf_write.len());
//     let (ptr_read, len_read) = (buf_read.as_mut_ptr(), buf_read.len());

//     self.regs.cr1.modify(|_, w| w.spe().clear_bit());

//     // todo: DRY here, with `write_dma`, and `read_dma`.

//     let periph_addr_write = &self.regs.txdr as *const _ as u32;
//     let periph_addr_read = &self.regs.rxdr as *const _ as u32;

//     let num_data_write = len_write as u32;
//     let num_data_read = len_read as u32;

//     // Be careful - order of enabling Rx and Tx may matter, along with other things like when we
//     // enable the channels, and the SPI periph.
//     self.regs.cfg1.modify(|_, w| w.rxdmaen().set_bit());

//     match dma_periph {
//         dma::DmaPeriph::Dma1 => {
//             let mut regs = unsafe { &(*DMA1::ptr()) };
//             dma::cfg_channel(
//                 &mut regs,
//                 channel_write,
//                 periph_addr_write,
//                 ptr_write as u32,
//                 num_data_write,
//                 dma::Direction::ReadFromMem,
//                 dma::DataSize::S8,
//                 dma::DataSize::S8,
//                 channel_cfg_write,
//             );

//             dma::cfg_channel(
//                 &mut regs,
//                 channel_read,
//                 periph_addr_read,
//                 ptr_read as u32,
//                 num_data_read,
//                 dma::Direction::ReadFromPeriph,
//                 dma::DataSize::S8,
//                 dma::DataSize::S8,
//                 channel_cfg_read,
//             );
//         }

//         dma::DmaPeriph::Dma2 => {
//             let mut regs = unsafe { &(*pac::DMA2::ptr()) };
//             dma::cfg_channel(
//                 &mut regs,
//                 channel_write,
//                 periph_addr_write,
//                 ptr_write as u32,
//                 num_data_write,
//                 dma::Direction::ReadFromMem,
//                 dma::DataSize::S8,
//                 dma::DataSize::S8,
//                 channel_cfg_write,
//             );

//             dma::cfg_channel(
//                 &mut regs,
//                 channel_read,
//                 periph_addr_read,
//                 ptr_read as u32,
//                 num_data_read,
//                 dma::Direction::ReadFromPeriph,
//                 dma::DataSize::S8,
//                 dma::DataSize::S8,
//                 channel_cfg_read,
//             );
//         }
//     }

//     self.regs.cfg1.modify(|_, w| w.txdmaen().set_bit());

//     self.regs.cr1.modify(|_, w| w.spe().set_bit());
//     self.regs.cr1.modify(|_, w| w.cstart().set_bit()); // Must be separate from SPE enable.
// }

// /// Enable an interrupt.
// pub fn enable_interrupt(&mut self, interrupt_type: SpiInterrupt) {
//     self.regs.ier.modify(|_, w| match interrupt_type {
//         SpiInterrupt::NumberOfTransactionsReload => w.tserfie().set_bit(),
//         SpiInterrupt::ModeFault => w.modfie().set_bit(),
//         SpiInterrupt::Tifre => w.tifreie().set_bit(),
//         SpiInterrupt::CrcError => w.crceie().set_bit(),
//         SpiInterrupt::Overrun => w.ovrie().set_bit(),
//         SpiInterrupt::Underrun => w.udrie().set_bit(),
//         SpiInterrupt::Txtfie => w.txtfie().set_bit(),
//         SpiInterrupt::EotSuspTxc => w.eotie().set_bit(),
//         // SpiInterrupt::Dxp => w.dxpie().set_bit(),
//         // SpiInterrupt::Txp => w.txpie().set_bit(),
//         // SpiInterrupt::Rxp => w.rxpie().set_bit(),
//         _ => w.eotie().set_bit(), // todo: PAC ommission?
//     });
// }

// /// Clear an interrupt.
// pub fn clear_interrupt(&mut self, interrupt_type: SpiInterrupt) {
//     self.regs.ifcr.write(|w| match interrupt_type {
//         SpiInterrupt::NumberOfTransactionsReload => w.tserfc().set_bit(),
//         SpiInterrupt::ModeFault => w.modfc().set_bit(),
//         SpiInterrupt::Tifre => w.tifrec().set_bit(),
//         SpiInterrupt::CrcError => w.crcec().set_bit(),
//         SpiInterrupt::Overrun => w.ovrc().set_bit(),
//         SpiInterrupt::Underrun => w.udrc().set_bit(),
//         SpiInterrupt::Txtfie => w.txtfc().set_bit(),
//         SpiInterrupt::EotSuspTxc => w.eotc().set_bit(),
//         // SpiInterrupt::Dxp => w.dxpc().set_bit(),
//         // SpiInterrupt::Txp => w.txpc().set_bit(),
//         // SpiInterrupt::Rxp => w.rxpc().set_bit(),
//         _ => w.eotc().set_bit(), // todo: PAC ommission?
//     });
// }

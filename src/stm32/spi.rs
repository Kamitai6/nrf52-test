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

    こんな感じに展開されるらしい
    dma_trait!(RxDma, Instance);
    ===
    /// Transmit DMA request trait
pub trait RxDma<T: Instance>: crate::dma::Channel {
    /// Get the DMA request number needed to use this channel as Transmit
    fn request(&self) -> crate::dma::Request;
}

    こんな感じに展開されるらしい
    let dma_channel = new_dma!(DMA1_CH4);
    ===
    let dma = DMA1_CH4.into_ref();
let request = dma.request();
Some(crate::dma::ChannelAndRequest {
    channel: dma.map_into(),
    request,
})

こんな感じに展開されるらしい
impl crate::usart::TxDma<crate::peripherals::USART1> for crate::peripherals::DMA1_CH4 {
    fn request(&self) -> crate::dma::Request {
        3
    }
}

    pub fn spi3_init(&self) {}

    pub fn spi3_begin(&self) {
        self.dp.GPIOA.odr().modify(|_, w| w.od15().low());
    }

    pub fn spi3_end(&self) {
        self.dp.GPIOA.odr().modify(|_, w| w.od15().high());
    }

    pub fn spi3_send(&self, data: u16) -> u16 {
        // check_errors!(self.dp.SPI3.sr().read());

        self.dp.SPI3.cr1().modify(|_, w| w.cstart().started());

        while self.dp.SPI3.sr().read().txp().is_full() {
            cortex_m::asm::nop();
        }
        self.dp.SPI3.txdr().write(|w| w.txdr().bits(data as u32));
        while self.dp.SPI3.sr().read().txc().is_ongoing() {
            cortex_m::asm::nop();
        }
        while self.dp.SPI3.sr().read().rxp().is_empty() {
            cortex_m::asm::nop();
        }
        let res = self.dp.SPI3.rxdr().read().bits();
        let resres = (res >> 16) as u16;

        self.dp.SPI3.ifcr().write(|w| w.eotc().set_bit());
        self.dp.SPI3.ifcr().write(|w| w.txtfc().set_bit());
        self.dp.SPI3.ier().reset();

        resres
    }

    async fn transfer_inner<W: Word>(
        &mut self,
        read: *mut [W],
        write: *const [W],
    ) -> Result<(), Error> {
        assert_eq!(read.len(), write.len());
        if read.len() == 0 {
            return Ok(());
        }

        self.set_word_size(W::CONFIG);
        self.info.regs.cr1().modify(|w| {
            w.set_spe(false);
        });

        // SPIv3 clears rxfifo on SPE=0
        #[cfg(not(any(spi_v3, spi_v4, spi_v5)))]
        flush_rx_fifo(self.info.regs);

        set_rxdmaen(self.info.regs, true);

        let rx_src = self.info.regs.rx_ptr();
        let rx_f = unsafe {
            self.rx_dma
                .as_mut()
                .unwrap()
                .read_raw(rx_src, read, Default::default())
        };

        let tx_dst = self.info.regs.tx_ptr();
        let tx_f = unsafe {
            self.tx_dma
                .as_mut()
                .unwrap()
                .write_raw(write, tx_dst, Default::default())
        };

        set_txdmaen(self.info.regs, true);
        self.info.regs.cr1().modify(|w| {
            w.set_spe(true);
        });
        #[cfg(any(spi_v3, spi_v4, spi_v5))]
        self.info.regs.cr1().modify(|w| {
            w.set_cstart(true);
        });

        join(tx_f, rx_f).await;

        finish_dma(self.info.regs);

        Ok(())
    }

    fn flush_rx_fifo(regs: Regs) {
        #[cfg(not(any(spi_v3, spi_v4, spi_v5)))]
        while regs.sr().read().rxne() {
            #[cfg(not(spi_v2))]
            let _ = regs.dr().read();
            #[cfg(spi_v2)]
            let _ = regs.dr16().read();
        }
        #[cfg(any(spi_v3, spi_v4, spi_v5))]
        while regs.sr().read().rxp() {
            let _ = regs.rxdr32().read();
        }
    }

    fn set_txdmaen(regs: Regs, val: bool) {
        #[cfg(not(any(spi_v3, spi_v4, spi_v5)))]
        regs.cr2().modify(|reg| {
            reg.set_txdmaen(val);
        });
        #[cfg(any(spi_v3, spi_v4, spi_v5))]
        regs.cfg1().modify(|reg| {
            reg.set_txdmaen(val);
        });
    }

    fn set_rxdmaen(regs: Regs, val: bool) {
        #[cfg(not(any(spi_v3, spi_v4, spi_v5)))]
        regs.cr2().modify(|reg| {
            reg.set_rxdmaen(val);
        });
        #[cfg(any(spi_v3, spi_v4, spi_v5))]
        regs.cfg1().modify(|reg| {
            reg.set_rxdmaen(val);
        });
    }

    fn finish_dma(regs: Regs) {
        #[cfg(spi_v2)]
        while regs.sr().read().ftlvl().to_bits() > 0 {}

        #[cfg(any(spi_v3, spi_v4, spi_v5))]
        {
            if regs.cr2().read().tsize() == 0 {
                while !regs.sr().read().txc() {}
            } else {
                while !regs.sr().read().eot() {}
            }
        }
        #[cfg(not(any(spi_v3, spi_v4, spi_v5)))]
        while regs.sr().read().bsy() {}

        // Disable the spi peripheral
        regs.cr1().modify(|w| {
            w.set_spe(false);
        });

        // The peripheral automatically disables the DMA stream on completion without error,
        // but it does not clear the RXDMAEN/TXDMAEN flag in CR2.
        #[cfg(not(any(spi_v3, spi_v4, spi_v5)))]
        regs.cr2().modify(|reg| {
            reg.set_txdmaen(false);
            reg.set_rxdmaen(false);
        });
        #[cfg(any(spi_v3, spi_v4, spi_v5))]
        regs.cfg1().modify(|reg| {
            reg.set_txdmaen(false);
            reg.set_rxdmaen(false);
        });
    }
}

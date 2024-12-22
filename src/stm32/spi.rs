use stm32h5::stm32h562;

pub struct Spi<'a, REG> {
    reg: &'a REG,
}
//type Result = Result<(), u8>;などとすることで、Resultの中身を変えることができる
//mod stm32 { //modはnamespaceのようなもの
    impl<'a, REG> Spi<'a, REG> {
        pub fn new(reg: &REG) -> Spi<REG> {
            Spi { reg }
        }

        pub fn spi3_init(&self) {
            /* use pa15 */
            self.reg.RCC.ahb2enr().modify(|_, w| w.gpioaen().enabled());
            self.dp.RCC.ahb2enr().modify(|_, w| w.gpiocen().enabled());
            self.dp.RCC.apb1lenr().modify(|_, w| w.spi3en().enabled());

            self.dp.GPIOA.moder().modify(|_, w| w.mode15().output());
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
                w.mbr().div64(); // 240MHz/32=7.5MHz
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

        // cfgはspi_v4らしい
        pub fn transfer_inner<W>(&mut self, read: *mut [W], write: *const [W]) -> Result {
            assert_eq!(read.len(), write.len());
            if read.len() == 0 {
                return Ok(());
            }

            self.set_word_size(W::CONFIG);
            self.info.regs.cr1().modify(|w| {
                w.set_spe(false);
            });

            // // SPIv3 clears rxfifo on SPE=0
            // #[cfg(not(any(spi_v3, spi_v4, spi_v5)))]
            // flush_rx_fifo(self.info.regs);

            self.set_rxdmaen(self.info.regs, true);

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

            self.set_txdmaen(self.info.regs, true);
            self.info.regs.cr1().modify(|w| {
                w.set_spe(true);
            });
            // #[cfg(any(spi_v3, spi_v4, spi_v5))]
            self.info.regs.cr1().modify(|w| {
                w.set_cstart(true);
            });

            Ok(())
        }

        // fn flush_rx_fifo(regs: Regs) {
        //     #[cfg(not(any(spi_v3, spi_v4, spi_v5)))]
        //     while regs.sr().read().rxne() {
        //         #[cfg(not(spi_v2))]
        //         let _ = regs.dr().read();
        //         #[cfg(spi_v2)]
        //         let _ = regs.dr16().read();
        //     }
        //     #[cfg(any(spi_v3, spi_v4, spi_v5))]
        //     while regs.sr().read().rxp() {
        //         let _ = regs.rxdr32().read();
        //     }
        // }

        fn set_txdmaen(regs: Regs, val: bool) {
            // #[cfg(not(any(spi_v3, spi_v4, spi_v5)))]
            // regs.cr2().modify(|reg| {
            //     reg.set_txdmaen(val);
            // });
            // #[cfg(any(spi_v3, spi_v4, spi_v5))]
            regs.cfg1().modify(|reg| {
                reg.set_txdmaen(val);
            });
        }

        fn set_rxdmaen(regs: Regs, val: bool) {
            // #[cfg(not(any(spi_v3, spi_v4, spi_v5)))]
            // regs.cr2().modify(|reg| {
            //     reg.set_rxdmaen(val);
            // });
            // #[cfg(any(spi_v3, spi_v4, spi_v5))]
            regs.cfg1().modify(|reg| {
                reg.set_rxdmaen(val);
            });
        }

        fn finish_dma(regs: Regs) {
            #[cfg(spi_v2)]
            while regs.sr().read().ftlvl().to_bits() > 0 {}

            // #[cfg(any(spi_v3, spi_v4, spi_v5))]
            {
                if regs.cr2().read().tsize() == 0 {
                    while !regs.sr().read().txc() {}
                } else {
                    while !regs.sr().read().eot() {}
                }
            }
            // #[cfg(not(any(spi_v3, spi_v4, spi_v5)))]
            // while regs.sr().read().bsy() {}

            // Disable the spi peripheral
            regs.cr1().modify(|w| {
                w.set_spe(false);
            });

            // The peripheral automatically disables the DMA stream on completion without error,
            // but it does not clear the RXDMAEN/TXDMAEN flag in CR2.
            // #[cfg(not(any(spi_v3, spi_v4, spi_v5)))]
            // regs.cr2().modify(|reg| {
            //     reg.set_txdmaen(false);
            //     reg.set_rxdmaen(false);
            // });
            // #[cfg(any(spi_v3, spi_v4, spi_v5))]
            regs.cfg1().modify(|reg| {
                reg.set_txdmaen(false);
                reg.set_rxdmaen(false);
            });
        }
    }
}

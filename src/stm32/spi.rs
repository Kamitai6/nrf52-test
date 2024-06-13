use stm32h5::stm32h562;

pub struct SPI<'a> {
    dp: &'a stm32h562::Peripherals,
}

impl<'a> SPI<'a> {
    pub fn new(dp: &stm32h562::Peripherals) -> SPI {
        SPI { dp }
    }

    pub fn spi3_init(&self) {
        // use pa15
        self.dp.RCC.ahb2enr().modify(|_, w| w.gpioaen().enabled());
        self.dp.RCC.ahb2enr().modify(|_, w| w.gpiocen().enabled());
        self.dp.GPIOA.moder().modify(|_, w| w.mode15().output());
        // self.dp.GPIOA.afrh().modify(|_, w| w.afsel15().af6());
        self.dp.GPIOC.moder().modify(|_, w| w.mode10().alternate());
        self.dp.GPIOC.afrh().modify(|_, w| w.afsel10().af6());
        self.dp.GPIOC.moder().modify(|_, w| w.mode11().alternate());
        self.dp.GPIOC.afrh().modify(|_, w| w.afsel11().af6());
        self.dp.GPIOC.moder().modify(|_, w| w.mode12().alternate());
        self.dp.GPIOC.afrh().modify(|_, w| w.afsel12().af6());

        self.spi3_end();

        self.dp.RCC.apb1lenr().modify(|_, w| w.spi3en().enabled());

        self.dp.SPI3.cfg1().modify(|_, w| w.mbr().div32()); // 240MHz/32=7.5MHz
        self.dp.SPI3.cfg1().modify(|_, w| w.dsize().bits(16 - 1));
        self.dp.SPI3.cfg1().modify(|_, w| w.fthlv().four_frames());
        self.dp.SPI3.cfg1().modify(|_, w| w.crcen().disabled());
        self.dp.SPI3.cfg1().modify(|_, w| w.crcsize().bits(8 - 1));

        self.dp.SPI3.cfg2().modify(|_, w| w.comm().full_duplex());
        self.dp.SPI3.cfg2().modify(|_, w| w.lsbfrst().msbfirst());
        self.dp.SPI3.cfg2().modify(|_, w| w.cpha().second_edge());
        self.dp.SPI3.cfg2().modify(|_, w| w.cpol().idle_low());
        self.dp.SPI3.cfg2().modify(|_, w| w.master().master());
        self.dp.SPI3.cfg2().modify(|_, w| w.ssm().enabled());
        self.dp.SPI3.cfg2().modify(|_, w| w.sp().motorola());

        self.dp.SPI3.cr1().modify(|_, w| w.ssi().set_bit()); // must be after setting ssm bit
        self.dp.SPI3.cr2().modify(|_, w| w.tsize().bits(0));
        self.dp.SPI3.cr1().modify(|_, w| w.spe().enabled()); // spi enable
        self.dp.SPI3.cr1().modify(|_, w| w.cstart().set_bit());
    }

    pub fn spi3_begin(&self) {
        self.dp.GPIOA.odr().modify(|_, w| w.od15().low());
    }

    pub fn spi3_end(&self) {
        self.dp.GPIOA.odr().modify(|_, w| w.od15().high());
    }

    pub fn spi3_send(&self, data: u32) -> u32 {
        self.dp.SPI3.cr2().modify(|_, w| w.tsize().bits(1));
        self.dp.SPI3.cfg2().modify(|_, w| w.comm().full_duplex());
        self.dp.SPI3.cr1().modify(|_, w| w.cstart().set_bit());

        while self.dp.SPI3.sr().read().txp().bit_is_clear() {
            cortex_m::asm::nop();
        }
        self.dp.SPI3.txdr().write(|w| w.txdr().bits(data));
        while self.dp.SPI3.sr().read().rxwne().is_at_least32() {
            cortex_m::asm::nop();
        }
        let res = self.dp.SPI3.rxdr().read().bits();

        self.dp.SPI3.ifcr().write(|w| w.eotc().set_bit());
        self.dp.SPI3.ifcr().write(|w| w.txtfc().set_bit());
        self.dp.SPI3.cr2().modify(|_, w| w.tsize().bits(0));
        self.dp.SPI3.cr1().modify(|_, w| w.spe().disabled());
        self.dp.SPI3.ier().reset();

        res
    }
}

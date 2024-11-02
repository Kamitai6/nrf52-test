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

    pub fn spi3_send(&self, data: u16) -> u16 {
        // check_errors!(self.dp.SPI3.sr().read());

        self.dp.SPI3.cr1().modify(|_, w| w.cstart().started());

        while self.dp.SPI3.sr().read().txp().is_full() {
            cortex_m::asm::nop();
        }
        self.dp.SPI3.txdr().write(|w| w.txdr().bits(data as u32));
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
}

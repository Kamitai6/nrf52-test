use stm32h5::stm32h562;

pub fn gpio_a12_init(dp: &stm32h562::Peripherals) {
    dp.RCC.ahb2enr().modify(|_, w| w.gpioaen().enabled());
    dp.GPIOA.moder().modify(|_, w| w.mode12().output());
}

pub fn gpio_c10_init(dp: &stm32h562::Peripherals) {
    dp.RCC.ahb2enr().modify(|_, w| w.gpiocen().enabled());
    dp.GPIOC.moder().modify(|_, w| w.mode10().output());
}

pub fn gpio_c11_init(dp: &stm32h562::Peripherals) {
    dp.RCC.ahb2enr().modify(|_, w| w.gpiocen().enabled());
    dp.GPIOC.moder().modify(|_, w| w.mode11().output());
}

pub fn gpio_c12_init(dp: &stm32h562::Peripherals) {
    dp.RCC.ahb2enr().modify(|_, w| w.gpiocen().enabled());
    dp.GPIOC.moder().modify(|_, w| w.mode12().output());
}

pub fn gpio_a12_toggle(dp: &stm32h562::Peripherals, toggle: bool) {
    if toggle {
        dp.GPIOA.odr().modify(|_, w| w.od12().high());
    } else {
        dp.GPIOA.odr().modify(|_, w| w.od12().low());
    }
}

pub fn gpio_c10_toggle(dp: &stm32h562::Peripherals, toggle: bool) {
    if toggle {
        dp.GPIOC.odr().modify(|_, w| w.od10().high());
    } else {
        dp.GPIOC.odr().modify(|_, w| w.od10().low());
    }
}

pub fn gpio_c11_toggle(dp: &stm32h562::Peripherals, toggle: bool) {
    if toggle {
        dp.GPIOC.odr().modify(|_, w| w.od11().high());
    } else {
        dp.GPIOC.odr().modify(|_, w| w.od11().low());
    }
}

pub fn gpio_c12_toggle(dp: &stm32h562::Peripherals, toggle: bool) {
    if toggle {
        dp.GPIOC.odr().modify(|_, w| w.od12().high());
    } else {
        dp.GPIOC.odr().modify(|_, w| w.od12().low());
    }
}

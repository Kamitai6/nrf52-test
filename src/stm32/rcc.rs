use stm32h5::stm32h562;

pub fn clock_init(dp: &stm32h562::Peripherals) {
    dp.FLASH.acr().modify(|_, w| unsafe { w.latency().bits(5) });
    while dp.FLASH.acr().read().latency().bits() != 5 {}
    dp.PWR.voscr().modify(|_, w| w.vos().vos0());
    while dp.PWR.vossr().read().vosrdy().is_not_ready() {}
    dp.RCC.cr().modify(|_, w| w.hsion().on());
    while dp.RCC.cr().read().hsirdy().is_not_ready() {}
    dp.RCC.hsicfgr().modify(|_, w| w.hsitrim().bits(64));
    dp.RCC.cr().modify(|_, w| w.hsidiv().div1());

    dp.RCC.pll1cfgr().modify(|_, w| w.pll1src().hsi());
    dp.RCC.pll1cfgr().modify(|_, w| w.pll1rge().range8());
    dp.RCC.pll1cfgr().modify(|_, w| w.pll1vcosel().wide_vco());
    dp.RCC
        .pll1cfgr()
        .modify(|_, w| unsafe { w.pll1m().bits(4) });
    dp.RCC
        .pll1divr()
        .modify(|_, w| unsafe { w.pll1n().bits(30 - 1) });
    dp.RCC.pll1divr().modify(|_, w| w.pll1p().bits(2 - 1));
    dp.RCC.pll1divr().modify(|_, w| w.pll1q().bits(2 - 1));
    dp.RCC.pll1divr().modify(|_, w| w.pll1r().bits(2 - 1));
    dp.RCC.pll1cfgr().modify(|_, w| w.pll1qen().enabled());
    dp.RCC.pll1cfgr().modify(|_, w| w.pll1pen().enabled());
    dp.RCC.cr().modify(|_, w| w.pll1on().on());
    while dp.RCC.cr().read().pll1rdy().is_not_ready() {}

    dp.RCC.cfgr1().modify(|_, w| w.sw().pll1());
    while !dp.RCC.cfgr1().read().sws().is_pll1() {}

    dp.RCC.cfgr2().modify(|_, w| w.hpre().div1());
    dp.RCC.cfgr2().modify(|_, w| w.ppre1().div1());
    dp.RCC.cfgr2().modify(|_, w| w.ppre2().div1());
    dp.RCC.cfgr2().modify(|_, w| w.ppre3().div1());
}

use stm32h5::stm32h562;

fn tim2_init(dp: &stm32h562::Peripherals) {
    // dp.RCC.apb1enr.modify(|_, w| w.tim2en().enabled()); // (24)TIM2のクロックを有効にする
    // dp.TIM2.psc.modify(|_, w| unsafe { w.bits(84 - 1) }); // (25)プリスケーラの設定

    // // 周波数はここで決める
    // dp.TIM2.arr.modify(|_, w| unsafe { w.bits(1000 - 1) }); // (26)ロードするカウント値
    // dp.TIM2.ccmr1_output().modify(|_, w| w.oc1m().pwm_mode1()); // (27)出力比較1 PWMモード1

    // // Duty比はここで決まる
    // dp.TIM2.ccr1.modify(|_, w| unsafe { w.bits(500 - 1) }); // (28)キャプチャ比較モードレジスタ1
}

fn tim2_start(dp: &stm32h562::Peripherals) {
    // dp.TIM2.cr1.modify(|_, w| w.cen().enabled()); // (29)カウンタ有効
    // dp.TIM2.ccer.modify(|_, w| w.cc1e().set_bit()); // (30)キャプチャ比較1出力イネーブル
}

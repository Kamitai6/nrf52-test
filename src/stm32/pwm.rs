use stm32h5::stm32h562;

fn tim2_change_duty(dp: &stm32h562::Peripherals, duty: u32) {
    // let config;
    // if duty == 0 {
    //     config = 1;
    // } else if duty > 1000 {
    //     config = 1000;
    // } else {
    //     config = duty;
    // }
    // dp.TIM2.ccr1.modify(|_, w| unsafe { w.bits(config - 1) }); // (31)キャプチャ比較モードレジスタ1
}

// #![no_std]
// #![no_main]
// mod defaults;
// // mod sensor;

// use cortex_m_rt::entry;
// use embassy_executor::Executor;
// use embassy_stm32::gpio::{Level, Output, Speed};
// use embassy_stm32::mode::Async;
// use embassy_stm32::mode::Blocking;
// use embassy_stm32::{spi, Config};
// use embassy_time::{Duration, Timer};
// use heapless::String;
// use panic_halt as _;
// use rtt_target::{rprintln, rtt_init_print};
// use static_cell::StaticCell;

// // #[embassy_executor::task]
// // async fn main_task(mut ss: Output<'static>, mut spi: spi::Spi<'static, Async>) {
// //     let mut read_buffer = [0u16]; // ダミーデータ
// //     let mut write_buffer: [u16; 1] = [0x3FFF]; // 角度読み取りコマンド

// //     loop {
// //         // SPI経由でデータを送受信
// //         ss.set_low();
// //         rprintln!("Sending read command");
// //         // spi.transfer(&mut read_buffer, &write_buffer).await.ok();
// //         spi.write(&write_buffer).await.ok();
// //         spi.read(&mut read_buffer).await.ok();
// //         // unsafe {
// //         //     let result = spi.blocking_transfer_in_place(&mut write_buffer);
// //         //     if let Err(_) = result {
// //         //         rprintln!("crap");
// //         //     }
// //         // }
// //         rprintln!("Received angle: {}", read_buffer[0]);
// //         ss.set_high();
// //         write_buffer[0] = 0x0000;
// //         Timer::after(Duration::from_millis(1)).await;
// //         ss.set_low();
// //         rprintln!("Sending read command");
// //         spi.write(&write_buffer).await.ok();
// //         spi.read(&mut read_buffer).await.ok();
// //         // unsafe {
// //         //     let result = spi.blocking_transfer_in_place(&mut write_buffer);
// //         //     if let Err(_) = result {
// //         //         rprintln!("crap");
// //         //     }
// //         // }
// //         rprintln!("Received angle: {}", read_buffer[0]);
// //         ss.set_high();
// //         rprintln!("Finished SPI communication");
// //     }
// // }

// // static EXECUTOR: StaticCell<Executor> = StaticCell::new();

// #[entry]
// fn main() -> ! {
//     rtt_init_print!();
//     let mut config = embassy_stm32::Config::default();
//     {
//         use embassy_stm32::rcc::*;
//         config.rcc.ls = LsConfig::off();
//         config.rcc.hsi = Some(HSIPrescaler::DIV1);
//         config.rcc.csi = false;
//         config.rcc.pll1 = Some(Pll {
//             source: PllSource::HSI,
//             prediv: PllPreDiv::DIV4, // pllm
//             mul: PllMul::MUL30,      //plln
//             divp: Some(PllDiv::DIV2),
//             divq: Some(PllDiv::DIV2),
//             divr: Some(PllDiv::DIV2),
//         });
//         config.rcc.sys = Sysclk::PLL1_P;
//         config.rcc.ahb_pre = AHBPrescaler::DIV1;
//         config.rcc.apb1_pre = APBPrescaler::DIV1;
//         config.rcc.apb2_pre = APBPrescaler::DIV1;
//         config.rcc.apb3_pre = APBPrescaler::DIV1;
//         config.rcc.voltage_scale = VoltageScale::Scale0;
//     }
//     let p = embassy_stm32::init(config);
//     rprintln!("Hello, world!");

//     let mut spi3_ss = Output::new(p.PA15, Level::High, Speed::Low);
//     let mut spi3_config = embassy_stm32::spi::Config::default();
//     spi3_config.mode = embassy_stm32::spi::MODE_1;
//     spi3_config.frequency = embassy_stm32::time::mhz(3);
//     spi3_config.bit_order = embassy_stm32::spi::BitOrder::MsbFirst;
//     //     // let mut spi3 = embassy_stm32::spi::Spi::new(
//     //     //     p.SPI3,
//     //     //     p.PC10,
//     //     //     p.PC12,
//     //     //     p.PC11,
//     //     //     p.GPDMA2_CH3,
//     //     //     p.GPDMA2_CH4,
//     //     //     spi3_config,
//     //     // );
//     let mut spi3 =
//         embassy_stm32::spi::Spi::new_blocking(p.SPI3, p.PC10, p.PC12, p.PC11, spi3_config);
//     //     // let mut encoder = sensor::encoder::AS5048::new(spi3, spi3_ss);

//     //     // encoder.init();
//     //     // motor.linkSensor(&encoder);
//     //     // driver.voltage_power_supply = 12;
//     //     // driver.pwm_frequency = 15000; // suggested under 18khz
//     //     // driver.init();
//     //     // motor.linkDriver(&driver);
//     //     // current_sense.linkDriver(&driver);
//     //     // motor.torque_controller = TorqueControlType::voltage;
//     //     // motor.controller = MotionControlType::torque;
//     //     // motor.motion_downsample = 0.0;
//     //     // motor.PID_velocity.P = 0.2;
//     //     // motor.PID_velocity.I = 5.0;
//     //     // motor.LPF_velocity.Tf = 0.02;
//     //     // motor.P_angle.P = 20.0;
//     //     // motor.LPF_angle.Tf = 0.0;
//     //     // motor.PID_current_q.P = 3.0;
//     //     // motor.PID_current_q.I = 100.0;
//     //     // motor.LPF_current_q.Tf = 0.02;
//     //     // motor.PID_current_d.P = 3.0;
//     //     // motor.PID_current_d.I = 100.0;
//     //     // motor.LPF_current_d.Tf = 0.02;
//     //     // motor.velocity_limit = 100.0; // 100 rad/s velocity limit
//     //     // motor.voltage_limit = 12.0; // 12 Volt limit
//     //     // motor.current_limit = 2.0; // 2 Amp current limit
//     //     // motor.init();
//     //     // current_sense.init();
//     //     // cs.gain_a *= -1;
//     //     // cs.gain_b *= -1;
//     //     // cs.gain_c *= -1;
//     //     // motor.linkCurrentSense(&current_sense);
//     //     // motor.initFOC();

//     //     // let executor = EXECUTOR.init(Executor::new());

//     //     // executor.run(|spawner| {
//     //     //     spawner.spawn(main_task(spi3_ss, spi3)).unwrap();
//     //     // })
//     let mut read_buffer = [0u16]; // ダミーデータ
//     let mut write_buffer: [u16; 1] = [0x3FFF]; // 角度読み取りコマンド
//     loop {
//         spi3_ss.set_low();
//         rprintln!("Sending read command");
//         spi3.blocking_write(&write_buffer).ok();
//         spi3.blocking_read(&mut read_buffer).ok();
//         spi3_ss.set_high();
//         rprintln!("Received angle: {}", read_buffer[0]);
//     }
// }
#![no_std]
#![no_main]

use cortex_m::{self, delay};
use cortex_m_rt::entry;
use embedded_alloc::Heap;
use panic_halt as _;
use rtt_target::{rprintln, rtt_init_print};
use stm32h5::stm32h562;

mod as5048;
mod stm32;

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[entry]
fn main() -> ! {
    rtt_init_print!();
    use core::mem::MaybeUninit;
    const HEAP_SIZE: usize = 1024;
    static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
    unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }

    let dp = stm32h562::Peripherals::take().unwrap();
    let cp = cortex_m::peripheral::Peripherals::take().unwrap();
    let mut delay = delay::Delay::new(cp.SYST, 240000000_u32);

    stm32::rcc::clock_init(&dp);
    let spi = stm32::spi::SPI::new(&dp);
    let delay_func = move |ms: u32| delay.delay_ms(ms);
    let mut encoder = as5048::AS5048::new(&spi, delay_func);
    spi.spi3_init();

    loop {
        // let state = encoder.getDiagnostic();
        // rprintln!("state: {}", state);

        // let gain = encoder.getGain();
        // rprintln!("gain: {}", gain);

        // let error = encoder.getErrors();
        // rprintln!("error: {}", error);

        let rotation = encoder.getRawRotation();
        rprintln!("rotation: {}", rotation);
    }
}

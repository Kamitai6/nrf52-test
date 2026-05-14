#![no_std]
#![no_main]

use core::{
    cell::{Cell, RefCell, UnsafeCell},
    f32::consts::PI,
    iter::once,
    sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering},
};
use cortex_m::{
    interrupt::{Mutex, free},
    singleton,
};
use cortex_m_rt::entry;
use embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin};

use nrf52840_hal as hal;
use nrf52840_hal::gpio::Level;
use panic_halt as _;

use usbd_serial::SerialPort;
use usbhid::num_enum::FromPrimitive;
use usbhid::*;
use usbhid::{
    device::keyboard::NKROBootKeyboardReport,
    usb_device::{class_prelude::*, prelude::*},
};
use usbhid::{device::mouse::WheelMouseReport, usb_class::UsbHidClassBuilder};
use enum_map::{Enum, EnumMap};
use itoa::Buffer;
use libm::{cosf, hypotf, roundf, sinf};
mod events;


// ledはアクティブロー
// LSM6DS3TR-Cが乗っているらしい
#[entry]
fn main() -> ! {
    let p = hal::pac::Peripherals::take().unwrap();
    let port0 = hal::gpio::p0::Parts::new(p.P0);
    let mut led = port0.p0_17.into_push_pull_output(Level::High);
    loop {
        led.set_low().unwrap();
        // イベントがあるかチェック
        // if events::has_pending_events() {
        //     // 全てのイベントを処理
        //     while let Some(event) = events::get_event() {
        //         match event {
        //             events::Event::LedUpdate => {
                        
        //             }
        //             events::Event::CdcUpdate => {
        //             }
        //         }
        //     }
        // }
    }
}

// // 24kHz
// #[interrupt]
// fn TIMER_IRQ_0() {
//     free(|cs| {
//         let counter = TIMER_COUNTER.borrow(cs).get();
//         let polling_rate = POLLINGRATE.borrow(cs).get();
//         // まずOption::takeで中身を一時的に取り出す
//         let mut opt = TIMERIRQ0_ITEMS.borrow(cs).borrow_mut().take();
//         if let Some((mut alarm0)) = opt {
//             alarm0.clear_interrupt();
//             alarm0.schedule(PERIOD_US.micros()).unwrap();
//             *TIMERIRQ0_ITEMS.borrow(cs).borrow_mut() = Some((alarm0));

//             let div = FREQUENCY / polling_rate;
//             if counter % div == 0 {
//                 events::post_event(events::Event::UsbUpdate);
//             }
//             let div = div * 10;
//             if counter % div == 0 {
//                 events::post_event(events::Event::CdcUpdate);
//             }
//             let div = div * 10;
//             if counter % div == 0 {
//                 events::post_event(events::Event::LedUpdate);
//             }
//             TIMER_COUNTER.borrow(cs).set(counter + 1);
//         }
//     });
// }
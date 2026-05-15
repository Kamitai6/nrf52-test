#![no_std]
#![no_main]

use core::{
    cell::{Cell, RefCell, UnsafeCell},
    f32::consts::PI,
    iter::once,
    sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering},
    fmt::Write,
};
use cortex_m::{
    interrupt::{Mutex, free},
    singleton,
};
use cortex_m_rt::entry;
use embedded_hal::{
    digital::{InputPin, OutputPin, StatefulOutputPin},
    i2c::I2c
};

use nrf52840_hal as hal;
use nrf52840_hal::{
    clocks::Clocks,
    pac::{self, interrupt, NVIC},
    gpio::Level,
    usbd::{Usbd, UsbPeripheral},
    twim::{self, Pins, Twim},
    timer::{Timer, Periodic},
};
use panic_halt as _;

use max3010x::{Led, Max3010x};
use usbd_serial::SerialPort;
use usbhid::num_enum::FromPrimitive;
use usbhid::*;
use usbhid::{
    device::keyboard::NKROBootKeyboardReport,
    usb_device::{class_prelude::*, prelude::*},
};
use usbhid::{device::mouse::WheelMouseReport, usb_class::UsbHidClassBuilder};
use enum_map::{Enum, EnumMap};
use libm::{cosf, hypotf, roundf, sinf};
use nb::block;
use heapless::String;
mod events;

pub const FREQUENCY: u32 = 2000;
pub const PERIOD_US: u32 = 500;
pub type Timer1IrqItems<'a> = (Timer<pac::TIMER1, Periodic>);
pub static TIMER1IRQ_ITEMS: Mutex<RefCell<Option<Timer1IrqItems>>> = Mutex::new(RefCell::new(None));
pub static TIMER_COUNTER: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));
pub static POLLINGRATE: Mutex<Cell<u32>> = Mutex::new(Cell::new(1000));
pub static mut CONTROL_BUFFER: UnsafeCell<[u8; 256]> = UnsafeCell::new([0; 256]);

// ledはアクティブロー
// LSM6DS3TR-Cが乗っているらしい
// MAX30102
// TMP117
#[entry]
fn main() -> ! {
    let pac = hal::pac::Peripherals::take().unwrap();
    // このhalは、Adafruitブートローダーに対応していないので自分で設定する必要がある
    unsafe {
        (*cortex_m::peripheral::SCB::PTR).vtor.write(0x0002_7000);
    }
    let clocks = Clocks::new(pac.CLOCK).enable_ext_hfosc();
    let port0 = hal::gpio::p0::Parts::new(pac.P0);
    let mut led = port0.p0_17.into_push_pull_output(Level::High);
    
    let usb_bus = UsbBusAllocator::new(Usbd::new(UsbPeripheral::new(pac.USBD, &clocks)));
    let mut serial = SerialPort::new(&usb_bus);

    //https://pid.codes
    #[allow(static_mut_refs)]
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd), unsafe {
        CONTROL_BUFFER.get_mut()
    })
    .strings(&[StringDescriptors::default()
        .manufacturer("Kamitai6")
        .product("XIAO nRF52840 Serial")
        .serial_number("1234")])
    .unwrap()
    .composite_with_iads()
    .max_packet_size_0(64)
    .unwrap()
    .device_class(0xEF) // Multi-interface Function
    .device_sub_class(0x02) // Common Class
    .device_protocol(0x01) // IAD protocol
    .build()
    .unwrap();

    let scl = port0.p0_05.into_floating_input().degrade();
    let sda = port0.p0_04.into_floating_input().degrade();
    let pins = Pins { scl, sda };
    let i2c = Twim::new(pac.TWIM0, pins, twim::Frequency::K400);
    let mut max30102 = Max3010x::new_max30102(i2c)
        .into_oximeter().unwrap();
    max30102.set_pulse_amplitude(Led::All, 15).unwrap();
    max30102.enable_fifo_rollover().unwrap();
    // let max30_temp = nb::block!(max30102.read_temperature());

    let scl1 = port0.p0_03.into_floating_input().degrade();
    let sda1 = port0.p0_02.into_floating_input().degrade();
    let pins1 = Pins { scl:scl1, sda:sda1 };
    let mut i2c1 = Twim::new(pac.TWIM1, pins1, twim::Frequency::K100);

    let mut timer = Timer::periodic(pac.TIMER1);
    timer.enable_interrupt();
    timer.start(PERIOD_US);

    free(|cs| {
        TIMER1IRQ_ITEMS.borrow(cs).replace(Some((timer)));
    });

    unsafe {
        NVIC::unmask(pac::Interrupt::TIMER1);
    }

    // 最新のFIFOサンプル（Red, IR）
    // data[0] = Red, data[1] = IR（1サンプル分）
    let mut latest_red: u32 = 0;
    let mut latest_ir: u32 = 0;
    let mut counter: u32 = 0;

    loop {
        // イベントがあるかチェック
        if events::has_pending_events() {
            // 全てのイベントを処理
            while let Some(event) = events::get_event() {
                match event {
                    events::Event::LedUpdate => {
                        led.toggle().unwrap();
                    }
                    events::Event::ReadSensor => {
                        // 1サンプル分（[Red, IR]）を読み出す
                        let mut data = [0u32; 2];
                        match max30102.read_fifo(&mut data) {
                            Ok(count) if count > 0 => {
                                latest_red = data[0];
                                latest_ir  = data[1];
                            }
                            _ => {}
                        }
                    }
                    events::Event::CdcUpdate => {
                        if usb_dev.state() == usb_device::device::UsbDeviceState::Configured {
                            // match max30_temp {
                            //     Ok(temp) => {
                            //         let _ = serial.write(b"max30102 is Ok ");
                            //         let _ = serial.write(b"Current Die Temp: ");
                            //         let _ = serial.write(buf.format(temp as i32).as_bytes());
                            //     }
                            //     Err(_) => {
                            //         let _ = serial.write(b"max30102 failed ");
                            //     }
                            // }
                            // let _ = serial.write(buf.format().as_bytes());
                            let mut buf: String<64> = String::new();
                            write!(&mut buf, "Red: {:?}", latest_red).unwrap();
                            let _ = serial.write(buf.as_bytes());
                            let _ = serial.write(b" ");
                            let mut buf: String<64> = String::new();
                            write!(&mut buf, "IR: {:?}", latest_ir).unwrap();
                            let _ = serial.write(buf.as_bytes());
                            let _ = serial.write(b" ");
                            // 2. TMP117の処理 (約1秒 = 50ループに1回だけ実行)
                            if counter % 50 == 0 {
                                let mut temp_buf = [0u8; 2];
                                if i2c1.write_read(0x48, &[0x00], &mut temp_buf).is_ok() {
                                    // バッファから16bit値を取り出して温度に変換
                                    let raw_temp = i16::from_be_bytes(temp_buf) as f32;
                                    let temperature = raw_temp * 0.0078125; // TMP117の分解能
                                    
                                    let mut buf: String<64> = String::new();
                                    write!(&mut buf, "tmp: {:?}", temperature).unwrap();
                                    let _ = serial.write(buf.as_bytes());
                                    let _ = serial.write(b" ");
                                }
                            }
                            counter = counter.wrapping_add(1);

                            let _ = serial.write(b"\r\n");
                        }
                    }
                    events::Event::UsbUpdate => {
                        if usb_dev.poll(&mut [&mut serial]) {
                            let mut buf = [0u8; 64];
                            match serial.read(&mut buf) {
                                Ok(count) if count > 0 => {
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }
}

#[interrupt]
fn TIMER1() {
    free(|cs| {
        let counter = TIMER_COUNTER.borrow(cs).get();
        let polling_rate = POLLINGRATE.borrow(cs).get();
        let mut opt = TIMER1IRQ_ITEMS.borrow(cs).borrow_mut().take();
        if let Some((mut timer)) = opt {
            timer.reset_event();
            *TIMER1IRQ_ITEMS.borrow(cs).borrow_mut() = Some((timer));
            let div = FREQUENCY / polling_rate;
            if counter % div == 0 {
                events::post_event(events::Event::UsbUpdate);
            }
            let div = div * 20;
            if counter % div == 0 {
                events::post_event(events::Event::ReadSensor);
                events::post_event(events::Event::CdcUpdate);
            }
            let div = div * 5;
            if counter % div == 0 {
                events::post_event(events::Event::LedUpdate);
            }
            TIMER_COUNTER.borrow(cs).set(counter + 1);
        }
    });
}
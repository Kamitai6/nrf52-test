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
use nrf52840_hal::{
    clocks::Clocks,
    pac::{self, interrupt, NVIC},
    gpio::Level,
    usbd::{Usbd, UsbPeripheral},
    timer::{Timer, Periodic},
};
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

pub const FREQUENCY: u32 = 2000;
pub const PERIOD_US: u32 = 500;
pub type Timer1IrqItems<'a> = (Timer<pac::TIMER1, Periodic>);
pub static TIMER1IRQ_ITEMS: Mutex<RefCell<Option<Timer1IrqItems>>> = Mutex::new(RefCell::new(None));
pub static TIMER_COUNTER: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));
// pub static TIMER_COUNTER: AtomicU32 = AtomicU32::new(0);
pub static POLLINGRATE: Mutex<Cell<u32>> = Mutex::new(Cell::new(1000));
pub static mut CONTROL_BUFFER: UnsafeCell<[u8; 256]> = UnsafeCell::new([0; 256]);

// ledはアクティブロー
// LSM6DS3TR-Cが乗っているらしい
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

    let mut timer = Timer::periodic(pac.TIMER1);
    timer.enable_interrupt();
    timer.start(PERIOD_US);

    free(|cs| {
        TIMER1IRQ_ITEMS.borrow(cs).replace(Some((timer)));
    });

    unsafe {
        NVIC::unmask(pac::Interrupt::TIMER1);
    }

    loop {
        // イベントがあるかチェック
        if events::has_pending_events() {
            // 全てのイベントを処理
            while let Some(event) = events::get_event() {
                match event {
                    events::Event::LedUpdate => {
                        led.toggle().unwrap();
                    }
                    events::Event::CdcUpdate => {
                        if usb_dev.state() == usb_device::device::UsbDeviceState::Configured {
                            let mut buf = Buffer::new();
                            let _ = serial.write(b"Hello world!");
                            let _ = serial.write(b" ");
                            let _ = serial.write(b"x: ");
                            let _ = serial.write(buf.format(0).as_bytes());
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
            let div = div * 10;
            if counter % div == 0 {
                events::post_event(events::Event::CdcUpdate);
            }
            let div = div * 10;
            if counter % div == 0 {
                events::post_event(events::Event::LedUpdate);
            }
            TIMER_COUNTER.borrow(cs).set(counter + 1);
        }
    });
}
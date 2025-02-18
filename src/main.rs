#![no_std]
#![no_main]

use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

use core::{
    cell::RefCell,
    sync::atomic::{AtomicU32, Ordering},
};
use core::any::type_name;

use cortex_m::{asm, interrupt::{Mutex}};
use cortex_m_rt::entry;
use pac::interrupt;
use stm32h7xx_hal::{pac, prelude::*, spi, timer};
use stm32h7xx_hal::{adc, delay::Delay, rcc::rec::AdcClkSel};
use stm32h7xx_hal::pwm::{self, FaultMonitor, Polarity};


fn print_type_of<T>(_: &T) {
    rprintln!("{}", type_name::<T>());
}
pub struct Drv8343Reg {
    pub fault_status: u8,
    pub diag_status: [u8; 3],
    pub control: [u8; 14],
}

static OVERFLOWS: AtomicU32 = AtomicU32::new(0);
static PHASE: AtomicU32 = AtomicU32::new(0);
static TIMER: Mutex<RefCell<Option<timer::Timer<pac::TIM2>>>> =
    Mutex::new(RefCell::new(None));
    
static PWM1: Mutex<RefCell<Option<pwm::Pwm<pac::TIM1, 0, pwm::ComplementaryEnabled>>>> =
    Mutex::new(RefCell::new(None));
static PWM2: Mutex<RefCell<Option<pwm::Pwm<pac::TIM1, 1, pwm::ComplementaryEnabled>>>> =
    Mutex::new(RefCell::new(None));
static PWM3: Mutex<RefCell<Option<pwm::Pwm<pac::TIM1, 2, pwm::ComplementaryEnabled>>>> =
    Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    rtt_init_print!();
    let mut cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    // Constrain and Freeze power
    rprintln!("Setup PWR...                  ");
    let pwr = dp.PWR.constrain();
    let pwrcfg = pwr.freeze();

    // Constrain and Freeze clock
    rprintln!("Setup RCC...                  ");
    let rcc = dp.RCC.constrain();
    let mut ccdr = rcc
        .sys_ck(100.MHz())
        .pll1_q_ck(100.MHz())
        .freeze(pwrcfg, &dp.SYSCFG);
    ccdr.peripheral.kernel_adc_clk_mux(AdcClkSel::Per);

    let gpioa = dp.GPIOA.split(ccdr.peripheral.GPIOA);
    let gpiob = dp.GPIOB.split(ccdr.peripheral.GPIOB);
    let gpioc = dp.GPIOC.split(ccdr.peripheral.GPIOC);
    let gpiod = dp.GPIOD.split(ccdr.peripheral.GPIOD);
    let gpioe = dp.GPIOE.split(ccdr.peripheral.GPIOE);

    let sck = gpioc.pc10.into_alternate();
    let miso = gpioc.pc11.into_alternate();
    let mosi = gpioc.pc12.into_alternate();
    let mut nss = gpioa.pa15.into_push_pull_output();

    let mut en_gate = gpioa.pa4.into_push_pull_output();
    en_gate.set_high();

    let mut led = gpiod.pd0.into_push_pull_output();
    let mut delay = cp.SYST.delay(ccdr.clocks);

    let mut adc3 = adc::Adc::adc3(
        dp.ADC3,
        4.MHz(),
        &mut delay,
        ccdr.peripheral.ADC3,
        &ccdr.clocks,
    )
    .enable();
    adc3.set_resolution(adc::Resolution::SixteenBit); //16bit

    let mut control_timer = dp.TIM2.timer(10.Hz(), ccdr.peripheral.TIM2, &ccdr.clocks);
    
    // let mut pwm = dp.TIM1.pwm(
    //     gpioe.pe9.into_alternate(),
    //     10.kHz(),
    //     ccdr.peripheral.TIM1,
    //     &ccdr.clocks,
    // );
    let t1builder = dp.TIM1.pwm_advanced(
            (
                gpioe.pe9.into_alternate(),
                gpioe.pe11.into_alternate(),
                gpioe.pe13.into_alternate(),
            ),
            ccdr.peripheral.TIM1,
            &ccdr.clocks,
        )
        .frequency(10.kHz()) // max 200kHz
        .with_deadtime(1.micros())
        // .with_break_pin(gpioe.pe15.into_alternate(), Polarity::ActiveLow)
        .center_aligned();
        // .finalize();

    let (mut t1control, (t1c1, t1c2, t1c3)) = t1builder.finalize();
    let mut t1c1 = t1c1
        .into_complementary(gpioe.pe8.into_alternate());
        // .into_active_low();
    let mut t1c2 = t1c2
        .into_complementary(gpioe.pe10.into_alternate());
        // .into_active_low();
    let mut t1c3 = t1c3
        .into_complementary(gpioe.pe12.into_alternate());
        // .into_comp_active_low();

    print_type_of(&t1c1);
    print_type_of(&t1c2);
    print_type_of(&t1c3);

    let period = t1c1.get_max_duty();
    t1c1.set_duty(period / 2);
    t1c2.set_duty(period / 2);
    t1c3.set_duty(period / 2);

    t1c1.enable();
    t1c2.enable();
    t1c3.enable();

    // let mut channel = gpioa.pa6.into_analog();
    let mut channel = gpioc.pc2.into_analog();

    let mut spi: spi::Spi<_, _, u16> = dp.SPI3.spi(
        (sck, miso, mosi),
        spi::MODE_2, // or MODE_1?
        500.kHz(),
        ccdr.peripheral.SPI3,
        &ccdr.clocks,
    );
    nss.set_high();
    led.set_high();

    delay.delay_us(100_u16);

    let mut spi_buffer: [u16; 1] = [(0b1 << 15) | (0b0000110 << 8) | 0b00000000]; //fault statusを読みたい

    nss.set_low();
    led.set_low();
    let result = spi.transfer(&mut spi_buffer);
    match result {
        Ok(values) => {
            for (i, &value) in values.iter().enumerate() {
                rprintln!("Received data {}: {:#018b}", i, value);
            }
        }
        Err(e) => rprintln!("Error: {:?}", e),
    }
    nss.set_high();
    led.set_high();

    control_timer.listen(timer::Event::TimeOut);

    let tim2_ptr = unsafe { &*pac::TIM2::ptr() };
    rprintln!("dier: {:#010x}", tim2_ptr.dier.read().bits());
    rprintln!("sr: {:#010x}", tim2_ptr.sr.read().bits());
    rprintln!("arr: {:#010x}", tim2_ptr.arr.read().bits());

    cortex_m::interrupt::free(|cs| {
        TIMER.borrow(cs).replace(Some(control_timer));
        PWM1.borrow(cs).replace(Some(t1c1));
        PWM2.borrow(cs).replace(Some(t1c2));
        PWM3.borrow(cs).replace(Some(t1c3));
    });

    unsafe {
        // cp.NVIC.set_priority(pac::interrupt::TIM2, 0);
        pac::NVIC::unmask(pac::interrupt::TIM2);
    }

    rprintln!("iser0: {:#010x}", cp.NVIC.iser[0].read());
    rprintln!("iser1: {:#010x}", cp.NVIC.iser[1].read());
    rprintln!("iser2: {:#010x}", cp.NVIC.iser[2].read());
    rprintln!("iser3: {:#010x}", cp.NVIC.iser[3].read());
    rprintln!("iser4: {:#010x}", cp.NVIC.iser[4].read());
    rprintln!("iser5: {:#010x}", cp.NVIC.iser[5].read());
    rprintln!("iser6: {:#010x}", cp.NVIC.iser[6].read());
    rprintln!("iser7: {:#010x}", cp.NVIC.iser[7].read());

    // let tim2_ptr = unsafe { &*pac::TIM2::ptr() };
    // rprintln!("cr1: {:#010x}", tim2_ptr.);

    loop {
        // if t1control.is_fault_active() {
        //     // Fault is active, turn on LED
        //     led.set_high();
        // } else {
        //     // Fault is not active
        //     led.set_low();
        // }

        // let data: u32 = adc3.read(&mut channel).unwrap();
        //// voltage = reading * (vref/resolution)
        // rprintln!(
        //     "ADC reading: {}, voltage: {}",
        //     data,
        //     data as f32 * (3.3 / adc3.slope() as f32)
        // );
        let ctr = cortex_m::interrupt::free(|cs| {
            let rc = TIMER.borrow(cs).borrow();
            let timer = rc.as_ref().unwrap();
            timer.counter() as u64
        });
        rprintln!("count: {}", ctr);
        rprintln!("phase: {}", PHASE.load(Ordering::Relaxed));
        rprintln!("overflows: {}", OVERFLOWS.load(Ordering::SeqCst) as u64);
    }
}

#[interrupt]
fn TIM2() {
    // rprintln!("interrupt!!!");
    OVERFLOWS.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
    cortex_m::interrupt::free(|cs| {
        let mut rc = TIMER.borrow(cs).borrow_mut();
        let timer = rc.as_mut().unwrap();
        timer.clear_irq();

        let mut rc = PWM1.borrow(cs).borrow_mut();
        let pwm1 = rc.as_mut().unwrap();

        let mut rc = PWM2.borrow(cs).borrow_mut();
        let pwm2 = rc.as_mut().unwrap();

        let mut rc = PWM3.borrow(cs).borrow_mut();
        let pwm3 = rc.as_mut().unwrap();

        let period = pwm1.get_max_duty();
        match PHASE.load(Ordering::Relaxed) {
            0 => {
                pwm1.set_duty(period / 4);  // A+
                pwm2.set_duty(0);           // B-
                pwm3.set_duty(period / 8);  // Cフロート
            }
            1 => {
                pwm1.set_duty(period / 4);  // A+
                pwm2.set_duty(period / 8);  // Bフロート
                pwm3.set_duty(0);           // C-
            }
            2 => {
                pwm1.set_duty(period / 8);  // Aフロート
                pwm2.set_duty(period / 4);  // B+
                pwm3.set_duty(0);           // C-
            }
            3 => {
                pwm1.set_duty(0);           // A-
                pwm2.set_duty(period / 4);  // B+
                pwm3.set_duty(period / 8);  // Cフロート
            }
            4 => {
                pwm1.set_duty(0);           // A-
                pwm2.set_duty(period / 8);  // Bフロート
                pwm3.set_duty(period / 4);  // C+
            }
            5 => {
                pwm1.set_duty(period / 8);  // Aフロート
                pwm2.set_duty(0);           // B-
                pwm3.set_duty(period / 4);  // C+
            }
            _ => {}
        }
        let _ = PHASE.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |phase| {
            Some((phase + 1) % 6)
        });
    })
}
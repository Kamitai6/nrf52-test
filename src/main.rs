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
use stm32h7xx_hal::{pac, prelude::*, spi, timer, gpio};
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

fn transfer_spi(rw: bool, nss: &mut gpio::Pin<'A', 15, gpio::Output>, spi: &mut spi::Spi<pac::SPI3, spi::Enabled, u16>, spi_buffer: &mut [u16; 1]) {
    nss.set_low();
    // led.set_low();
    let result = {
        if rw {
            spi.transfer(spi_buffer).unwrap()
        }
        else {
            spi.write(spi_buffer);
            spi_buffer
        }
    };
    for (i, &value) in result.iter().enumerate() {
        // rprintln!("Received data {}: {:#010b}", i, value);
        rprintln!("Received data {}: {:#018b}", i, value);
    }
    nss.set_high();
    // led.set_high();
}

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

    let mut en_gate = gpioa.pa4.into_push_pull_output(); //default low
    let mut led = gpiod.pd0.into_push_pull_output();
    let mut delay = cp.SYST.delay(ccdr.clocks);

    // en_gate.set_low();
    en_gate.set_high();
    delay.delay_us(10_u16);

    // let mut adc3 = adc::Adc::adc3(
    //     dp.ADC3,
    //     4.MHz(),
    //     &mut delay,
    //     ccdr.peripheral.ADC3,
    //     &ccdr.clocks,
    // )
    // .enable();
    // adc3.set_resolution(adc::Resolution::SixteenBit); //16bit

    // let mut channel = gpioa.pa6.into_analog();
    // let mut channel = gpioc.pc2.into_analog();

    let mut spi: spi::Spi<_, _, u16> = dp.SPI3.spi(
        (sck, miso, mosi),
        spi::MODE_1,
        5.MHz(),
        // 500.kHz(),
        ccdr.peripheral.SPI3,
        &ccdr.clocks,
    );
    nss.set_high();
    led.set_high();

    // en_gate.set_high();
    // delay.delay_ms(1000_u16);
    // en_gate.set_high();
    // delay.delay_ms(1000_u16);
    // en_gate.set_high();
    // delay.delay_ms(1000_u16);

    let mut spi_buffer: [u16; 1] = [0; 1];

    spi_buffer = [(0b10000000 << 8) | 0b00000000]; //read fault
    transfer_spi(true, &mut nss, &mut spi, &mut spi_buffer);
    delay.delay_us(10_u16);
    spi_buffer = [(0b00001101 << 8) | 0b01100000]; // write lock free
    transfer_spi(false, &mut nss, &mut spi, &mut spi_buffer);
    delay.delay_us(10_u16);
    spi_buffer = [(0b00000100 << 8) | 0b10000000]; // write clear fault
    transfer_spi(false, &mut nss, &mut spi, &mut spi_buffer);
    delay.delay_us(10_u16);
    spi_buffer = [(0b00001110 << 8) | 0b00000010]; // write ocp mode
    transfer_spi(false, &mut nss, &mut spi, &mut spi_buffer);
    delay.delay_us(10_u16);
    // spi_buffer = [(0b10001110 << 8) | 0b00000000]; //read ic11
    // transfer_spi(true, &mut nss, &mut spi, &mut spi_buffer);
    // delay.delay_us(10_u16);
    spi_buffer = [(0b10000000 << 8) | 0b00000000]; //read fault
    transfer_spi(true, &mut nss, &mut spi, &mut spi_buffer);
    delay.delay_us(10_u16);
    spi_buffer = [(0b10000111 << 8) | 0b00000000]; //read communication check
    transfer_spi(true, &mut nss, &mut spi, &mut spi_buffer);
    delay.delay_us(10_u16);

    let (mut t1control, (t1c1, t1c2, t1c3)) = dp.TIM1.pwm_advanced(
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
    .center_aligned()
    .finalize();

    let mut t1c1 = t1c1
        .into_complementary(gpioe.pe8.into_alternate());
    let mut t1c2 = t1c2
        .into_complementary(gpioe.pe10.into_alternate());
    let mut t1c3 = t1c3
        .into_complementary(gpioe.pe12.into_alternate());

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

    let mut control_timer = dp.TIM2.timer(10.Hz(), ccdr.peripheral.TIM2, &ccdr.clocks);
    control_timer.listen(timer::Event::TimeOut);

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

        spi_buffer = [(0b10000000 << 8) | 0b00000000]; //read fault
        transfer_spi(true, &mut nss, &mut spi, &mut spi_buffer);
        delay.delay_us(10_u16);

        // let data: u32 = adc3.read(&mut channel).unwrap();
        //// voltage = reading * (vref/resolution)
        // rprintln!(
        //     "ADC reading: {}, voltage: {}",
        //     data,
        //     data as f32 * (3.3 / adc3.slope() as f32)
        // );

        // let ctr = cortex_m::interrupt::free(|cs| {
        //     let rc = TIMER.borrow(cs).borrow();
        //     let timer = rc.as_ref().unwrap();
        //     timer.counter() as u64
        // });
        // rprintln!("count: {}", ctr);
        // rprintln!("phase: {}", PHASE.load(Ordering::Relaxed));
        // rprintln!("overflows: {}", OVERFLOWS.load(Ordering::SeqCst) as u64);
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
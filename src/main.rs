#![no_std]
#![no_main]

use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

// use rtic::app;

// use cmsis_dsp_api as dsp_api;
// use cmsis_dsp_sys as dsp_sys;

use core::cell::{Cell, RefCell};
use cortex_m::interrupt::{free, Mutex};
use cortex_m::delay::Delay;
use cortex_m::peripheral::NVIC;
use cortex_m_rt::entry;
use hal::{
    clocks::{Clocks, PllCfg},
    dma::{self, Dma, DmaChannel, DmaInput, DmaInterrupt, DmaPeriph},
    gpio::{self, Pin, PinMode, Port},
    pac::{self, DMA1, SPI2, interrupt},
    spi::{self, BaudRate, Spi, SpiConfig, SpiMode},
};

use hal::{
    // clocks::Clocks,
    // gpio::{Edge, Pin, PinMode, Port},
    low_power, 
    timer::{self,
        Alignment, UpdateReqSrc, CaptureCompareDma, BasicTimer, CaptureCompare, CountDir, InputSlaveMode, InputTrigger,
        MasterModeSelection, OutputCompare, TimChannel, Timer, TimerConfig, TimerInterrupt,
    },
};

// use rtic_monotonics::systick::prelude::*;
// use stm32h7xx_hal::timer::Timer;

// systick_monotonic!(Mono, 1000);

#[link_section = ".sram3"]
static mut SPI_READ_BUF: [u8; 8] = [0; 8];

#[link_section = ".sram3"]
static mut SPI_WRITE_BUF: [u8; 8] = [0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x13, 0xEA];

static SPI: Mutex<RefCell<Option<Spi<SPI2>>>> = Mutex::new(RefCell::new(None));
static NSS: Mutex<RefCell<Option<Pin>>> = Mutex::new(RefCell::new(None));


#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("Start!!!");

    let mut cp = cortex_m::Peripherals::take().unwrap();
    let mut dp = pac::Peripherals::take().unwrap();

    let mut clock_cfg = Clocks::default(); //400MHz

    // ---------------clock---------------
    clock_cfg.pll1 = PllCfg {
        enabled: true,
        pllp_en: true,
        pllq_en: true,
        pllr_en: true,
        divm: 32,
        divn: 200,
        divp: 2,
        divq: 4,
        divr: 2,
    };

    clock_cfg.setup().unwrap();

    let mut delay = Delay::new(cp.SYST, clock_cfg.systick());
    // let mut led = Pin::new(Port::D, 0, PinMode::Output);
    let _pwm_pin = Pin::new(Port::B, 1, PinMode::Alt(1));

    let mut pwm_timer = Timer::new_tim1(
        dp.TIM1,
        1_0000.,
        TimerConfig {
            auto_reload_preload: true,
            // Setting auto reload preload allow changing frequency (period) while the timer is running.
            ..Default::default()
        },
        &clock_cfg,
    );

    pwm_timer.enable_pwm_output(TimChannel::C3, OutputCompare::Pwm2, 1.0);
    // pwm_timer.enable();

    // let mut countdown_timer = Timer::new_tim3(
    //     dp.TIM3, 
    //     1., 
    //     TimerConfig {
    //         one_pulse_mode: true,
    //         update_request_source: UpdateReqSrc::Any,
    //         auto_reload_preload: true,
    //         alignment: Alignment::Edge,
    //         capture_compare_dma: CaptureCompareDma::Ccx,
    //         direction: CountDir::Down,
    //     }, 
    //     &clock_cfg
    // );
    // countdown_timer.enable_interrupt(TimerInterrupt::Update); // Enable update event interrupts.

    // countdown_timer.enable(); // Start the counter.

    // unsafe {
    //     NVIC::unmask(pac::Interrupt::TIM3);
    // }

    let mut pwm = 1.0;
    loop {
        // pwm_timer.set_duty(TimChannel::C3, (pwm_timer.get_max_duty() as f32 * pwm) as u16);
        // // pwm_timer.enable_pwm_output(TimChannel::C3, OutputCompare::Pwm2, pwm);
        // delay.delay_ms(1_00);
        // pwm -= 0.1;
        // if pwm < 0. {
        //     pwm = 1.0;
        // }
    }
}

// #[interrupt]
// /// Timer interrupt handler; runs when the countdown period expires.
// fn TIM3() {
//     timer::clear_update_interrupt(3);

//     // Do something.
// }
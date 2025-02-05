#![no_std]
#![no_main]

use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

use core::cell::{Cell, RefCell};

use cortex_m::peripheral::NVIC;
use cortex_m::delay::Delay;
use cortex_m_rt::entry;
use hal::{
    clocks::{Clocks, PllCfg},
    dma::{self, Dma, DmaChannel, DmaInput, DmaInterrupt, DmaPeriph},
    gpio::{self, Pin, PinMode, Port},
    low_power,
    pac::{self, interrupt},
    prelude::*,
    spi::{self, BaudRate, Spi, SpiConfig, SpiMode},
};

// Byte 0 is for the address we pass in the `write` transfer; relevant data is in the rest of
// the values.
static mut SPI_READ_BUF: [u8; 8] = [0; 8];
static mut SPI_WRITE_BUF: [u8; 8] = [0x00, 0x00, 0xAA, 0xAA, 0x00, 0x00, 0xD0, 0xAB];

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("Start!!!");
    // Set up CPU peripherals
    let mut cp = cortex_m::Peripherals::take().unwrap();
    // Set up microcontroller peripherals
    let mut dp = pac::Peripherals::take().unwrap();

    let mut clock_cfg = Clocks::default(); //400MHz

    // ---------------clock---------------
    // hsi(pllsrc) / divm -> pll1-ref-clk
    // pll1-ref-clk * divp(or divq or divr) -> pll1p(or pll1q or pll1r) clk
    clock_cfg.pll1 = PllCfg {
        enabled: true,
        // fractional: false,
        pllp_en: true,
        pllq_en: true,
        pllr_en: true,
        divm: 32,
        divn: 200,
        divp: 2,// pll1p clock = 12.5 * 2 = 25MHz
        divq: 4,// pll1q clock = 12.5 * 8 = 100MHz
        divr: 2,// pll1r clock = 12.5 * 2 = 25MHz
    };

    clock_cfg.setup().unwrap();

    // Configure pins for Spi
    let _sck = Pin::new(Port::A, 12, PinMode::Alt(5));
    let _miso = Pin::new(Port::B, 14, PinMode::Alt(5));
    let _mosi = Pin::new(Port::B, 15, PinMode::Alt(5));

    let mut cs = Pin::new(Port::A, 11, PinMode::Output);

    let mut led = Pin::new(Port::D, 0, PinMode::Output);
    let mut delay = Delay::new(cp.SYST, clock_cfg.systick());

    let spi_cfg = SpiConfig {
        mode: SpiMode::mode1(),
        // `SpiConfig::default` is mode 0, full duplex, with software CS.
        ..Default::default()
    };

    // Set up an SPI peripheral, running at 4Mhz, in SPI mode 0.
    let mut spi = Spi::new(
        dp.SPI2,
        spi_cfg,
        BaudRate::Div64, // 100MHz / 64 = 1.5625MHz
    );

    let rcc = unsafe { &*pac::RCC::ptr() };
    rprintln!("ahb1enr: {:#010x}", rcc.ahb1enr.read().bits());
    rprintln!("ahb1lpenr: {:#010x}", rcc.ahb1lpenr.read().bits());
    rprintln!("ahb2enr: {:#010x}", rcc.ahb2enr.read().bits());
    rprintln!("ahb2lpenr: {:#010x}", rcc.ahb2lpenr.read().bits());
    rprintln!("ahb3enr: {:#010x}", rcc.ahb3enr.read().bits());
    rprintln!("ahb3lpenr: {:#010x}", rcc.ahb3lpenr.read().bits());
    rprintln!("ahb4enr: {:#010x}", rcc.ahb4enr.read().bits());
    rprintln!("ahb4lpenr: {:#010x}", rcc.ahb4lpenr.read().bits());

    rprintln!("ahb1enr: {:#010x}", rcc.ahb1enr.read().bits());
    rprintln!("ahb1enr: {:#010x}", rcc.ahb1enr.read().bits());
    rprintln!("apb1henr: {:#010x}", rcc.apb1henr.read().bits());
    rprintln!("apb1lenr: {:#010x}", rcc.apb1lenr.read().bits());
    rprintln!("apb1hlpenr: {:#010x}", rcc.apb1hlpenr.read().bits());
    rprintln!("apb1llpenr: {:#010x}", rcc.apb1llpenr.read().bits());
    rprintln!("apb2lpenr: {:#010x}", rcc.apb2lpenr.read().bits());
    rprintln!("apb2enr: {:#010x}", rcc.apb2enr.read().bits());
    rprintln!("apb3enr: {:#010x}", rcc.apb3enr.read().bits());
    rprintln!("apb3lpenr: {:#010x}", rcc.apb3lpenr.read().bits());
    rprintln!("apb4enr: {:#010x}", rcc.apb4enr.read().bits());
    rprintln!("apb4lpenr: {:#010x}", rcc.apb4lpenr.read().bits());

    cs.set_high();
    led.set_high();

    let mut spi_buf: [u8; 8] = [0x00, 0x00, 0xAA, 0xAA, 0x00, 0x00, 0xD0, 0xAB];
    cs.set_low();
    // delay.delay_us(1_00);
    spi.transfer(&mut spi_buf).ok();
    let values = spi_buf;
    for (i, &value) in values.iter().enumerate() {
        rprintln!("Received data 1 {}: {}", i, value);
    }
    cs.set_high();

    loop {
        delay.delay_us(8_00);
        spi_buf = [0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x13, 0xEA];
        cs.set_low();
        // delay.delay_us(1_00);
        spi.transfer(&mut spi_buf).ok();
        let values = spi_buf;
        for (i, &value) in values.iter().enumerate() {
            rprintln!("Received data 2 {}: {}", i, value);
        }
        let angle_lsb = ((values[1] & 0x3F) as u16) << 8 | (values[0] as u16);
        rprintln!("angle {}", angle_lsb);
        let error_lsb = (values[1] as u16) >> 6;
        rprintln!("error {}", error_lsb);
        let crc_lsb = (values[7] as u16);
        rprintln!("crc {}", crc_lsb);
        let vgain_lsb = (values[4] as u16);
        rprintln!("vgain {}", vgain_lsb);
        let rollcnt_lsb = (values[6] as u16) & 0x3F;
        rprintln!("rollcnt {}", rollcnt_lsb);
        cs.set_high();
    }
}
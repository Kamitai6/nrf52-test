#![no_std]
#![no_main]

use core::mem::MaybeUninit;
use core::marker::PhantomData;
use core::ptr::addr_of_mut;
use core::sync::atomic::{AtomicU32, Ordering};

use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

use rtic::app;

use stm32h7xx_hal::{ethernet, stm32, gpio, spi, dma, prelude::*};
use stm32h7xx_hal::{ethernet::smoltcp};

use smoltcp::iface::{Config, Interface, SocketSet, SocketStorage};
use smoltcp::time::Instant;
use smoltcp::wire::{HardwareAddress, IpAddress, IpCidr};

mod communication;
use communication::net::*;

/// TIME is an atomic u32 that counts milliseconds. Although not used
/// here, it is very useful to have for network protocols
static TIME: AtomicU32 = AtomicU32::new(0);

/// Locally administered MAC address
const MAC_ADDRESS: [u8; 6] = [0x02, 0x00, 0x11, 0x22, 0x33, 0x44];

const BUFFER_SIZE: usize = 100;

#[link_section = ".axisram.buffers"]
static mut BUFFER: MaybeUninit<[u8; BUFFER_SIZE]> = MaybeUninit::uninit();

static mut WRITE_BUF: [u8; 13] = [0; 13];

// IMU readings buffer. 3 accelerometer, and 3 gyro measurements; 2 bytes each. 0-padded on the left,
// since that's where we pass the register in the write buffer.
// This buffer is static, to ensure it lives through the life of the program.
pub static mut IMU_READINGS: [u8; 13] = [0; 13];

/// Utility function to write a single byte.
fn write_one(reg: Reg, word: u8, spi: &mut Spi<SPI1>, cs: &mut Pin) {
    cs.set_low();
    spi.write(&[reg as u8, word]).ok();
    cs.set_high();
}

pub fn setup(spi: &mut Spi<SPI1>, cs: &mut Pin, delay: &mut Delay) {
    // Leave default of SPI mode 0 and 3.

    // Enable gyros and accelerometers in low noise mode.
    write_one(Reg::PwrMgmt0, 0b0000_1111, spi, cs);

    // Set gyros and accelerometers to 8kHz update rate, 2000 DPS gyro full scale range,
    // and +-16g accelerometer full scale range.
    write_one(Reg::GyroConfig0, 0b0000_0011, spi, cs);
    // "When transitioning from OFF to any of the other modes, do not issue any
    // register writes for 200µs." (Gyro and accel)
    delay.delay_us(200);

    write_one(Reg::AccelConfig0, 0b0000_0011, spi, cs);
    delay.delay_us(200);

    // (Leave default interrupt settings of active low, push pull, pulsed.)

    // Enable UI data ready interrupt routed to the INT1 pin.
    write_one(Reg::IntSource0, 0b0000_1000, spi, cs);
}

/// We use this to assemble readings from the DMA buffer.
pub fn from_buffer(buf: &[u8]) -> Self {
    // todo: Note: this mapping may be different for diff IMUs, eg if they use a different reading register ordering.
    // todo: Currently hard-set for ICM426xx.

    // Ignore byte 0; it's for the first reg passed during the `write` transfer.
    Self {
        a_x: interpret_accel(i16::from_be_bytes([buf[1], buf[2]])),
        a_y: interpret_accel(i16::from_be_bytes([buf[3], buf[4]])),
        a_z: interpret_accel(i16::from_be_bytes([buf[5], buf[6]])),
        v_pitch: interpret_gyro(i16::from_be_bytes([buf[7], buf[8]])),
        v_roll: interpret_gyro(i16::from_be_bytes([buf[9], buf[10]])),
        v_yaw: interpret_gyro(i16::from_be_bytes([buf[11], buf[12]])),
    }
}

pub fn read_imu_dma(starting_addr: u8, spi: &mut Spi<SPI1>, cs: &mut Pin) {
    // First byte is the first data reg, per this IMU's. Remaining bytes are empty, while
    // the MISO line transmits readings.
    // Note that we use a static buffer to ensure it lives throughout the DMA xfer.
    unsafe {
        WRITE_BUF[0] = starting_addr;
    }

    cs.set_low();

    unsafe {
        spi.transfer_dma(
            &WRITE_BUF,
            &mut crate::IMU_READINGS,
            DmaChannel::C1,
            DmaChannel::C2,
            Default::default(),
            Default::default(),
            DmaPeriph::Dma1,
        );
    }
}

#[app(device = stm32, peripherals = true)]
mod app {
    use stm32h7xx_hal::{ethernet, ethernet::PHY, gpio, prelude::*};
    use communication::dp83848::DP83848;

    use super::*;
    use core::sync::atomic::Ordering;

    #[shared]
    struct SharedResources {
        nss:gpio::gpioa::PA11<gpio::Output<gpio::PushPull>>,
        spi_write_transfer: dma::Transfer<
            dma::dma::Stream1<stm32::DMA1>,
            spi::Spi<stm32::SPI2, spi::Disabled, u8>,
            dma::PeripheralToMemory,
            &'static mut [u8; BUFFER_SIZE],
            dma::DBTransfer,
        >,
        spi_read_transfer: dma::Transfer<
            dma::dma::Stream2<stm32::DMA1>,
            spi::Spi<stm32::SPI2, spi::Disabled, u8>,
            dma::MemoryToPeripheral,
            &'static mut [u8; BUFFER_SIZE],
            dma::DBTransfer,
        >,
    }
    #[local]
    struct LocalResources {
        net: Net<'static>,
        dp83848: DP83848<ethernet::EthernetMAC>,
        link_led: gpio::gpiod::PD0<gpio::Output<gpio::PushPull>>,
    }

    #[init]
    fn init(mut ctx: init::Context) -> (SharedResources, LocalResources) {
        rtt_init_print!();
        // Initialise power...
        rprintln!("Setup PWR...                  ");
        let pwr = ctx.device.PWR.constrain();
        let pwrcfg = pwr.freeze();

        // Link the SRAM3 power state to CPU1
        ctx.device.RCC.ahb2enr.modify(|_, w| w.sram3en().set_bit());

        // Initialise clocks...
        rprintln!("Setup RCC...                  ");
        let mut rcc = ctx.device.RCC.constrain();
        let reason = rcc.get_reset_reason();
        let ccdr = rcc
            .sys_ck(200.MHz())
            .hclk(200.MHz())
            .pll1_q_ck(200.MHz())
            .freeze(pwrcfg, &ctx.device.SYSCFG);

        // Initialise system...
        ctx.core.SCB.enable_icache();
        // TODO: ETH DMA coherence issues
        // ctx.core.SCB.enable_dcache(&mut ctx.core.CPUID);
        ctx.core.DWT.enable_cycle_counter();

        rprintln!("Why rst -> : {}", reason);

        // Initialise IO...
        let gpioa = ctx.device.GPIOA.split(ccdr.peripheral.GPIOA);
        let gpiob = ctx.device.GPIOB.split(ccdr.peripheral.GPIOB);
        let gpioc = ctx.device.GPIOC.split(ccdr.peripheral.GPIOC);
        let gpiod = ctx.device.GPIOD.split(ccdr.peripheral.GPIOD);
        
        //spi
        let spi = {
            let sck = gpioa.pa12.into_alternate().speed(gpio::Speed::VeryHigh);
            let miso = gpiob.pb14.into_alternate().speed(gpio::Speed::VeryHigh);
            let mosi = gpiob.pb15.into_alternate().speed(gpio::Speed::VeryHigh);
            let config = spi::Config::new(spi::MODE_1)
                .communication_mode(spi::CommunicationMode::FullDuplex);

            let spi: spi::Spi<_, _, u8> = ctx.device.SPI2.spi(
                (sck, miso, mosi),
                config,
                2.MHz(),
                ccdr.peripheral.SPI2,
                &ccdr.clocks,
            );

            spi.disable()
        };
        let mut nss = gpioa.pa11.into_push_pull_output().speed(gpio::Speed::VeryHigh);
        nss.set_high();

        let tx_buffer: &'static mut [u8; BUFFER_SIZE] = {
            let buf: &mut [MaybeUninit<u8>; BUFFER_SIZE] = unsafe {
                &mut *(core::ptr::addr_of_mut!(BUFFER)
                    as *mut [MaybeUninit<u8>; BUFFER_SIZE])
            };

            for (i, value) in buf.iter_mut().enumerate() {
                unsafe {
                    value.as_mut_ptr().write(i as u8 + 0x60); // 0x60, 0x61, 0x62...
                }
            }

            #[allow(static_mut_refs)] // TODO: Fix this
            unsafe {
                BUFFER.assume_init_mut()
            }
        };
        let rx_buffer: &'static mut [u8; BUFFER_SIZE] = {
            let buf: &mut [MaybeUninit<u8>; BUFFER_SIZE] = unsafe {
                &mut *(core::ptr::addr_of_mut!(BUFFER)
                    as *mut [MaybeUninit<u8>; BUFFER_SIZE])
            };

            for (i, value) in buf.iter_mut().enumerate() {
                unsafe {
                    value.as_mut_ptr().write(i as u8 + 0x60); // 0x60, 0x61, 0x62...
                }
            }

            #[allow(static_mut_refs)] // TODO: Fix this
            unsafe {
                BUFFER.assume_init_mut()
            }
        };
        let streams = dma::dma::StreamsTuple::new(ctx.device.DMA1, ccdr.peripheral.DMA1);
        let tx_config = dma::dma::DmaConfig::default()
            .memory_increment(true)
            .transfer_complete_interrupt(true);
        let rx_config = dma::dma::DmaConfig::default()
            .memory_increment(true)
            .transfer_complete_interrupt(true);
        let spi_write_transfer: dma::Transfer<_, _, dma::PeripheralToMemory, _, _,>
             = dma::Transfer::init(streams.1, spi, tx_buffer, None, tx_config);
        let spi_read_transfer: dma::Transfer<_, _, dma::MemoryToPeripheral, _, _,>
             = dma::Transfer::init_const(streams.2, spi, rx_buffer, None, rx_config);

        dma::mux(DmaPeriph::Dma1, DmaChannel::C1, DmaInput::Spi1Tx);
        dma::mux(DmaPeriph::Dma1, DmaChannel::C2, DmaInput::Spi1Rx);

        // We use Spi transfer complete to know when our readings are ready.
        dma.enable_interrupt(DmaChannel::C2, DmaInterrupt::TransferComplete);

        unsafe {
            // Write to SPI, using DMA.
            // spi.write_dma(&write_buf, DmaChannel::C1, Default::default(), DmaPeriph::Dma2);
    
            // Read (transfer) from SPI, using DMA.
            spi.transfer_dma(
                // Write buffer, starting with the registers we'd like to access, and 0-padded to
                // read 3 bytes.
                &SPI_WRITE_BUF,
                &mut SPI_READ_BUF,  // Read buf, where the data will go
                DmaChannel::C1,     // Write channel
                DmaChannel::C2,     // Read channel
                Default::default(), // Write channel config
                Default::default(), // Read channel config
                DmaPeriph::Dma2,
            );
        }

        // ethernet
        let mut link_led = gpiod.pd0.into_push_pull_output();
        link_led.set_high();

        let rmii_ref_clk = gpioa.pa1.into_alternate();
        let rmii_mdio = gpioa.pa2.into_alternate();
        let rmii_mdc = gpioc.pc1.into_alternate();
        let rmii_crs_dv = gpioa.pa7.into_alternate();
        let rmii_rxd0 = gpioc.pc4.into_alternate();
        let rmii_rxd1 = gpioc.pc5.into_alternate();
        let rmii_tx_en = gpiob.pb11.into_alternate();
        let rmii_txd0 = gpiob.pb12.into_alternate();
        let rmii_txd1 = gpiob.pb13.into_alternate();

        assert_eq!(ccdr.clocks.hclk().raw(), 200_000_000); // HCLK 200MHz
        assert_eq!(ccdr.clocks.pclk1().raw(), 100_000_000); // PCLK 100MHz
        assert_eq!(ccdr.clocks.pclk2().raw(), 100_000_000); // PCLK 100MHz
        assert_eq!(ccdr.clocks.pclk4().raw(), 100_000_000); // PCLK 100MHz

        let mac_addr = smoltcp::wire::EthernetAddress::from_bytes(&MAC_ADDRESS);
        let (eth_dma, eth_mac) = unsafe {
            #[allow(static_mut_refs)] // TODO: Fix this
            DES_RING.write(ethernet::DesRing::new());

            ethernet::new(
                ctx.device.ETHERNET_MAC,
                ctx.device.ETHERNET_MTL,
                ctx.device.ETHERNET_DMA,
                (
                    rmii_ref_clk,
                    rmii_mdio,
                    rmii_mdc,
                    rmii_crs_dv,
                    rmii_rxd0,
                    rmii_rxd1,
                    rmii_tx_en,
                    rmii_txd0,
                    rmii_txd1,
                ),
                #[allow(static_mut_refs)] // TODO: Fix this
                DES_RING.assume_init_mut(),
                mac_addr,
                ccdr.peripheral.ETH1MAC,
                &ccdr.clocks,
            )
        };

        // Initialise ethernet PHY...
        let eth_mac_custom = eth_mac.set_phy_addr(0x01);
        let mut dp83848 = DP83848::new(eth_mac_custom);
        dp83848.phy_reset();
        dp83848.phy_init();

        unsafe { ethernet::enable_interrupt() };

        // unsafe: mutable reference to static storage, we only do this once
        let store = unsafe {
            #[allow(static_mut_refs)] // TODO: Fix this
            let store_ptr = STORE.as_mut_ptr();

            // Initialise the socket_storage field. Using `write` instead of
            // assignment via `=` to not call `drop` on the old, uninitialised
            // value
            addr_of_mut!((*store_ptr).socket_storage)
                .write([SocketStorage::EMPTY; 8]);

            // Now that all fields are initialised we can safely use
            // assume_init_mut to return a mutable reference to STORE
            #[allow(static_mut_refs)] // TODO: Fix this
            STORE.assume_init_mut()
        };

        let net = Net::new(store, eth_dma, mac_addr.into(), Instant::ZERO);

        (
            SharedResources {
                nss,
                spi_write_transfer,
                spi_read_transfer,
            },
            LocalResources {
                net,
                dp83848,
                link_led,
            },
        )
    }

    #[idle(local = [dp83848, link_led], shared = [spi_write_transfer, spi_read_transfer, nss])]
    fn idle(ctx: idle::Context) -> ! {
        // Start the DMA transfer over SPI.
        (ctx.shared.spi_write_transfer, ctx.shared.spi_read_transfer, ctx.shared.nss).lock(|spi_tx, spi_rx, nss| {
            spi_tx.start(|spi| {
                // Set CS low for the transfer.
                nss.set_low();

                // Enable TX DMA support, enable the SPI peripheral, and start the transaction.
                spi.enable_dma_tx();
                spi.inner_mut().cr1.modify(|_, w| w.spe().enabled());
                spi.inner_mut().cr1.modify(|_, w| w.cstart().started());

                // The transaction immediately begins as the TX FIFO is now being filled by DMA.
            });
            spi_rx.start(|spi| {
                spi.enable_dma_rx();
            });
        });

        loop {
            asm::nop();
            // Ethernet
            let status = ctx.local.dp83848.poll_link();
            rprintln!("status: {}", status);
            if status == 0 {
                ctx.local.link_led.set_low();
            }

            // let mut spi_buffer = [0x00, 0x00, 0xAA, 0xAA, 0x00, 0x00, 0xD0, 0xAB];

            // ctx.local.nss.set_low();
            // let result = ctx.local.spi.transfer(&mut spi_buffer);
            // match result {
            //     Ok(values) => {
            //         for (i, &value) in values.iter().enumerate() {
            //             rprintln!("Received data {}: {}", i, value);
            //         }
            //     }
            //     Err(e) => rprintln!("Error: {:?}", e),
            // }
            // ctx.local.nss.set_high();

            // spi_buffer = [0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x13, 0xEA];
            // ctx.local.nss.set_low();
            // let result2 = ctx.local.spi.transfer(&mut spi_buffer);
            // match result2 {
            //     Ok(values) => {
            //         for (i, &value) in values.iter().enumerate() {
            //             rprintln!("Received data {}: {}", i, value);
            //         }
            //         let angle_lsb = ((values[1] & 0x3F) as u16) << 8 | (values[0] as u16);
            //         rprintln!("angle {}", angle_lsb);
            //         let error_lsb = (values[1] as u16) >> 6;
            //         rprintln!("error {}", error_lsb);
            //         let crc_lsb = (values[7] as u16);
            //         rprintln!("crc {}", crc_lsb);
            //         let vgain_lsb = (values[4] as u16);
            //         rprintln!("vgain {}", vgain_lsb);
            //         let rollcnt_lsb = (values[6] as u16) & 0x3F;
            //         rprintln!("rollcnt {}", rollcnt_lsb);
            //     }
            //     Err(e) => rprintln!("Error: {:?}", e),
            // }
            // ctx.local.nss.set_high();
        }
    }

    /// Runs when new IMU data is ready. Trigger a DMA read.
    #[task(binds = EXTI4, shared = [cs_imu, dma, spi1], priority = 1)]
    fn imu_data_isr(cx: imu_data_isr::Context) {
        gpio::clear_exti_interrupt(4);

        (cx.shared.cs_imu, cx.shared.spi1).lock(|cs_imu, spi| {
            imu::read_imu_dma(imu::READINGS_START_ADDR, spi, cs_imu);
        });
    }

    #[task(binds = DMA1_CH2, shared = [spi1, cs_imu, imu_filters], priority = 2)]
    /// This ISR Handles received data from the IMU, after DMA transfer is complete. This occurs whenever
    /// we receive IMU data; it triggers the inner PID loop.
    fn imu_tc_isr(mut cx: imu_tc_isr::Context) {
        dma::clear_interrupt(
            DmaPeriph::Dma1,
            DmaChannel::C1,
            DmaInterrupt::TransferComplete,
        );

        (cx.shared.spi).lock(|dma, spi| {
            // Note that this step is mandatory, per STM32 RM.
            spi.stop_dma(DmaChannel::C1, Some(DmaChannel::C2), DmaPeriph::Dma1);
        });

        cx.shared.cs_imu.lock(|cs| {
            cs.set_high();
        });

        let mut imu_data = imu::ImuReadings::from_buffer(unsafe { &IMU_READINGS });

        // Apply our lowpass filter.
        cx.shared.imu_filters.lock(|imu_filters| {
            imu_filters.apply(&mut imu_data);
        });
    }

    #[task(binds=DMA1_STR1, shared = [spi_write_transfer, nss], priority=2)]
    fn tx_dma_complete(ctx: tx_dma_complete::Context) {
        // If desired, the transfer can scheduled again here to continue transmitting.
        (ctx.shared.spi_write_transfer, ctx.shared.nss).lock(|spi_write_transfer, cs| {
            spi_write_transfer.clear_transfer_complete_interrupt();
            spi_write_transfer.pause(|spi| {
                // At this point, the DMA transfer is done, but the data is still in the SPI output
                // FIFO. Wait for it to complete before disabling CS.
                while spi.inner().sr.read().txc().bit_is_clear() {}
                cs.set_high();

                //ここで読む
            });
        });
    }

    // #[task(binds = DMA1_STR2, shared = [spi_read_transfer], priority = 2)]
    // fn rx_dma_complete(ctx: rx_dma_complete::Context) {
    //     ctx.shared.spi_read_transfer.lock(|spi_read_transfer| {
    //         spi_read_transfer.clear_transfer_complete_interrupt();
    //     });
    // }

    #[task(binds = ETH, local = [net])]
    fn ethernet_event(ctx: ethernet_event::Context) {
        unsafe { ethernet::interrupt_handler() }

        let time = TIME.load(Ordering::Relaxed);
        ctx.local.net.poll(time as i64);
    }

    #[task(binds = SysTick, priority=15)]
    fn systick_tick(_: systick_tick::Context) {
        TIME.fetch_add(1, Ordering::Relaxed);
    }
}

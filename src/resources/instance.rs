use h7lib::*;
use periph::{pwr, rcc, gpio, adc, spi, timer, dma, ethernet};
use plugin::{pwm, ethernet_phy};

use ethernet::smoltcp;
use smoltcp::iface::{Config, Interface, SocketSet, SocketStorage};
use smoltcp::time::Instant;
use smoltcp::wire::{HardwareAddress, IpAddress, IpCidr};

pub struct Instance {
    spi2: spi::Spi<2>,
    spi2_nss: gpio::PA<11>,
    spi3: spi::Spi<3>,
    spi3_nss: gpio::PA<15>,
    dma1: dma::Dma<1>,
}

impl Instance {
    pub fn new(clock: &rcc::Rcc) -> Self {
        let spi2 = spi::Spi::<2>::init(
            gpio::PA::<12>::init(gpio::PinMode::AltFn(5, gpio::OutputType::PushPull), &clock), 
            gpio::PB::<14>::init(gpio::PinMode::AltFn(5, gpio::OutputType::PushPull), &clock), 
            gpio::PB::<15>::init(gpio::PinMode::AltFn(5, gpio::OutputType::PushPull), &clock),
            spi::SpiConfig {
                mode: spi::SpiMode::mode1(),
                data_size: spi::DataSize::D8,
                ..Default::default()
            },
        );
        let mut spi2_nss = gpio::PA::<11>::init(gpio::PinMode::Output(gpio::OutputType::PushPull), &clock);
        spi2_nss.set_high();
        let spi3 = spi::Spi::<3>::init(
            gpio::PC::<10>::init(gpio::PinMode::AltFn(6, gpio::OutputType::PushPull), &clock), 
            gpio::PC::<11>::init(gpio::PinMode::AltFn(6, gpio::OutputType::PushPull), &clock), 
            gpio::PC::<12>::init(gpio::PinMode::AltFn(6, gpio::OutputType::PushPull), &clock), 
            spi::SpiConfig {
                mode: spi::SpiMode::mode1(),
                data_size: spi::DataSize::D8,
                ..Default::default()
            }
        );
        let mut spi3_nss = gpio::PA::<15>::init(gpio::PinMode::Output(gpio::OutputType::PushPull), &clock);
        spi3_nss.set_high();

        let mut adc1 = adc::Adc::<1>::new(
            adc::Config {
                ..Default::default()
            },
            &clock,
            [
                adc::Channel::init(
                    adc::ChannelNum::C0,
                    gpio::PB::<1>::init(gpio::PinMode::Analog, &clock), 
                    adc::ChannelCfg {
                        ..Default::default()
                    }
                ),
            ]
        );
        
        let dma1 = dma::Dma::<1>::init();
        dma1.mux1(dma::DmaChannel::C1, dma::DmaInput::Spi2Tx);
        dma1.mux1(dma::DmaChannel::C2, dma::DmaInput::Spi2Rx);

        let tim1 = timer::Timer::<1>::init(timer::CountMode::Loop, &clock);
        let (ch1, ch2, ch3, ch4) = tim1.split(
            timer::ChannelOption {
                frequency: 10.kHz(),
                polarity: timer::Polarity::ActiveLow,
                alignment: Some(timer::Alignment::Center),
                deadtime: Some(1.micros()),
            }
        );
        let mut pwm1 = pwm::Pwm::<1, 1>::new_with_comp(
            ch1, 
            gpio::PE::<9>::init(gpio::PinMode::AltFn(1, gpio::OutputType::PushPull), &clock), 
            gpio::PE::<8>::init(gpio::PinMode::AltFn(1, gpio::OutputType::PushPull), &clock)
        );
        let mut pwm2 = pwm::Pwm::<1, 2>::new_with_comp(
            ch2, 
            gpio::PE::<11>::init(gpio::PinMode::AltFn(1, gpio::OutputType::PushPull), &clock), 
            gpio::PE::<10>::init(gpio::PinMode::AltFn(1, gpio::OutputType::PushPull), &clock)
        );
        let mut pwm3 = pwm::Pwm::<1, 3>::new_with_comp(
            ch3, 
            gpio::PE::<13>::init(gpio::PinMode::AltFn(1, gpio::OutputType::PushPull), &clock), 
            gpio::PE::<12>::init(gpio::PinMode::AltFn(1, gpio::OutputType::PushPull), &clock)
        );
        pwm1.set_duty(pwm1.get_max_duty() / 2);
        pwm2.set_duty(pwm2.get_max_duty() / 2);
        pwm3.set_duty(pwm3.get_max_duty() / 2);

        pwm1.enable();
        pwm2.enable();
        pwm3.enable();

        let mut tim2 = timer::Timer::<2>::init(timer::CountMode::Interrupt, &clock);
        tim2.start(10.Hz());
        tim2.listen();

        let mut led = gpio::PD::<0>::init(gpio::PinMode::Output(gpio::OutputType::PushPull), &clock);
        led.set_high();

        // Initialise ethernet...
        assert_eq!(clock.hclk.raw(), 200_000_000); // HCLK 200MHz
        assert_eq!(clock.pclk1.raw(), 100_000_000); // PCLK 100MHz
        assert_eq!(clock.pclk2.raw(), 100_000_000); // PCLK 100MHz
        assert_eq!(clock.pclk4.raw(), 100_000_000); // PCLK 100MHz

        let rmii_ref_clk = gpio::PA::<1>::init(gpio::PinMode::AltFn(11, gpio::OutputType::PushPull), &clock);
        let rmii_mdio = gpio::PA::<2>::init(gpio::PinMode::AltFn(11, gpio::OutputType::PushPull), &clock);
        let rmii_mdc = gpio::PC::<1>::init(gpio::PinMode::AltFn(11, gpio::OutputType::PushPull), &clock);
        let rmii_crs_dv = gpio::PA::<7>::init(gpio::PinMode::AltFn(11, gpio::OutputType::PushPull), &clock);
        let rmii_rxd0 = gpio::PC::<4>::init(gpio::PinMode::AltFn(11, gpio::OutputType::PushPull), &clock);
        let rmii_rxd1 = gpio::PC::<5>::init(gpio::PinMode::AltFn(11, gpio::OutputType::PushPull), &clock);
        let rmii_tx_en = gpio::PB::<11>::init(gpio::PinMode::AltFn(11, gpio::OutputType::PushPull), &clock);
        let rmii_txd0 = gpio::PB::<12>::init(gpio::PinMode::AltFn(11, gpio::OutputType::PushPull), &clock);
        let rmii_txd1 = gpio::PB::<13>::init(gpio::PinMode::AltFn(11, gpio::OutputType::PushPull), &clock);

        let mac_addr = smoltcp::wire::EthernetAddress::from_bytes(&MAC_ADDRESS);
        let (eth_dma, eth_mac) = unsafe {
            #[allow(static_mut_refs)] // TODO: Fix this
            ethernet_phy::net::DES_RING.write(ethernet::DesRing::new());

            ethernet::new(
                ctx.device.ETHERNET_MAC,
                ctx.device.ETHERNET_MTL,
                ctx.device.ETHERNET_DMA,
                #[allow(static_mut_refs)] // TODO: Fix this
                ethernet_phy::net::DES_RING.assume_init_mut(),
                mac_addr,
                &clock,
                rmii_ref_clk,
                rmii_mdio,
                rmii_mdc,
                rmii_crs_dv,
                rmii_rxd0,
                rmii_rxd1,
                rmii_tx_en,
                rmii_txd0,
                rmii_txd1,
            )
        };

        Instance {
            spi2,
            spi2_nss,
            spi3,
            spi3_nss,
            dma1,
        }

        // // Initialise ethernet PHY...
        // let eth_mac_custom = eth_mac.set_phy_addr(0x01);
        // let mut dp83848 = ethernet_phy::dp83848::DP83848::new(eth_mac_custom);
        // dp83848.phy_reset();
        // // for i in 0..5000000 {

        // // }
        // dp83848.phy_init();
        // // The eth_dma should not be used until the PHY reports the link is up

        // unsafe { ethernet::enable_interrupt() };

        // // unsafe: mutable reference to static storage, we only do this once
        // let store = unsafe {
        //     #[allow(static_mut_refs)] // TODO: Fix this
        //     let store_ptr = ethernet_phy::net::STORE.as_mut_ptr();

        //     // Initialise the socket_storage field. Using `write` instead of
        //     // assignment via `=` to not call `drop` on the old, uninitialised
        //     // value
        //     addr_of_mut!((*store_ptr).socket_storage)
        //         .write([SocketStorage::EMPTY; 8]);

        //     // Now that all fields are initialised we can safely use
        //     // assume_init_mut to return a mutable reference to STORE
        //     #[allow(static_mut_refs)] // TODO: Fix this
        //     ethernet_phy::net::STORE.assume_init_mut()
        // };

        // let net = ethernet_phy::net::Net::new(store, eth_dma, mac_addr.into(), Instant::ZERO);
    }
}
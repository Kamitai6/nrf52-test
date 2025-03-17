// fn transfer_spi<const NSS_PORT: char, const NSS_PIN: u8, 
    //     const SPI_N: u8
    // >(
    //     rw: bool, nss: &mut gpio::Gpio<NSS_PORT, NSS_PIN>, 
    //     spi: &mut spi::Spi<SPI_N>, spi_buffer: &mut [u8])
    // {
    //     nss.set_low();
    //     if rw {
    //         let result = spi.transfer(spi_buffer);
    //         match result {
    //             Ok(values) => {
    //                 for (i, &value) in values.iter().enumerate() {
    //                     rprintln!("Received data {}: {:#010b}", i, value);
    //                     // rprintln!("Received data {}: {:#018b}", i, value);
    //                 }
    //             }
    //             Err(e) => rprintln!("Error {:?}", e),
    //         }
    //     }
    //     else {
    //         let _ = spi.write(spi_buffer);
    //     }
        
    //     nss.set_high();
    // }

    // let mut spi_buffer: [u8; 2] = [0; 2];

    // spi_buffer = [0b10000000, 0b00000000]; //read fault
    // transfer_spi(true, &mut nss, &mut spi3, &mut spi_buffer);
    // delay_us(&clock, 10);

    // let rccregs = unsafe {&*pac::RCC::ptr()};
    // rprintln!("apb1henr {:#010x}", rccregs.apb1henr.read().bits());
    // rprintln!("apb1lenr {:#010x}", rccregs.apb1lenr.read().bits());
    // rprintln!("apb2enr {:#010x}", rccregs.apb2enr.read().bits());
    // let spiregs = unsafe {&*pac::SPI3::ptr()};
    // rprintln!("cr1: {:#010x}", spiregs.cr1.read().bits());
    // rprintln!("cr2: {:#010x}", spiregs.cr2.read().bits());
    // rprintln!("cfg1: {:#010x}", spiregs.cfg1.read().bits());
    // rprintln!("cfg2: {:#010x}", spiregs.cfg2.read().bits());
    // rprintln!("sr: {:#010x}", spiregs.sr.read().bits());

    // spi_buffer = [0b00001101, 0b01100000]; // write lock free
    // transfer_spi(false, &mut nss, &mut spi3, &mut spi_buffer);
    // delay_us(&clock, 10);
    // spi_buffer = [0b00000100, 0b10000000]; // write clear fault
    // transfer_spi(false, &mut nss, &mut spi3, &mut spi_buffer);
    // delay_us(&clock, 10);
    // spi_buffer = [0b00001110, 0b00000010]; // write ocp mode
    // transfer_spi(false, &mut nss, &mut spi3, &mut spi_buffer);
    // delay_us(&clock, 10);
    // spi_buffer = [0b10001110, 0b00000000]; //read ic11
    // transfer_spi(true, &mut nss, &mut spi3, &mut spi_buffer);
    // delay_us(&clock, 10);
    // spi_buffer = [0b10000000, 0b00000000]; //read fault
    // transfer_spi(true, &mut nss, &mut spi3, &mut spi_buffer);
    // delay_us(&clock, 10);
    // spi_buffer = [0b10000111, 0b00000000]; //read communication check
    // transfer_spi(true, &mut nss, &mut spi3, &mut spi_buffer);
    // delay_us(&clock, 10);
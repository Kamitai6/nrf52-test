pub struct MLX90363<SPI, SS> {
    spi: SPI,
    ss: SS,
    spi_read_buffer: [u8; 8],
    spi_write_buffer: [u8; 8],
    angle_degrees: f32,
    angle_lsb: u16,
    error_lsb: u8,
    roll_counter: u8,
    virtual_gain: u8,
    crc: u8,
}

impl<SPI, SS, E> MLX90363<SPI, SS>
where
    SPI: Transfer<u8, Error = E> + Write<u8, Error = E>,
    SS: OutputPin,
{
    pub fn new(spi: SPI, ss: SS) -> Self {
        Self {
            spi,
            ss,
            spi_read_buffer: [0; 8],
            spi_write_buffer: [0; 8],
            angle_degrees: 0.0,
            angle_lsb: 0,
            error_lsb: 0,
            roll_counter: 0,
            virtual_gain: 0,
            crc: 0,
        }
        // let mut spi_buf: [u8; 8] = [0x00, 0x00, 0xAA, 0xAA, 0x00, 0x00, 0xD0, 0xAB];
    // nss.set_low();
    // spi2.transfer(&mut spi_buf).ok();
    // let values = spi_buf;
    // for (i, &value) in values.iter().enumerate() {
    //     rprintln!("Received data 1 {}: {:#010x}", i, value);
    // }
    // nss.set_high();
    }

    fn angle_lsb(&mut self) -> u16 {
        self.angle_lsb =
            ((self.spi_read_buffer[1] & 0x3F) as u16) << 8 | (self.spi_read_buffer[0] as u16);
        self.angle_lsb
    }

    fn angle_degrees(&mut self) -> f32 {
        const LSB_TO_DEGREES: f32 = 0.02197;
        self.angle_lsb();
        self.angle_degrees = self.angle_lsb as f32 * LSB_TO_DEGREES;
        self.angle_degrees
    }

    fn error_bits(&self) -> u8 {
        self.spi_read_buffer[1] >> 6
    }

    fn rolling_counter(&self) -> u8 {
        self.spi_read_buffer[6] & 0x3F
    }

    fn virtual_gain(&self) -> u8 {
        self.spi_read_buffer[4]
    }

    fn crc(&self) -> u8 {
        self.spi_read_buffer[7]
    }

    pub fn read_data(&mut self) -> Result<(), E> {
        self.spi_write_buffer = [0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x13, 0xEA];

        self.ss.set_low().ok();
        self.spi.transfer(&mut self.spi_write_buffer)?;
        self.spi_read_buffer.copy_from_slice(&self.spi_write_buffer);
        self.ss.set_high().ok();

        self.angle_lsb = self.angle_lsb();
        self.angle_degrees = self.angle_degrees();
        self.error_lsb = self.error_bits();
        self.roll_counter = self.rolling_counter();
        self.virtual_gain = self.virtual_gain();
        self.crc = self.crc();

        Ok(())
    }
}

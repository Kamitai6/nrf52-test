const AS5048A_CLEAR_ERROR_FLAG: u16 = 0x0001;
const AS5048A_PROGRAMMING_CONTROL: u16 = 0x0003;
const AS5048A_OTP_REGISTER_ZERO_POS_HIGH: u16 = 0x0016;
const AS5048A_OTP_REGISTER_ZERO_POS_LOW: u16 = 0x0017;
const AS5048A_DIAG_AGC: u16 = 0x3FFD;
const AS5048A_MAGNITUDE: u16 = 0x3FFE;
const AS5048A_ANGLE: u16 = 0x3FFF;

const AS5048A_AGC_FLAG: u8 = 0xFF;
const AS5048A_ERROR_PARITY_FLAG: u8 = 0x04;
const AS5048A_ERROR_COMMAND_INVALID_FLAG: u8 = 0x02;
const AS5048A_ERROR_FRAMING_FLAG: u8 = 0x01;

const AS5048A_DIAG_COMP_HIGH: u16 = 0x2000;
const AS5048A_DIAG_COMP_LOW: u16 = 0x1000;
const AS5048A_DIAG_COF: u16 = 0x0800;
const AS5048A_DIAG_OCF: u16 = 0x0400;

const AS5048A_MAX_VALUE: f64 = 8191.0;

use embassy_stm32::spi::Spi;
// use embedded_hal::blocking::spi::Transfer;
// use core::convert::Infallible;
// use embedded_hal::digital::v2::OutputPin;
// use embedded_hal_async::digital::Wait;
// use embedded_hal_async::spi::SpiBus;
use heapless;
use rtt_target::{rprintln, rtt_init_print};

// pub struct AS5048<SPI, SS> {
//     spi: SPI,
//     ss: SS,
// }

// impl<SPI, SS, SPIE, E> AS5048<SPI, SS>
// where
//     SPI: SpiBus<u16, Error = SPIE>, // Transferトレイトを追加
//     SS: OutputPin<Error = E>,
// {
//     pub fn new(spi: SPI, ss: SS) -> Self {
//         Self { spi, ss }
//     }

//     pub async fn read_angle(&mut self) -> Result<u16, SPIE> {
//         let mut read_buffer = [0u16]; // ダミーデータ
//         let mut write_buffer = [AS5048A_ANGLE]; // 角度読み取りコマンド

//         // SPI経由でデータを送受信
//         self.ss.set_low(); // エラーハンドリング
//         rprintln!("Sending read command");
//         self.spi
//             .transfer(&mut read_buffer, &write_buffer)
//             .await
//             .map_err(|e| e)?;
//         rprintln!("Received angle");
//         self.ss.set_high();
//         rprintln!("Finished SPI communication");

//         // エンコーダのデータを返す
//         Ok(read_buffer[0])
//     }
// }
pub struct AS5048<'a> {
    spi: spi,
            delay_ms: Box::new(delay_func),
            position: 0,
}

impl<'a> AS5048<'a> {
    pub fn new<F>(spi: &'a stm32::spi::SPI, delay_func: F) -> AS5048<'a>
    where
        F: 'static + FnMut(u32),
    {
        AS5048 {
            spi: spi,
            delay_ms: Box::new(delay_func),
            position: 0,
        }
    }

    fn spiCalcEvenParity(&self, value: u16) -> u8 {
        let mut cnt = 0;
        let mut val = value;

        for i in 0..16 {
            if (val & 0x1) != 0 {
                cnt += 1;
            }
            val = val >> 1;
        }
        cnt & 0x1
    }

    pub fn getRotation(&mut self) -> i16 {
        let data = self.getRawRotation();
        let mut rotation = data as i16 - self.position as i16;
        if rotation > AS5048A_MAX_VALUE as i16 {
            rotation = -((0x3FFF) - rotation); //more than -180
        }
        rotation
    }

    pub fn getRawRotation(&mut self) -> u32 {
        self.read(AS5048A_ANGLE) as u32
    }

    fn getRotationInDegrees(&mut self) -> f64 {
        let rotation = self.getRotation();
        let degrees = 360.0 * (rotation as f64 + AS5048A_MAX_VALUE) / (AS5048A_MAX_VALUE * 2.0);
        degrees
    }

    fn getRotationInRadians(&mut self) -> f64 {
        let rotation = self.getRotation();
        let radians = 3.14 * (rotation as f64 + AS5048A_MAX_VALUE) / AS5048A_MAX_VALUE;
        radians
    }

    pub fn getState(&mut self) -> u32 {
        self.read(AS5048A_DIAG_AGC) as u32
    }

    pub fn getGain(&mut self) -> u8 {
        let data = self.getState();
        data as u8 & AS5048A_AGC_FLAG
    }

    pub fn getDiagnostic(&mut self) -> String {
        let mut data = self.getState() as u16;
        if (data & AS5048A_DIAG_COMP_HIGH) != 0 {
            return "COMP high".to_string();
        }
        if (data & AS5048A_DIAG_COMP_LOW) != 0 {
            return "COMP low".to_string();
        }
        if (data & AS5048A_DIAG_COF) != 0 {
            return "CORDIC overflow".to_string();
        }
        return "".to_string();
    }

    pub fn getErrors(&mut self) -> String {
        let error: u8 = self.read(AS5048A_CLEAR_ERROR_FLAG) as u8;
        if (error & AS5048A_ERROR_PARITY_FLAG) != 0 {
            return "Parity Error".to_string();
        }
        if (error & AS5048A_ERROR_COMMAND_INVALID_FLAG) != 0 {
            return "Command invalid".to_string();
        }
        if (error & AS5048A_ERROR_FRAMING_FLAG) != 0 {
            return "Framing error".to_string();
        }
        return "".to_string();
    }

    pub fn setZeroPosition(&mut self, position: i16) {
        self.position = position % 0x3FFF;
    }

    fn read(&mut self, register_address: u16) -> u16 {
        let mut command = 0x4000; // PAR=0 R/W=R
        command = command | register_address;

        //Add a parity bit on the the MSB
        command |= (self.spiCalcEvenParity(command) as u16) << 0xF;

        self.spi.spi3_begin();
        let response = self.spi.spi3_send(command as u32);
        self.spi.spi3_end();

        (self.delay_ms)(1_u32);

        self.spi.spi3_begin();
        let response = self.spi.spi3_send(0x0000);
        self.spi.spi3_end();

        //Check if the error bit is set
        if (response & 0x4000) != 0 {
            // error
        }

        //Return the data, stripping the parity and error bits
        response as u16 & !0xC000
    }

    fn write(&mut self, register_address: u16, data: u16) -> u16 {
        let mut command = 0x0000; // PAR=0 R/W=W
        command |= register_address;

        //Add a parity bit on the the MSB
        command |= (self.spiCalcEvenParity(command) as u16) << 0xF;

        self.spi.spi3_begin();
        self.spi.spi3_send(command as u32);
        self.spi.spi3_end();

        let mut data_to_send = 0x0000;
        data_to_send |= data;

        //Craft another packet including the data and parity
        data_to_send |= (self.spiCalcEvenParity(data_to_send) as u16) << 0xF;

        self.spi.spi3_begin();
        self.spi.spi3_send(data_to_send as u32);
        self.spi.spi3_end();

        (self.delay_ms)(50_u32);

        self.spi.spi3_begin();
        let response = self.spi.spi3_send(0x0000);
        self.spi.spi3_end();

        //Return the data, stripping the parity and error bits
        response as u16 & !0xC000
    }
}

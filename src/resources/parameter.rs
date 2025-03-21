// default configuration values
// change these values to optimal ones for your application

const DEF_POWER_SUPPLY: f32 = 12.0; // default power supply voltage

// velocity PI controller params
const DEF_PID_VEL_P: f32 = 0.5; // default PID controller P value
const DEF_PID_VEL_I: f32 = 10.0; // default PID controller I value
const DEF_PID_VEL_D: f32 = 0.0; // default PID controller D value
const DEF_PID_VEL_RAMP: f32 = 1000.0; // default PID controller voltage ramp value
const DEF_PID_VEL_LIMIT: f32 = DEF_POWER_SUPPLY; // default PID controller voltage limit

// default current limit values
const DEF_CURRENT_LIM: f32 = 2.0; // 2Amps current limit by default

// default monitor downsample
const DEF_MON_DOWNSAMPLE: u32 = 100; // default monitor downsample
const DEF_MOTION_DOWNSAMPLE: u32 = 0; // default motion downsample - disable

// angle P params
const DEF_P_ANGLE_P: f32 = 20.0; // default P controller P value
const DEF_VEL_LIM: f32 = 20.0; // angle velocity limit default

// index search
const DEF_INDEX_SEARCH_TARGET_VELOCITY: f32 = 1.0; // default index search velocity
                                                   // align voltage
const DEF_VOLTAGE_SENSOR_ALIGN: f32 = 3.0; // default voltage for sensor and motor zero alignment
                                           // low pass filter velocity
const DEF_VEL_FILTER_Tf: f32 = 0.005; // default velocity filter time constant

// current sense default parameters
const DEF_LPF_PER_PHASE_CURRENT_SENSE_Tf: f32 = 0.0; // default current sense per phase low pass filter time constant

pub const MAC_ADDRESS: [u8; 6] = [0x02, 0x00, 0x11, 0x22, 0x33, 0x44];

#[link_section = ".sram3"]
pub static mut SPI2_READ_BUF: [u8; 8] = [0; 8];
#[link_section = ".sram3"]
pub static mut SPI2_WRITE_BUF: [u8; 8] = [0; 8];
#[link_section = ".sram3"]
pub static mut SPI3_READ_BUF: [u8; 8] = [0; 8];
#[link_section = ".sram3"]
pub static mut SPI3_WRITE_BUF: [u8; 8] = [0; 8];
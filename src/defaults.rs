// default configuration values
// change these values to optimal ones for your application

const DEF_POWER_SUPPLY: f32 = 12.0; // default power supply voltage

// velocity PI controller params
const DEF_PID_VEL_P: f32 = 0.5; // default PID controller P value
const DEF_PID_VEL_I: f32 = 10.0; // default PID controller I value
const DEF_PID_VEL_D: f32 = 0.0; // default PID controller D value
const DEF_PID_VEL_RAMP: f32 = 1000.0; // default PID controller voltage ramp value
const DEF_PID_VEL_LIMIT: f32 = DEF_POWER_SUPPLY; // default PID controller voltage limit

// current sensing PID values
#[cfg(any(
    target_arch = "avr_atmega328p",
    target_arch = "avr_atmega168",
    target_arch = "avr_atmega328pb",
    target_arch = "avr_atmega2560"
))]
mod avr_pid {
    pub const DEF_PID_CURR_P: f32 = 2.0; // default PID controller P value
    pub const DEF_PID_CURR_I: f32 = 100.0; // default PID controller I value
    pub const DEF_PID_CURR_D: f32 = 0.0; // default PID controller D value
    pub const DEF_PID_CURR_RAMP: f32 = 1000.0; // default PID controller voltage ramp value
    pub const DEF_PID_CURR_LIMIT: f32 = super::DEF_POWER_SUPPLY; // default PID controller voltage limit
    pub const DEF_CURR_FILTER_Tf: f32 = 0.01; // default velocity filter time constant
}

#[cfg(not(any(
    target_arch = "avr_atmega328p",
    target_arch = "avr_atmega168",
    target_arch = "avr_atmega328pb",
    target_arch = "avr_atmega2560"
)))]
mod non_avr_pid {
    pub const DEF_PID_CURR_P: f32 = 3.0; // default PID controller P value
    pub const DEF_PID_CURR_I: f32 = 300.0; // default PID controller I value
    pub const DEF_PID_CURR_D: f32 = 0.0; // default PID controller D value
    pub const DEF_PID_CURR_RAMP: f32 = 0.0; // default PID controller voltage ramp value
    pub const DEF_PID_CURR_LIMIT: f32 = super::DEF_POWER_SUPPLY; // default PID controller voltage limit
    pub const DEF_CURR_FILTER_Tf: f32 = 0.005; // default current filter time constant
}

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

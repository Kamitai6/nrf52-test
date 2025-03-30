#![no_std]
#![allow(unused_unsafe)]

pub mod math_const {
    pub const TWO_DIV_SQRT3: f32 = 1.15470053838;
    pub const SQRT3: f32 = 1.73205080757;
    pub const ONE_DIV_SQRT3: f32 = 0.57735026919;
    pub const SQRT3_DIV_2: f32 = 0.86602540378;
    pub const SQRT2: f32 = 1.41421356237;
    pub const DEG120_TO_RAD: f32 = 2.09439510239;
    pub const PI: f32 = 3.14159265359;
    pub const PI_DIV_2: f32 = 1.57079632679;
    pub const PI_DIV_3: f32 = 1.0471975512;
    pub const TWO_PI: f32 = 6.28318530718;
    pub const THREE_PI_DIV_2: f32 = 4.71238898038;
    pub const PI_DIV_6: f32 = 0.52359877559;
    pub const RPM_TO_RADS: f32 = 0.10471975512;
}

pub mod foc;
pub mod motor;

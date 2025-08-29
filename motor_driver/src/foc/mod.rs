pub mod pid;
pub mod smc;
use super::motor;

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

//Field Oriented Controller
pub struct Foc {
    sens_gain: f32,
    current_offset: [f32; 3],
    sensor_offset: f32,
    theta_electrical: f32,    // 電気角
    alpha_beta_voltage: (f32, f32),
    d_q_voltage: (f32, f32),
    d_q_current: (f32, f32),

    target_d_current: f32,
    target_q_current: f32,
    target_velocity: f32,
    target_flux: f32,

    max_d_current: f32,
    max_q_current: f32,
    max_velocity: f32,
    max_flux: f32,

    motor: motor::PMSM,
    d_current_pid: pid::PID,
    q_current_pid: pid::PID,
    velocity_pid: pid::PID,
    flux_pid: pid::PID,
}

impl Foc {
    fn new(shunt_resistor: f32, gain: f32) -> Self {
        Self {
            sens_gain: 1.0 / shunt_resistor / gain,
            current_offset: [0.0; 3],
        }
    }

    /// 外部から目標速度を設定
    pub fn set_target_velocity(&mut self, velocity: f32) {
        self.target_velocity = velocity.clamp(-self.max_velocity, self.max_velocity);
    }

    /// 外部から目標磁束を設定
    pub fn set_target_flux(&mut self, flux: f32) {
        self.target_flux = flux.clamp(-self.max_flux, self.max_flux);
    }

    pub fn get_velocity() {
        let velocity = getVelocity();
        if velocity > limit return Err();
    }

    pub fn get_current() {
        let current = getCurrent();
        let (d_current, q_current) = convert_foc_current(current);
        if q_current > limit return Err();
    }

    /// 速度ループ - 低頻度（例：1kHz）で呼び出し
    /// velocity_loopとflux_loopは同じタイマー割り込みで実行
    pub fn update_velocity_loop() {
        let velocity = getVelocity();
        let (target_torque, target_d) = smc.compute(velocity, target_velocity);
    }

    /// 磁束ループ - 低頻度（例：1kHz）で呼び出し
    pub fn update_flux_loop(&mut self, measured_flux: f32) -> () {
        // 磁束PIDコントローラーを更新（出力はd軸電流の目標値）
        self.target_d_current = self.flux_pid.update(
            self.target_flux, 
            measured_flux
        );
        
        // d軸電流の上限を適用
        self.target_d_current = self.target_d_current.clamp(
            0.0, // d軸電流は通常正
            self.config.max_current
        );
    }
    
    /// 電流ループ - 高頻度（例：10kHz）で呼び出し
    pub fn update_current_loop() {
        let (d_current, q_current) = getCurrent();
        let ud = pid.compute(d_current, d_target); // or 0
        let uq = pid.compute(q_current, q_target);
        let pwm = convert_foc_pwm(ud, uq);
        setPwm(pwm);
    }

    fn convert_foc_current(&self, adc_volt_a: f32, adc_volt_b: f32, adc_volt_c: Option<f32>) -> (f32, f32) {
        // read current phase currents
        let phase_a = (adc_volt_a - self.offset[0]) * self.gain;
        let phase_b = (adc_volt_b - self.offset[1]) * self.gain;
        let phase_c = adc_volt_c.map(|volt| (volt - self.offset[2]) * self.gain);

        // calculate clarke transform
        let (alpha, beta) = {
            if let Some(some_phase_c) = phase_c {
                let mid = (phase_a + phase_b + some_phase_c) / 3.;
                let a = phase_a - mid;
                let b = phase_b - mid;
                (
                    a,
                    _1_SQRT3 * a + _2_SQRT3 * b
                )
            } else {
                (
                    phase_a,
                    _1_SQRT3 * phase_a + _2_SQRT3 * phase_b,
                )
            }
        };

        // calculate park transform
        (
            current.alpha * ct + current.beta * st,
            current.beta * ct - current.alpha * st
        )
    }

    // 逆パーク変換
    pub fn convert_foc_pwm(&mut self, d: f32, q: f32) -> (f32, f32, f32) {
        let voltage_limit = self.voltage_limit.unwrap_or(self.voltage_power_supply);
        let ua = ua.clamp(0.0, voltage_limit);
        let ub = ub.clamp(0.0, voltage_limit);
        let uc = uc.clamp(0.0, voltage_limit);

        self.dc_a = (ua / self.voltage_power_supply).clamp(0.0, 1.0);
        self.dc_b = (ub / self.voltage_power_supply).clamp(0.0, 1.0);
        self.dc_c = (uc / self.voltage_power_supply).clamp(0.0, 1.0);

        write_duty_cycle_6pwm(self.dc_a, self.dc_b, self.dc_c, &self.phase_state);
        // Mock function
         let (_sa, _ca) = self._sincos(angle_el);

        let Ualpha = _ca * Ud - _sa * Uq;
        let Ubeta = _sa * Ud + _ca * Uq;

        self.Ua = Ualpha;
        self.Ub = -0.5 * Ualpha + _SQRT3_2 * Ubeta;
        self.Uc = -0.5 * Ualpha - _SQRT3_2 * Ubeta;

        center = self.driver.voltage_limit / 2.0;
        if self.foc_modulation == FOCModulationType::SpaceVectorPWM {
            let Umin = self.Ua.min(self.Ub.min(self.Uc));
            let Umax = self.Ua.max(self.Ub.max(self.Uc));
            center -= (Umax + Umin) / 2.0;
        }

        if !self.modulation_centered {
            let Umin = self.Ua.min(self.Ub.min(self.Uc));
            self.Ua -= Umin;
            self.Ub -= Umin;
            self.Uc -= Umin;
        } else {
            self.Ua += center;
            self.Ub += center;
            self.Uc += center;
        }

        fn inverse_transform_park(d_q: (f32, f32), theta: f32) -> (f32, f32) {
            // 逆Park変換の実装
            let (d, q) = d_q;
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();
            
            let alpha = d * cos_theta - q * sin_theta;
            let beta = d * sin_theta + q * cos_theta;
            
            (alpha, beta)
        }
        
        fn svpwm(alpha_beta: (f32, f32)) -> [f32; 3] {
            // SVPWMアルゴリズムの実装
            // 略（α-β電圧から最適な3相PWM値を計算）
            [0.5, 0.5, 0.5] // 仮の戻り値
        }
    }
}
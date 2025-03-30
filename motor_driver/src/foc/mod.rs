//Field Oriented Controller
pub struct Foc {
    resistor: f32,
    gain: f32,
    current_offset: [f32; 3],
    sensor_offset: f32,
    // 内部状態
    theta_electrical: f32,    // 電気角
    alpha_beta_voltage: (f32, f32),
    d_q_voltage: (f32, f32),
    d_q_current: (f32, f32),

    d_current_pid: PidController,
    q_current_pid: PidController,
    velocity_pid: PidController,
    flux_pid: PidController,
}

impl Foc {
    fn new(shunt_resistor: f32, gain: f32) -> Self {
        Self {
            resistor: shunt_resistor,
            gain: 1.0 / shunt_resistor / gain,
            offset: [0.0; 3],
        }
    }

    /// 外部から目標速度を設定
    pub fn set_target_velocity(&mut self, velocity: f32) {
        self.target_velocity = velocity.clamp(-self.config.max_velocity, self.config.max_velocity);
    }

    /// 外部から目標磁束を設定
    pub fn set_target_flux(&mut self, flux: f32) {
        self.target_flux = flux.clamp(0.0, self.config.max_flux);
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

    // fn calibrate_offset() {
    //     const int calibration_rounds = 2000;

    //     // find adc offset = zero current voltage
    //     offset_ia = 0;
    //     offset_ib = 0;
    //     offset_ic = 0;
    //     // read the adc voltage 1000 times ( arbitrary number )
    //     for (int i = 0; i < calibration_rounds; i++) {
    //         _startADC3PinConversionLowSide();
    //         if(_isset(pinA)) offset_ia += (_readADCVoltageLowSide(pinA, params));
    //         if(_isset(pinB)) offset_ib += (_readADCVoltageLowSide(pinB, params));
    //         if(_isset(pinC)) offset_ic += (_readADCVoltageLowSide(pinC, params));
    //         _delay(1);
    //     }
    //     // calculate the mean offsets
    //     if(_isset(pinA)) offset_ia = offset_ia / calibration_rounds;
    //     if(_isset(pinB)) offset_ib = offset_ib / calibration_rounds;
    //     if(_isset(pinC)) offset_ic = offset_ic / calibration_rounds;
    // }
    pub fn calibration(&mut self) -> Result<_, ()> {
        // let mut exit_flag = 1; // success
        // println!("MOT: Align sensor.");

        // // check if sensor needs zero search
        // if self.sensor.as_ref().unwrap().needs_search() {
        //     exit_flag = self.absolute_zero_search();
        // }
        // // stop init if not found index
        // if exit_flag == 0 {
        //     return exit_flag;
        // }

        // // v2.3.3 fix for R_AVR_7_PCREL against symbol" bug for AVR boards
        // // TODO figure out why this works
        // let voltage_align = self.voltage_sensor_align;

        // // if unknown natural direction
        // if self.sensor_direction == Direction::Unknown {
        //     // find natural direction
        //     // move one electrical revolution forward
        //     for i in 0..=500 {
        //         let angle = 3.0 * PI / 2.0 + 2.0 * PI * i as f32 / 500.0;
        //         self.set_phase_voltage(voltage_align, 0.0, angle);
        //         self.sensor.as_mut().unwrap().update();
        //         std::thread::sleep(std::time::Duration::from_millis(2));
        //     }
        //     // take and angle in the middle
        //     self.sensor.as_mut().unwrap().update();
        //     let mid_angle = self.sensor.as_ref().unwrap().get_angle();
        //     // move one electrical revolution backwards
        //     for i in (0..=500).rev() {
        //         let angle = 3.0 * PI / 2.0 + 2.0 * PI * i as f32 / 500.0;
        //         self.set_phase_voltage(voltage_align, 0.0, angle);
        //         self.sensor.as_mut().unwrap().update();
        //         std::thread::sleep(std::time::Duration::from_millis(2));
        //     }
        //     self.sensor.as_mut().unwrap().update();
        //     let end_angle = self.sensor.as_ref().unwrap().get_angle();
        //     std::thread::sleep(std::time::Duration::from_millis(200));
        //     // determine the direction the sensor moved
        //     let moved = (mid_angle - end_angle).abs();
        //     if moved < MIN_ANGLE_DETECT_MOVEMENT {
        //         // minimum angle to detect movement
        //         println!("MOT: Failed to notice movement");
        //         return 0; // failed calibration
        //     } else if mid_angle < end_angle {
        //         println!("MOT: sensor_direction==CCW");
        //         self.sensor_direction = Direction::CCW;
        //     } else {
        //         println!("MOT: sensor_direction==CW");
        //         self.sensor_direction = Direction::CW;
        //     }
        //     // check pole pair number
        //     let pp_check_result = (moved * self.pole_pairs as f32 - 2.0 * PI).abs() <= 0.5;
        //     if !pp_check_result {
        //         println!("MOT: PP check: fail - estimated pp: {}", 2.0 * PI / moved);
        //     } else {
        //         println!("MOT: PP check: OK!");
        //     }
        // } else {
        //     println!("MOT: Skip dir calib.");
        // }

        // // zero electric angle not known
        // if !self.zero_electric_angle.is_some() {
        //     // align the electrical phases of the motor and sensor
        //     // set angle -90(270 = 3PI/2) degrees
        //     self.set_phase_voltage(voltage_align, 0.0, 3.0 * PI / 2.0);
        //     std::thread::sleep(std::time::Duration::from_millis(700));
        //     // read the sensor
        //     self.sensor.as_mut().unwrap().update();
        //     // get the current zero electric angle
        //     self.zero_electric_angle = self.electrical_angle();
        //     if self.monitor_port.is_some() {
        //         println!("MOT: Zero elec. angle: {}", self.zero_electric_angle);
        //     }
        //     // stop everything
        //     self.set_phase_voltage(0.0, 0.0, 0.0);
        //     std::thread::sleep(std::time::Duration::from_millis(200));
        // } else {
        //     println!("MOT: Skip offset calib.");
        // }
        // exit_flag
        Ok(())
    }
}
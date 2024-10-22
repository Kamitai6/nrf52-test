pub struct BLDCMotor {
    // mechanic parameter
    pole_pairs: i32,
    phase_resistance: f32,
    kv_rating: Option<f32>,
    phase_inductance: f32,

    // control parameter
    torque_controller: TorqueControlType,
    driver: Option<Box<BLDCDriver>>,
    velocity_limit: f32,
    voltage_limit: f32,
    current_limit: f32,
    velocity_index_search: f32,
    voltage_sensor_align: f32,
    foc_modulation: FOCModulationType,
    sensor_offset: f32,
    sensor: Option<Sensor>,
    current_sense: Option<CurrentSense>,
    enabled: bool,

    // control variables
    target: f32,
    voltage: DQVoltage_s,
    current_sp: f32,
    current: DQCurrent_s,
    voltage_bemf: f32,
    Ualpha: f32,
    Ubeta: f32,
    shaft_angle: f32,
    shaft_velocity: f32,
    electrical_angle: f32,
    zero_electric_angle: f32,
    shaft_angle_sp: f32,
    shaft_velocity_sp: f32,
    open_loop_timestamp: u64,
}

impl BLDCMotor {
    pub fn new(
        pole_pairs: i32,
        phase_resistance: f32,
        kv: Option<f32>,
        phase_inductance: f32,
    ) -> Self {
        BLDCMotor {
            pole_pairs,
            phase_resistance,
            kv_rating: kv,
            phase_inductance,
            torque_controller: TorqueControlType::Voltage, // default to voltage
            driver: None,
            velocity_limit: 0.0,
            voltage_limit: 0.0,
            current_limit: 0.0,
            velocity_index_search: 0.0,
            voltage_sensor_align: 0.0,
            foc_modulation: FOCModulationType::SinePWM,
            target: 0.0,
            voltage: DQVoltage_s { d: 0.0, q: 0.0 },
            current_sp: 0.0,
            current: DQCurrent_s { d: 0.0, q: 0.0 },
            voltage_bemf: 0.0,
            Ualpha: 0.0,
            Ubeta: 0.0,
            monitor_port: None,
            sensor_offset: 0.0,
            sensor: None,
            current_sense: None,
            enabled: false,
            shaft_angle: 0.0,
            shaft_velocity: 0.0,
            electrical_angle: 0.0,
            zero_electric_angle: 0.0,
            motion_cnt: 0,
            motion_downsample: 1,
            shaft_angle_sp: 0.0,
            shaft_velocity_sp: 0.0,
            open_loop_timestamp: 0,
        }
    }

    pub fn init(&mut self) -> i32 {
        if self.driver.is_none() || !self.driver.as_ref().unwrap().initialized {
            self.motor_status = FOCMotorStatus::MotorInitFailed;
            println!("MOT: Init not possible, driver not initialized");
            return 0;
        }
        self.motor_status = FOCMotorStatus::MotorInitializing;
        println!("MOT: Init");

        // sanity check for the voltage limit configuration
        if self.voltage_limit > self.driver.as_ref().unwrap().voltage_limit {
            self.voltage_limit = self.driver.as_ref().unwrap().voltage_limit;
        }
        // constrain voltage for sensor alignment
        if self.voltage_sensor_align > self.voltage_limit {
            self.voltage_sensor_align = self.voltage_limit;
        }

        // update the controller limits
        if let Some(current_sense) = &self.current_sense {
            // current control loop controls voltage
            self.PID_current_q.limit = self.voltage_limit;
            self.PID_current_d.limit = self.voltage_limit;
        }
        if self.phase_resistance.is_some() || self.torque_controller != TorqueControlType::Voltage {
            // velocity control loop controls current
            self.PID_velocity.limit = self.current_limit;
        } else {
            // velocity control loop controls the voltage
            self.PID_velocity.limit = self.voltage_limit;
        }
        self.P_angle.limit = self.velocity_limit;

        // if using open loop control, set a CW as the default direction if not already set
        if (self.controller == MotionControlType::AngleOpenloop
            || self.controller == MotionControlType::VelocityOpenloop)
            && self.sensor_direction == Direction::Unknown
        {
            self.sensor_direction = Direction::CW;
        }

        std::thread::sleep(std::time::Duration::from_millis(500));
        // enable motor
        println!("MOT: Enable driver.");
        self.enable();
        std::thread::sleep(std::time::Duration::from_millis(500));
        self.motor_status = FOCMotorStatus::MotorUncalibrated;
        1
    }

    pub fn disable(&mut self) {
        // disable the current sense
        if let Some(current_sense) = &self.current_sense {
            current_sense.disable();
        }
        // set zero to PWM
        self.driver.as_mut().unwrap().set_pwm(0.0, 0.0, 0.0);
        // disable the driver
        self.driver.as_mut().unwrap().disable();
        // motor status update
        self.enabled = false;
    }

    pub fn enable(&mut self) {
        // enable the driver
        self.driver.as_mut().unwrap().enable();
        // set zero to PWM
        self.driver.as_mut().unwrap().set_pwm(0.0, 0.0, 0.0);
        // enable the current sense
        if let Some(current_sense) = &self.current_sense {
            current_sense.enable();
        }
        // reset the pids
        self.PID_velocity.reset();
        self.P_angle.reset();
        self.PID_current_q.reset();
        self.PID_current_d.reset();
        // motor status update
        self.enabled = true;
    }

    pub fn shaft_angle(&self) -> f32 {
        // if no sensor linked return previous value ( for open loop )
        if self.sensor.is_none() {
            return self.shaft_angle;
        }
        self.sensor_direction * self.LPF_angle(self.sensor.as_ref().unwrap().get_angle())
            - self.sensor_offset
    }

    pub fn shaft_velocity(&self) -> f32 {
        // if no sensor linked return previous value ( for open loop )
        if self.sensor.is_none() {
            return self.shaft_velocity;
        }
        self.sensor_direction * self.LPF_velocity(self.sensor.as_ref().unwrap().get_velocity())
    }

    pub fn electrical_angle(&self) -> f32 {
        // if no sensor linked return previous value ( for open loop )
        if self.sensor.is_none() {
            return self.electrical_angle;
        }
        self.normalize_angle(
            (self.sensor_direction * self.pole_pairs as f32)
                * self.sensor.as_ref().unwrap().get_mechanical_angle()
                - self.zero_electric_angle,
        )
    }

    pub fn init_foc(&mut self) -> i32 {
        let mut exit_flag = 1;

        self.motor_status = FOCMotorStatus::MotorCalibrating;

        // align motor if necessary
        // alignment necessary for encoders!
        // sensor and motor alignment - can be skipped
        // by setting motor.sensor_direction and motor.zero_electric_angle
        if let Some(sensor) = &self.sensor {
            exit_flag *= self.align_sensor();
            // added the shaft_angle update
            sensor.update();
            self.shaft_angle = self.shaft_angle();

            // aligning the current sensor - can be skipped
            // checks if driver phases are the same as current sense phases
            // and checks the direction of measurement.
            if exit_flag != 0 {
                if let Some(current_sense) = &self.current_sense {
                    if !current_sense.initialized {
                        self.motor_status = FOCMotorStatus::MotorCalibFailed;
                        println!("MOT: Init FOC error, current sense not initialized");
                        exit_flag = 0;
                    } else {
                        exit_flag *= self.align_current_sense();
                    }
                } else {
                    println!("MOT: No current sense.");
                }
            }
        } else {
            println!("MOT: No sensor.");
            if self.controller == MotionControlType::AngleOpenloop
                || self.controller == MotionControlType::VelocityOpenloop
            {
                exit_flag = 1;
                println!("MOT: Openloop only!");
            } else {
                exit_flag = 0; // no FOC without sensor
            }
        }

        if exit_flag != 0 {
            println!("MOT: Ready.");
            self.motor_status = FOCMotorStatus::MotorReady;
        } else {
            println!("MOT: Init FOC failed.");
            self.motor_status = FOCMotorStatus::MotorCalibFailed;
            self.disable();
        }

        exit_flag
    }

    pub fn align_current_sense(&mut self) -> i32 {
        let mut exit_flag = 1; // success

        println!("MOT: Align current sense.");

        // align current sense and the driver
        exit_flag = self
            .current_sense
            .as_mut()
            .unwrap()
            .driver_align(self.voltage_sensor_align, self.modulation_centered);
        if exit_flag == 0 {
            // error in current sense - phase either not measured or bad connection
            println!("MOT: Align error!");
            exit_flag = 0;
        } else {
            // output the alignment status flag
            println!("MOT: Success: {}", exit_flag);
        }

        exit_flag > 0
    }

    pub fn align_sensor(&mut self) -> i32 {
        let mut exit_flag = 1; // success
        println!("MOT: Align sensor.");

        // check if sensor needs zero search
        if self.sensor.as_ref().unwrap().needs_search() {
            exit_flag = self.absolute_zero_search();
        }
        // stop init if not found index
        if exit_flag == 0 {
            return exit_flag;
        }

        // v2.3.3 fix for R_AVR_7_PCREL against symbol" bug for AVR boards
        // TODO figure out why this works
        let voltage_align = self.voltage_sensor_align;

        // if unknown natural direction
        if self.sensor_direction == Direction::Unknown {
            // find natural direction
            // move one electrical revolution forward
            for i in 0..=500 {
                let angle = 3.0 * PI / 2.0 + 2.0 * PI * i as f32 / 500.0;
                self.set_phase_voltage(voltage_align, 0.0, angle);
                self.sensor.as_mut().unwrap().update();
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            // take and angle in the middle
            self.sensor.as_mut().unwrap().update();
            let mid_angle = self.sensor.as_ref().unwrap().get_angle();
            // move one electrical revolution backwards
            for i in (0..=500).rev() {
                let angle = 3.0 * PI / 2.0 + 2.0 * PI * i as f32 / 500.0;
                self.set_phase_voltage(voltage_align, 0.0, angle);
                self.sensor.as_mut().unwrap().update();
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            self.sensor.as_mut().unwrap().update();
            let end_angle = self.sensor.as_ref().unwrap().get_angle();
            std::thread::sleep(std::time::Duration::from_millis(200));
            // determine the direction the sensor moved
            let moved = (mid_angle - end_angle).abs();
            if moved < MIN_ANGLE_DETECT_MOVEMENT {
                // minimum angle to detect movement
                println!("MOT: Failed to notice movement");
                return 0; // failed calibration
            } else if mid_angle < end_angle {
                println!("MOT: sensor_direction==CCW");
                self.sensor_direction = Direction::CCW;
            } else {
                println!("MOT: sensor_direction==CW");
                self.sensor_direction = Direction::CW;
            }
            // check pole pair number
            let pp_check_result = (moved * self.pole_pairs as f32 - 2.0 * PI).abs() <= 0.5;
            if !pp_check_result {
                println!("MOT: PP check: fail - estimated pp: {}", 2.0 * PI / moved);
            } else {
                println!("MOT: PP check: OK!");
            }
        } else {
            println!("MOT: Skip dir calib.");
        }

        // zero electric angle not known
        if !self.zero_electric_angle.is_some() {
            // align the electrical phases of the motor and sensor
            // set angle -90(270 = 3PI/2) degrees
            self.set_phase_voltage(voltage_align, 0.0, 3.0 * PI / 2.0);
            std::thread::sleep(std::time::Duration::from_millis(700));
            // read the sensor
            self.sensor.as_mut().unwrap().update();
            // get the current zero electric angle
            self.zero_electric_angle = self.electrical_angle();
            if self.monitor_port.is_some() {
                println!("MOT: Zero elec. angle: {}", self.zero_electric_angle);
            }
            // stop everything
            self.set_phase_voltage(0.0, 0.0, 0.0);
            std::thread::sleep(std::time::Duration::from_millis(200));
        } else {
            println!("MOT: Skip offset calib.");
        }
        exit_flag
    }

    pub fn absolute_zero_search(&mut self) -> i32 {
        println!("MOT: Index search...");
        // search the absolute zero with small velocity
        let limit_vel = self.velocity_limit;
        let limit_volt = self.voltage_limit;
        self.velocity_limit = self.velocity_index_search;
        self.voltage_limit = self.voltage_sensor_align;
        self.shaft_angle = 0.0;
        while self.sensor.as_ref().unwrap().needs_search() && self.shaft_angle < 2.0 * PI {
            self.angle_openloop(1.5 * 2.0 * PI);
            // call important for some sensors not to lose count
            // not needed for the search
            self.sensor.as_mut().unwrap().update();
        }
        // disable motor
        self.set_phase_voltage(0.0, 0.0, 0.0);
        // reinit the limits
        self.velocity_limit = limit_vel;
        self.voltage_limit = limit_volt;
        // check if the zero found
        if self.monitor_port.is_some() {
            if self.sensor.as_ref().unwrap().needs_search() {
                println!("MOT: Error: Not found!");
            } else {
                println!("MOT: Success!");
            }
        }
        !self.sensor.as_ref().unwrap().needs_search()
    }

    pub fn loop_foc(&mut self) {
        // update sensor - do this even in open-loop mode, as user may be switching between modes and we could lose track
        //                 of full rotations otherwise.
        if let Some(sensor) = &self.sensor {
            sensor.update();
        }

        // if open-loop do nothing
        if self.controller == MotionControlType::AngleOpenloop
            || self.controller == MotionControlType::VelocityOpenloop
        {
            return;
        }

        // if disabled do nothing
        if !self.enabled {
            return;
        }

        // Needs the update() to be called first
        // This function will not have numerical issues because it uses Sensor::getMechanicalAngle()
        // which is in range 0-2PI
        self.electrical_angle = self.electrical_angle();
        match self.torque_controller {
            TorqueControlType::Voltage => {
                // no need to do anything really
            }
            TorqueControlType::DcCurrent => {
                if self.current_sense.is_none() {
                    return;
                }
                // read overall current magnitude
                self.current.q = self
                    .current_sense
                    .as_ref()
                    .unwrap()
                    .get_dc_current(self.electrical_angle);
                // filter the value values
                self.current.q = self.LPF_current_q(self.current.q);
                // calculate the phase voltage
                self.voltage.q = self.PID_current_q(self.current_sp - self.current.q);
                // d voltage  - lag compensation
                if self.phase_inductance.is_some() {
                    self.voltage.d = self.constrain(
                        -self.current_sp
                            * self.shaft_velocity
                            * self.pole_pairs as f32
                            * self.phase_inductance.unwrap(),
                        -self.voltage_limit,
                        self.voltage_limit,
                    );
                } else {
                    self.voltage.d = 0.0;
                }
            }
            TorqueControlType::FocCurrent => {
                if self.current_sense.is_none() {
                    return;
                }
                // read dq currents
                self.current = self
                    .current_sense
                    .as_ref()
                    .unwrap()
                    .get_foc_currents(self.electrical_angle);
                // filter values
                self.current.q = self.LPF_current_q(self.current.q);
                self.current.d = self.LPF_current_d(self.current.d);
                // calculate the phase voltages
                self.voltage.q = self.PID_current_q(self.current_sp - self.current.q);
                self.voltage.d = self.PID_current_d(-self.current.d);
                // d voltage - lag compensation - TODO verify
                // if(self.phase_inductance.is_some()) self.voltage.d = self.constrain( self.voltage.d - self.current_sp*self.shaft_velocity*self.pole_pairs*self.phase_inductance.unwrap(), -self.voltage_limit, self.voltage_limit);
            }
            _ => {
                // no torque control selected
                println!("MOT: no torque control selected!");
            }
        }

        // set the phase voltage - FOC heart function :)
        self.set_phase_voltage(self.voltage.q, self.voltage.d, self.electrical_angle);
    }

    fn move_motor(&mut self, new_target: f32) {
        if self._isset(new_target) {
            self.target = new_target;
        }

        if self.motion_cnt < self.motion_downsample {
            self.motion_cnt += 1;
            return;
        }
        self.motion_cnt = 0;

        if self.controller != MotionControlType::AngleOpenloop
            && self.controller != MotionControlType::VelocityOpenloop
        {
            self.shaft_angle = self.shaft_angle();
        }
        self.shaft_velocity = self.shaft_velocity();

        if !self.enabled {
            return;
        }

        if let Some(KV_rating) = self.KV_rating {
            self.voltage_bemf = self.shaft_velocity / (KV_rating * _SQRT3) / _RPM_TO_RADS;
        }

        if !self.current_sense {
            if let Some(phase_resistance) = self.phase_resistance {
                self.current.q = (self.voltage.q - self.voltage_bemf) / phase_resistance;
            }
        }

        match self.controller {
            MotionControlType::Torque => {
                if self.torque_controller == TorqueControlType::Voltage {
                    if let Some(phase_resistance) = self.phase_resistance {
                        self.voltage.q = self.target * phase_resistance + self.voltage_bemf;
                    } else {
                        self.voltage.q = self.target;
                    }
                    self.voltage.q =
                        self._constrain(self.voltage.q, -self.voltage_limit, self.voltage_limit);

                    if let Some(phase_inductance) = self.phase_inductance {
                        self.voltage.d = self._constrain(
                            -self.target
                                * self.shaft_velocity
                                * self.pole_pairs as f32
                                * phase_inductance,
                            -self.voltage_limit,
                            self.voltage_limit,
                        );
                    } else {
                        self.voltage.d = 0.0;
                    }
                } else {
                    self.current.q = self.target;
                }
            }
            MotionControlType::Angle => {
                self.shaft_angle_sp = self.target;
                self.shaft_velocity_sp = self.feed_forward_velocity
                    + self.P_angle(self.shaft_angle_sp - self.shaft_angle);
                self.shaft_velocity_sp = self._constrain(
                    self.shaft_velocity_sp,
                    -self.voltage_limit,
                    self.voltage_limit,
                );
                self.current.q = self.PID_velocity(self.shaft_velocity_sp - self.shaft_velocity);

                if self.torque_controller == TorqueControlType::Voltage {
                    if let Some(phase_resistance) = self.phase_resistance {
                        self.voltage.q = self._constrain(
                            self.current.q * phase_resistance + self.voltage_bemf,
                            -self.voltage_limit,
                            self.voltage_limit,
                        );
                    } else {
                        self.voltage.q = self.current.q;
                    }

                    if let Some(phase_inductance) = self.phase_inductance {
                        self.voltage.d = self._constrain(
                            -self.current.q
                                * self.shaft_velocity
                                * self.pole_pairs as f32
                                * phase_inductance,
                            -self.voltage_limit,
                            self.voltage_limit,
                        );
                    } else {
                        self.voltage.d = 0.0;
                    }
                }
            }
            MotionControlType::Velocity => {
                self.shaft_velocity_sp = self.target;
                self.current.q = self.PID_velocity(self.shaft_velocity_sp - self.shaft_velocity);

                if self.torque_controller == TorqueControlType::Voltage {
                    if let Some(phase_resistance) = self.phase_resistance {
                        self.voltage.q = self._constrain(
                            self.current.q * phase_resistance + self.voltage_bemf,
                            -self.voltage_limit,
                            self.voltage_limit,
                        );
                    } else {
                        self.voltage.q = self.current.q;
                    }

                    if let Some(phase_inductance) = self.phase_inductance {
                        self.voltage.d = self._constrain(
                            -self.current.q
                                * self.shaft_velocity
                                * self.pole_pairs as f32
                                * phase_inductance,
                            -self.voltage_limit,
                            self.voltage_limit,
                        );
                    } else {
                        self.voltage.d = 0.0;
                    }
                }
            }
            MotionControlType::VelocityOpenloop => {
                self.shaft_velocity_sp = self.target;
                self.voltage.q = self.velocity_openloop(self.shaft_velocity_sp);
                self.voltage.d = 0.0;
            }
            MotionControlType::AngleOpenloop => {
                self.shaft_angle_sp = self.target;
                self.voltage.q = self.angle_openloop(self.shaft_angle_sp);
                self.voltage.d = 0.0;
            }
        }
    }

    fn set_phase_voltage(&mut self, Uq: f32, Ud: f32, angle_el: f32) {
        let mut center;
        let sector;
        let (_ca, _sa);

        match self.foc_modulation {
            FOCModulationType::Trapezoid120 => {
                sector = (6.0 * (self._normalize_angle(angle_el + _PI_6) / _2PI)) as usize;
                center = if self.modulation_centered {
                    self.driver.voltage_limit / 2.0
                } else {
                    Uq
                };

                if trap_120_map[sector][0] == _HIGH_IMPEDANCE {
                    self.Ua = center;
                    self.Ub = trap_120_map[sector][1] * Uq + center;
                    self.Uc = trap_120_map[sector][2] * Uq + center;
                    self.driver.set_phase_state(
                        PhaseState::PhaseOff,
                        PhaseState::PhaseOn,
                        PhaseState::PhaseOn,
                    );
                } else if trap_120_map[sector][1] == _HIGH_IMPEDANCE {
                    self.Ua = trap_120_map[sector][0] * Uq + center;
                    self.Ub = center;
                    self.Uc = trap_120_map[sector][2] * Uq + center;
                    self.driver.set_phase_state(
                        PhaseState::PhaseOn,
                        PhaseState::PhaseOff,
                        PhaseState::PhaseOn,
                    );
                } else {
                    self.Ua = trap_120_map[sector][0] * Uq + center;
                    self.Ub = trap_120_map[sector][1] * Uq + center;
                    self.Uc = center;
                    self.driver.set_phase_state(
                        PhaseState::PhaseOn,
                        PhaseState::PhaseOn,
                        PhaseState::PhaseOff,
                    );
                }
            }
            FOCModulationType::Trapezoid150 => {
                sector = (12.0 * (self._normalize_angle(angle_el + _PI_6) / _2PI)) as usize;
                center = if self.modulation_centered {
                    self.driver.voltage_limit / 2.0
                } else {
                    Uq
                };

                if trap_150_map[sector][0] == _HIGH_IMPEDANCE {
                    self.Ua = center;
                    self.Ub = trap_150_map[sector][1] * Uq + center;
                    self.Uc = trap_150_map[sector][2] * Uq + center;
                    self.driver.set_phase_state(
                        PhaseState::PhaseOff,
                        PhaseState::PhaseOn,
                        PhaseState::PhaseOn,
                    );
                } else if trap_150_map[sector][1] == _HIGH_IMPEDANCE {
                    self.Ua = trap_150_map[sector][0] * Uq + center;
                    self.Ub = center;
                    self.Uc = trap_150_map[sector][2] * Uq + center;
                    self.driver.set_phase_state(
                        PhaseState::PhaseOn,
                        PhaseState::PhaseOff,
                        PhaseState::PhaseOn,
                    );
                } else if trap_150_map[sector][2] == _HIGH_IMPEDANCE {
                    self.Ua = trap_150_map[sector][0] * Uq + center;
                    self.Ub = trap_150_map[sector][1] * Uq + center;
                    self.Uc = center;
                    self.driver.set_phase_state(
                        PhaseState::PhaseOn,
                        PhaseState::PhaseOn,
                        PhaseState::PhaseOff,
                    );
                } else {
                    self.Ua = trap_150_map[sector][0] * Uq + center;
                    self.Ub = trap_150_map[sector][1] * Uq + center;
                    self.Uc = trap_150_map[sector][2] * Uq + center;
                    self.driver.set_phase_state(
                        PhaseState::PhaseOn,
                        PhaseState::PhaseOn,
                        PhaseState::PhaseOn,
                    );
                }
            }
            FOCModulationType::SinePWM | FOCModulationType::SpaceVectorPWM => {
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
            }
        }

        self.driver.set_pwm(self.Ua, self.Ub, self.Uc);
    }

    fn velocity_openloop(&mut self, target_velocity: f32) -> f32 {
        let now_us = self._micros();
        let mut Ts = (now_us - self.open_loop_timestamp) as f32 * 1e-6;
        if Ts <= 0.0 || Ts > 0.5 {
            Ts = 1e-3;
        }

        self.shaft_angle = self._normalize_angle(self.shaft_angle + target_velocity * Ts);
        self.shaft_velocity = target_velocity;

        let mut Uq = self.voltage_limit;
        if let Some(phase_resistance) = self.phase_resistance {
            Uq = self._constrain(
                self.current_limit * phase_resistance + self.voltage_bemf.abs(),
                -self.voltage_limit,
                self.voltage_limit,
            );
            self.current.q = (Uq - self.voltage_bemf.abs()) / phase_resistance;
        }

        self.set_phase_voltage(
            Uq,
            0.0,
            self._electrical_angle(self.shaft_angle, self.pole_pairs),
        );
        self.open_loop_timestamp = now_us;

        Uq
    }

    fn angle_openloop(&mut self, target_angle: f32) -> f32 {
        let now_us = self._micros();
        let mut Ts = (now_us - self.open_loop_timestamp) as f32 * 1e-6;
        if Ts <= 0.0 || Ts > 0.5 {
            Ts = 1e-3;
        }

        if (target_angle - self.shaft_angle).abs() > (self.velocity_limit * Ts).abs() {
            self.shaft_angle +=
                self._sign(target_angle - self.shaft_angle) * self.velocity_limit.abs() * Ts;
            self.shaft_velocity = self.velocity_limit;
        } else {
            self.shaft_angle = target_angle;
            self.shaft_velocity = 0.0;
        }

        let mut Uq = self.voltage_limit;
        if let Some(phase_resistance) = self.phase_resistance {
            Uq = self._constrain(
                self.current_limit * phase_resistance + self.voltage_bemf.abs(),
                -self.voltage_limit,
                self.voltage_limit,
            );
            self.current.q = (Uq - self.voltage_bemf.abs()) / phase_resistance;
        }

        self.set_phase_voltage(
            Uq,
            0.0,
            self._electrical_angle(self._normalize_angle(self.shaft_angle), self.pole_pairs),
        );
        self.open_loop_timestamp = now_us;

        Uq
    }
}

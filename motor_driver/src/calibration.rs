// calibration.rs - モーターとセンサーのキャリブレーション機能

use crate::motor::BrushlessMotor;
use crate::sensors::{encoder, current};
use crate::driver::pwm::PwmDriver;

pub struct CalibrationConfig {
    // キャリブレーション設定
    pub current_sensing_samples: usize,
    pub phase_resistance_test_current: f32,
    pub phase_inductance_test_voltage: f32,
    pub encoder_offset_test_current: f32,
    pub pole_pair_detection_current: f32,
    pub kv_rate: f32,
    pub cogging: f32,
}

pub struct CalibrationResult {
    // キャリブレーション結果
    pub current_sensor_offsets: [f32; 3],
    pub phase_resistance: f32,
    pub phase_inductance: f32,
    pub encoder_electrical_offset: f32,
    pub detected_pole_pairs: u8,
    pub kv_rate: f32,
    pub cogging: f32,
}

pub struct Calibrator {
    config: CalibrationConfig,
}

impl Calibrator {
    pub fn new(config: CalibrationConfig) -> Self {
        Self { config }
    }

    /// モーター全体のキャリブレーションを実行
    pub fn run_full_calibration(
        &self,
        motor: &mut BrushlessMotor,
        pwm_driver: &mut PwmDriver,
        current_sensor: &mut current::CurrentSensor,
        encoder: &mut encoder::Encoder,
    ) -> CalibrationResult {
        // キャリブレーションステップを順番に実行
        println!("Starting calibration sequence...");

        // 1. 電流センサーのオフセットキャリブレーション
        println!("Calibrating current sensors...");
        let current_offsets = self.calibrate_current_sensors(current_sensor);

        // 2. 巻線抵抗の測定
        println!("Measuring phase resistance...");
        let phase_resistance = self.measure_phase_resistance(
            pwm_driver,
            current_sensor,
            current_offsets,
        );

        // 3. 巻線インダクタンスの測定
        println!("Measuring phase inductance...");
        let phase_inductance = self.measure_phase_inductance(
            pwm_driver,
            current_sensor,
            current_offsets,
        );

        // 4. エンコーダオフセットの検出
        println!("Detecting encoder offset...");
        let encoder_offset = self.detect_encoder_offset(
            pwm_driver,
            current_sensor,
            encoder,
            current_offsets,
        );

        // 5. 極対数の検出
        println!("Detecting pole pairs...");
        let pole_pairs = self.detect_pole_pairs(
            pwm_driver,
            current_sensor,
            encoder,
            current_offsets,
        );

        println!("Calibration completed successfully!");

        CalibrationResult {
            current_sensor_offsets: current_offsets,
            phase_resistance,
            phase_inductance,
            encoder_electrical_offset: encoder_offset,
            detected_pole_pairs: pole_pairs,
        }
    }

    /// 電流センサーのオフセットを測定（PWM無効時）
    fn calibrate_current_sensors(
        &self,
        current_sensor: &mut current::CurrentSensor,
    ) -> [f32; 3] {
        let mut offsets = [0.0f32; 3];
        let samples = self.config.current_sensing_samples;

        // 複数サンプルの平均を取って電流センサーのオフセットを計算
        for _ in 0..samples {
            let currents = current_sensor.read_phase_currents();
            for i in 0..3 {
                offsets[i] += currents[i] / samples as f32;
            }
            // 短い遅延
        }

        offsets
    }

    /// 相抵抗を測定（DC電流を流して電圧/電流で計算）
    fn measure_phase_resistance(
        &self,
        pwm_driver: &mut PwmDriver,
        current_sensor: &mut current::CurrentSensor,
        offsets: [f32; 3],
    ) -> f32 {
        // DC電流を流して抵抗を測定する実装
        // 詳細は略（電圧と電流の測定からR=V/Iを計算）
        0.1 // 仮の戻り値
    }

    /// 相インダクタンスを測定（電圧ステップに対する電流応答から計算）
    fn measure_phase_inductance(
        &self,
        pwm_driver: &mut PwmDriver,
        current_sensor: &mut current::CurrentSensor,
        offsets: [f32; 3],
    ) -> f32 {
        // 電圧ステップを印加してL=V/(di/dt)を計算
        // 詳細は略
        0.001 // 仮の戻り値
    }

    /// エンコーダの電気角オフセットを検出
    fn detect_encoder_offset(
        &self,
        pwm_driver: &mut PwmDriver,
        current_sensor: &mut current::CurrentSensor,
        encoder: &mut encoder::Encoder,
        offsets: [f32; 3],
    ) -> f32 {
        // 特定の電気角にモーターを配置し、
        // エンコーダ値との差を計算
        // 詳細は略
        0.0 // 仮の戻り値
    }
    fn detect_sensor_properties(&mut self) {
        // Simulated detection of sensor zero offset and direction
        self.zero_electric_angle = 1.2345; // Example value
        self.sensor_direction = Direction::CW; // Example direction
    }

    /// 極対数を検出（1機械回転あたりの電気回転数をカウント）
    fn detect_pole_pairs(
        &self,
        pwm_driver: &mut PwmDriver,
        current_sensor: &mut current::CurrentSensor,
        encoder: &mut encoder::Encoder,
        offsets: [f32; 3],
    ) -> u8 {
        // 低速でモーターを回転させ、1機械回転あたりの電気回転数を計測
        // 詳細は略
        7 // 仮の戻り値
    }
    fn estimate_pole_pairs(motor: &mut BLDCMotor, sensor: &MagneticSensorSPI) -> i32 {
        println!("Pole pairs (PP) estimator");
    
        let pp_search_voltage = 4.0; // maximum power_supply_voltage/2
        let pp_search_angle = 6.0 * PI; // search electrical angle to turn
    
        // Move motor to electrical angle 0
        motor.voltage_limit = pp_search_voltage;
        motor.move_angle(0.0);
        thread::sleep(Duration::from_secs(1));
    
        // Read initial sensor angle
        sensor.update();
        let angle_begin = sensor.get_angle();
        thread::sleep(Duration::from_millis(50));
    
        // Move motor slowly
        let mut motor_angle = 0.0;
        while motor_angle <= pp_search_angle {
            motor_angle += 0.01;
            sensor.update();
            motor.move_angle(motor_angle);
            thread::sleep(Duration::from_millis(1));
        }
        thread::sleep(Duration::from_secs(1));
    
        // Read final sensor angle
        sensor.update();
        let angle_end = sensor.get_angle();
        thread::sleep(Duration::from_millis(50));
    
        // Turn off motor
        motor.move_angle(0.0);
        thread::sleep(Duration::from_secs(1));
    
        // Calculate pole pair number
        let pp = ((pp_search_angle) / (angle_end - angle_begin)).round() as i32;
    
        println!("Estimated PP: {}", pp);
        println!("PP = Electrical angle / Encoder angle");
        println!(
            "{}/{}",
            pp_search_angle * 180.0 / PI,
            (angle_end - angle_begin) * 180.0 / PI
        );
        println!("Calculation: {}", (pp_search_angle) / (angle_end - angle_begin));
    
        // Validate pole pair number
        if pp <= 0 {
            println!("PP number cannot be negative");
            println!(" - Try changing the search_voltage value or motor/sensor configuration.");
            return 0;
        } else if pp > 30 {
            println!("PP number very high, possible error.");
        } else {
            println!("If PP is estimated well your motor should turn now!");
            println!(" - If it is not moving try to relaunch the program!");
            println!(" - You can also try to adjust the target voltage using input!");
        }
    
        pp
    }

    fn calculate_kv(&self, motor: &BLDCMotor) -> f32 {
        // KV rating calculation
        // KV = (shaft_velocity / target / SQRT(3)) * (30 / PI)
        let kv = motor.shaft_velocity / motor.target / SQRT_3 * 30.0 / PI;
        println!("Calculated KV rating: {}", kv);
        kv
    }

    fn run(&mut self, motor: &mut BLDCMotor) {
        loop {
            print!("Enter command (T: set voltage, K: calculate KV, Q: quit): ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();

            match input.trim() {
                "T" => {
                    print!("Enter target voltage: ");
                    io::stdout().flush().unwrap();
                    let mut voltage = String::new();
                    io::stdin().read_line(&mut voltage).unwrap();
                    if let Ok(vol) = voltage.trim().parse() {
                        self.set_target_voltage(vol);
                        motor.move_voltage(vol);
                    }
                }
                "K" => {
                    self.calculate_kv(motor);
                }
                "Q" => break,
                _ => println!("Invalid command"),
            }
        }
    }

    fn test_alignment_and_cogging(motor: &mut BLDCMotor, sensor: &MagneticSensorI2C, direction: i32) {
        motor.move_to(0.0);
        thread::sleep(Duration::from_millis(200));
        
        sensor.update();
        let initial_angle = sensor.get_angle();
        
        let shaft_rotation = 720.0; // 720 deg test
        let sample_count = (shaft_rotation * motor.pole_pairs) as usize;
        
        let mut st_dev_sum = 0.0;
        let mut mean = 0.0;
        let mut prev_mean = 0.0;
        
        for i in 0..sample_count {
            let shaft_angle = direction as f32 * i as f32 * shaft_rotation / sample_count as f32;
            let electric_angle = shaft_angle * motor.pole_pairs;
            
            // Move and wait
            motor.move_to(shaft_angle * PI / 180.0);
            thread::sleep(Duration::from_millis(5));
            
            // Measure
            sensor.update();
            let sensor_angle = (sensor.get_angle() - initial_angle) * 180.0 / PI;
            let sensor_electric_angle = sensor_angle * motor.pole_pairs;
            let electric_angle_error = electric_angle - sensor_electric_angle;
            
            // Print debug information
            println!(
                "{}\t{}\t{}",
                electric_angle, sensor_electric_angle, electric_angle_error
            );
            
            // Knuth standard deviation algorithm
            prev_mean = mean;
            mean += (electric_angle_error - mean) / (i + 1) as f32;
            st_dev_sum += (electric_angle_error - mean) * (electric_angle_error - prev_mean);
        }
        
        println!("\nALIGNMENT AND COGGING REPORT\n");
        println!("Direction: {}", direction);
        println!("Mean error (alignment): {} deg (electrical)", mean);
        println!(
            "Standard Deviation (cogging): {} deg (electrical)", 
            (st_dev_sum / sample_count as f32).sqrt()
        );
        
        println!("\nPlotting 3rd column of data (electricAngleError) will likely show sinusoidal cogging pattern with a frequency of 4xpole_pairs per rotation\n");
    }
}
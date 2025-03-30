/// PIDコントローラーの構造体
pub struct PIDController {
    kp: f64,      // 比例ゲイン
    ki: f64,      // 積分ゲイン
    kd: f64,      // 微分ゲイン
    setpoint: f64, // 目標値
    prev_error: f64, // 前回の誤差
    integral: f64,   // 積分項
    max_output: f64, // 出力の最大値
}

impl PIDController {
    /// 新しいPIDコントローラーを作成
    pub fn new(kp: f64, ki: f64, kd: f64) -> Self {
        PIDController {
            kp,
            ki,
            kd,
            setpoint: 0.0,
            prev_error: 0.0,
            integral: 0.0,
            min_output: f64::NEG_INFINITY,
            max_output: f64::INFINITY,
        }
    }

    /// 出力制限を設定
    pub fn output_limits(mut self, min: f64, max: f64) -> Self {
        self.min_output = min;
        self.max_output = max;
        self
    }

    /// 目標値を設定
    pub fn setpoint(mut self, setpoint: f64) -> Self {
        self.setpoint = setpoint;
        self
    }

    /// PID制御のメイン計算メソッド
    pub fn compute(&mut self, process_variable: f64) -> f64 {
        // 現在の誤差を計算
        let error = self.setpoint - process_variable;

        // 積分項を更新（積分誤差の蓄積）
        self.integral += error;

        // 微分項を計算（誤差の変化率）
        let derivative = error - self.prev_error;

        // PID制御出力を計算
        let mut output = 
            self.kp * error +           // 比例項
            self.ki * self.integral +   // 積分項
            self.kd * derivative;       // 微分項

        // 出力を制限
        output = output.max(self.min_output).min(self.max_output);

        // 前回の誤差を更新
        self.prev_error = error;

        output
    }

    /// 積分項をリセット
    pub fn reset(&mut self) {
        self.prev_error = 0.0;
        self.integral = 0.0;
    }
}

// 使用例
// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_pid_controller() {
//         // PIDコントローラーを初期化
//         let mut pid = PIDController::new(1.0, 0.1, 0.01)
//             .setpoint(100.0)
//             .output_limits(-10.0, 10.0);

//         // シミュレーション
//         let mut current_value = 0.0;
//         for _ in 0..50 {
//             let control_signal = pid.compute(current_value);
//             current_value += control_signal;
            
//             println!("Current: {}, Control: {}", current_value, control_signal);
//         }
//     }
// }
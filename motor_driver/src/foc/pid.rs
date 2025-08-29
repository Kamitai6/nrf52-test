/// PIDコントローラーの構造体
pub struct PID {
    kp: f64,      // 比例ゲイン
    ki: f64,      // 積分ゲイン
    kd: f64,      // 微分ゲイン
    prev_error: f64, // 前回の誤差
    integral: f64,   // 積分項
    max_output: f64, // 出力の最大値
}

impl PID {
    /// 新しいPIDコントローラーを作成
    pub fn new(kp: f64, ki: f64, kd: f64, max: f64) -> Self {
        PID {
            kp,
            ki,
            kd,
            prev_error: 0.0,
            integral: 0.0,
            max_output: max,
        }
    }

    /// PID制御のメイン計算メソッド
    pub fn compute(&mut self, target: f64, variable: f64) -> f64 {
        // 現在の誤差を計算
        let error = target - variable;

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
        output = output.min(self.max_output);

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

use crate::periph::adc;


pub struct BldcCurrent {
    resistor: f32,
    gain: f32,
    offset: [f32; 3],
}

impl BldcCurrent {
    fn new(shunt_resistor: f32, gain: f32) -> Self {
        Self {
            resistor: shunt_resistor,
            gain: 1.0 / shunt_resistor / gain,
            offset: [0.0; 3],
        }
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

    fn calibrate_offset() {
        const int calibration_rounds = 2000;

        // find adc offset = zero current voltage
        offset_ia = 0;
        offset_ib = 0;
        offset_ic = 0;
        // read the adc voltage 1000 times ( arbitrary number )
        for (int i = 0; i < calibration_rounds; i++) {
            _startADC3PinConversionLowSide();
            if(_isset(pinA)) offset_ia += (_readADCVoltageLowSide(pinA, params));
            if(_isset(pinB)) offset_ib += (_readADCVoltageLowSide(pinB, params));
            if(_isset(pinC)) offset_ic += (_readADCVoltageLowSide(pinC, params));
            _delay(1);
        }
        // calculate the mean offsets
        if(_isset(pinA)) offset_ia = offset_ia / calibration_rounds;
        if(_isset(pinB)) offset_ib = offset_ib / calibration_rounds;
        if(_isset(pinC)) offset_ic = offset_ic / calibration_rounds;
    }

    fn lowpass_filter() {
        unsigned long timestamp = _micros();
        float dt = (timestamp - timestamp_prev)*1e-6f;

        if (dt < 0.0f ) dt = 1e-3f;
        else if(dt > 0.3f) {
            y_prev = x;
            timestamp_prev = timestamp;
            return x;
        }

        float alpha = Tf/(Tf + dt);
        float y = alpha*y_prev + (1.0f - alpha)*x;
        y_prev = y;
        timestamp_prev = timestamp;
        return y;
    }
}

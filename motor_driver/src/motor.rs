pub struct Motor {
    pole_pairs: i32,
    phase_resistance: f32,
    kv_rating: Option<f32>,
    phase_inductance: f32,
}

impl Motor {
    pub fn new(
        pole_pairs: i32,
        phase_resistance: f32,
        kv: Option<f32>,
        phase_inductance: f32) -> Self
    {
        Self {
            pole_pairs,
            phase_resistance,
            kv_rating: kv,
            phase_inductance,
        }
    }

    pub fn enable() {

    }

    pub fn disable() {
        
    }
}

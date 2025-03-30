pub struct SixStepControl {
    speed: f32,
    phase: u8,
}

impl SixStepControl {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            phase: 0,
        }
    }

    pub fn update(&mut self) -> (f32, f32, f32) {
        match self.phase {
            0 => (
                speed,     // A
                0.,        // B
                speed / 2, // C
            )
            1 => (
                speed,
                speed / 2,
                0.,
            )
            2 => (
                speed / 2,
                speed,
                0.,
            )
            3 => (
                0.,
                speed,
                speed / 2,
            )
            4 => (
                0.,
                speed / 2,
                speed,
            )
            5 => (
                speed / 2,
                0.,
                speed,
            )
            _ => (0., 0., 0.)
        }
        self.phase = phase % 6 + 1;
    }
}

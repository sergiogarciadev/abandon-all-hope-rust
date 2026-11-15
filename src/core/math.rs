use std::f32::consts::TAU;

/// Small deterministic LCG RNG so procedural models come out identical every load.
pub struct SeededRng(u32);

impl SeededRng {
    pub fn new(seed: u32) -> Self {
        Self(seed)
    }

    /// Returns a float in `[0.0, 1.0)`.
    pub fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.0 as f32) / 4294967296.0
    }

    #[allow(dead_code)]
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + self.next_f32() * (b - a)
    }
}

#[allow(dead_code)]
pub fn wrap_a(a: f32) -> f32 {
    a - (a / TAU).round() * TAU
}

#[allow(dead_code)]
pub fn damp(a: f32, b: f32, k: f32, dt: f32) -> f32 {
    a + (b - a) * (1.0 - (-k * dt).exp())
}

#[allow(dead_code)]
pub fn ease(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

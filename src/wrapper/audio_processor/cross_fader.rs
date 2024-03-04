pub struct CrossFader {}

impl CrossFader {
    pub fn new(sample_rate: f64, fade_time_secs: f32) -> Self {
        todo!()
    }

    pub fn reset(&mut self) {}

    pub fn apply_crossfade(&self, fade_in: &[f32], fade_out: &[f32], output: &mut [f32]) {}

    pub fn advance(&mut self, sample_count: usize) {}

    pub fn is_done(&self) -> bool {
        todo!()
    }
}

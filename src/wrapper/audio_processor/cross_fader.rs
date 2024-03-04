pub struct CrossFader {
    remaining_fade_time_samples: u32,
    fade_time_samples: u32,
}

impl CrossFader {
    pub fn new(sample_rate: f64, fade_time_secs: f64) -> Self {
        let fade_time_samples = (fade_time_secs * sample_rate).floor() as u32;

        Self {
            fade_time_samples,
            remaining_fade_time_samples: fade_time_samples,
        }
    }

    pub fn reset(&mut self) {
        self.remaining_fade_time_samples = self.fade_time_samples
    }

    pub fn apply_crossfade(&self, fade_in: &[f32], fade_out: &[f32], output: &mut [f32]) {
        assert_eq!(fade_in.len(), output.len()); // To help with compiler optimizations a bit
        assert_eq!(fade_out.len(), output.len());

        let mut remaining_ratio =
            self.remaining_fade_time_samples as f32 / self.fade_time_samples as f32;
        let step_per_sample = 1.0 / self.fade_time_samples as f32;
        dbg!(
            remaining_ratio,
            step_per_sample,
            self.remaining_fade_time_samples,
            self.fade_time_samples
        );

        for ((fade_in, fade_out), output) in fade_in.iter().zip(fade_out).zip(output) {
            let fade_out_ratio = remaining_ratio;
            let fade_in_ratio = 1.0 - fade_out_ratio;

            *output = (fade_in * fade_in_ratio) + (fade_out * fade_out_ratio);

            remaining_ratio = (remaining_ratio - step_per_sample).max(0.0);
        }
    }

    pub fn advance(&mut self, sample_count: u32) {
        self.remaining_fade_time_samples = self
            .remaining_fade_time_samples
            .saturating_sub(sample_count);
    }

    pub fn is_done(&self) -> bool {
        self.remaining_fade_time_samples == 0
    }
}

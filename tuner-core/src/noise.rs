//! DC removal and noise-floor tracking.

/// Single-pole DC-blocking high-pass filter (`y = x - prev_x + pole * prev_y`).
///
/// At the default pole of 0.995 the −3 dB corner sits near 38 Hz for 48 kHz.
#[derive(Debug, Clone, Copy)]
pub struct DcBlocker {
    pole: f32,
    prev_x: f32,
    prev_y: f32,
}

impl Default for DcBlocker {
    fn default() -> Self {
        Self::new()
    }
}

impl DcBlocker {
    /// New blocker with the default pole (0.995).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pole: 0.995,
            prev_x: 0.0,
            prev_y: 0.0,
        }
    }

    /// New blocker with a custom pole in `(0, 1)`.
    #[must_use]
    pub const fn with_pole(pole: f32) -> Self {
        Self {
            pole,
            prev_x: 0.0,
            prev_y: 0.0,
        }
    }

    /// Process a single sample.
    pub fn process_sample(&mut self, x: f32) -> f32 {
        let y = x - self.prev_x + self.pole * self.prev_y;
        self.prev_x = x;
        self.prev_y = y;
        y
    }

    /// Process a buffer in place.
    pub fn process_in_place(&mut self, buf: &mut [f32]) {
        for s in buf.iter_mut() {
            *s = self.process_sample(*s);
        }
    }

    /// Reset the filter state.
    pub fn reset(&mut self) {
        self.prev_x = 0.0;
        self.prev_y = 0.0;
    }
}

/// Exponentially-smoothed estimate of the background noise RMS.
///
/// Only frames that are quiet relative to the current estimate update it, so a
/// sustained note does not raise the floor.
#[derive(Debug, Clone, Copy)]
pub struct NoiseFloor {
    rms: f32,
    alpha: f32,
    initialised: bool,
}

impl NoiseFloor {
    /// New tracker with smoothing factor `alpha` in `(0, 1)`.
    #[must_use]
    pub const fn new(alpha: f32) -> Self {
        Self {
            rms: 0.0,
            alpha,
            initialised: false,
        }
    }

    /// Update the floor with `frame_rms`, but only if the frame is within 6 dB
    /// (a factor of two) of the current estimate. The first frame always
    /// initialises the estimate.
    pub fn update_if_quiet(&mut self, frame_rms: f32) {
        if !self.initialised {
            self.rms = frame_rms;
            self.initialised = true;
            return;
        }
        if frame_rms <= 2.0 * self.rms {
            self.rms = self.alpha * frame_rms + (1.0 - self.alpha) * self.rms;
        }
    }

    /// Current noise-floor RMS estimate.
    #[must_use]
    pub const fn rms(&self) -> f32 {
        self.rms
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::synth_sine;
    use approx::assert_relative_eq;

    #[test]
    fn dc_blocker_removes_constant_offset() {
        let mut dc = DcBlocker::new();
        let mut buf = [1.0_f32; 4096];
        dc.process_in_place(&mut buf);
        // After settling, output hovers near zero.
        let tail_mean: f32 = buf[2048..].iter().sum::<f32>() / 2048.0;
        assert!(tail_mean.abs() < 0.05, "tail mean {tail_mean}");
    }

    #[test]
    fn dc_blocker_preserves_ac() {
        let mut dc = DcBlocker::new();
        let mut buf = synth_sine(1000.0, 48_000, 4096, 0.5);
        let in_rms: f32 = (buf.iter().map(|x| x * x).sum::<f32>() / buf.len() as f32).sqrt();
        dc.process_in_place(&mut buf);
        let out_rms: f32 =
            (buf[1024..].iter().map(|x| x * x).sum::<f32>() / (buf.len() - 1024) as f32).sqrt();
        // 1 kHz is far above the corner → amplitude essentially preserved.
        assert_relative_eq!(out_rms, in_rms, epsilon = 0.05);
    }

    #[test]
    fn dc_in_place_matches_per_sample() {
        let input = synth_sine(440.0, 48_000, 512, 0.5);
        let mut a = input.clone();
        let mut dc1 = DcBlocker::new();
        dc1.process_in_place(&mut a);

        let mut dc2 = DcBlocker::new();
        let b: alloc::vec::Vec<f32> = input.iter().map(|&x| dc2.process_sample(x)).collect();
        for (x, y) in a.iter().zip(b.iter()) {
            assert_relative_eq!(*x, *y, epsilon = 1e-7);
        }
    }

    #[test]
    fn noise_floor_initialises_on_first_update() {
        let mut nf = NoiseFloor::new(0.1);
        nf.update_if_quiet(0.01);
        assert_relative_eq!(nf.rms(), 0.01, epsilon = 1e-6);
    }

    #[test]
    fn noise_floor_ignores_loud_frames() {
        let mut nf = NoiseFloor::new(0.1);
        nf.update_if_quiet(0.01);
        nf.update_if_quiet(5.0); // loud → ignored
        assert_relative_eq!(nf.rms(), 0.01, epsilon = 1e-6);
    }

    #[test]
    fn noise_floor_smooths_similar_frames() {
        let mut nf = NoiseFloor::new(0.5);
        nf.update_if_quiet(0.01);
        nf.update_if_quiet(0.015);
        assert!(nf.rms() > 0.01 && nf.rms() < 0.015);
    }
}

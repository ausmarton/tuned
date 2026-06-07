//! Windowing and magnitude-spectrum computation.

use alloc::vec::Vec;
use num_complex::Complex;
use rustfft::FftPlanner;

#[cfg(not(feature = "std"))]
use libm::cosf;

#[cfg(feature = "std")]
#[inline]
fn cosf(x: f32) -> f32 {
    x.cos()
}

/// A periodic Hann window of fixed length.
#[derive(Debug, Clone)]
pub struct HannWindow {
    coeffs: Vec<f32>,
}

impl HannWindow {
    /// Build a periodic Hann window: `w[k] = 0.5 * (1 - cos(2π k / N))`.
    #[must_use]
    pub fn new(n: usize) -> Self {
        let two_pi = core::f32::consts::TAU;
        let coeffs = (0..n)
            .map(|k| 0.5 * (1.0 - cosf(two_pi * k as f32 / n as f32)))
            .collect();
        Self { coeffs }
    }

    /// Window length.
    #[must_use]
    pub fn len(&self) -> usize {
        self.coeffs.len()
    }

    /// Whether the window is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.coeffs.is_empty()
    }

    /// Multiply `samples` in place by the window coefficients.
    ///
    /// # Panics
    /// Panics if `samples.len() != self.len()`.
    pub fn apply(&self, samples: &mut [f32]) {
        assert_eq!(samples.len(), self.coeffs.len(), "window length mismatch");
        for (s, w) in samples.iter_mut().zip(self.coeffs.iter()) {
            *s *= *w;
        }
    }
}

/// Compute the magnitude spectrum of `buffer` after applying `window`.
///
/// Returns a vector of length `fft_len / 2 + 1` (the non-negative frequencies).
///
/// # Panics
/// Panics if `buffer.len() != window.len()` or if the length is not a power of two.
#[must_use]
pub fn magnitude_spectrum(buffer: &[f32], window: &HannWindow) -> Vec<f32> {
    assert_eq!(buffer.len(), window.len(), "buffer/window length mismatch");
    let fft_len = buffer.len();
    assert!(
        fft_len.is_power_of_two(),
        "fft length must be a power of two"
    );

    let mut windowed: Vec<f32> = buffer.to_vec();
    window.apply(&mut windowed);

    let mut spectrum: Vec<Complex<f32>> = windowed.iter().map(|&x| Complex::new(x, 0.0)).collect();

    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(fft_len);
    fft.process(&mut spectrum);

    spectrum[..=(fft_len / 2)]
        .iter()
        .map(|c| c.re.hypot(c.im))
        .collect()
}

/// Frequency of FFT bin `bin` for the given sample rate and FFT length.
#[must_use]
pub fn bin_to_hz(bin: usize, sample_rate_hz: u32, fft_len: usize) -> f32 {
    bin as f32 * sample_rate_hz as f32 / fft_len as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::synth_sine;
    use approx::assert_relative_eq;

    #[test]
    fn hann_endpoints_are_zero() {
        let w = HannWindow::new(1024);
        let c = &w.coeffs;
        assert_relative_eq!(c[0], 0.0, epsilon = 1e-6);
    }

    #[test]
    fn hann_centre_is_one() {
        let n = 1024;
        let w = HannWindow::new(n);
        assert_relative_eq!(w.coeffs[n / 2], 1.0, epsilon = 1e-4);
    }

    #[test]
    fn hann_empty() {
        let w = HannWindow::new(0);
        assert!(w.is_empty());
        assert_eq!(w.len(), 0);
    }

    #[test]
    fn apply_matches_manual() {
        let w = HannWindow::new(8);
        let mut buf = vec![1.0_f32; 8];
        w.apply(&mut buf);
        for (b, c) in buf.iter().zip(w.coeffs.iter()) {
            assert_relative_eq!(*b, *c, epsilon = 1e-6);
        }
    }

    #[test]
    fn magnitude_spectrum_picks_sine_bin() {
        let fft_len = 4096;
        let sr = 48_000;
        let buf = synth_sine(440.0, sr, fft_len, 1.0);
        let w = HannWindow::new(fft_len);
        let mag = magnitude_spectrum(&buf, &w);
        let peak = mag
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;
        let peak_hz = bin_to_hz(peak, sr, fft_len);
        assert!((peak_hz - 440.0).abs() < 20.0, "peak at {peak_hz} Hz");
    }

    #[test]
    fn bin_to_hz_known_values() {
        assert_relative_eq!(bin_to_hz(0, 48_000, 4096), 0.0, epsilon = 1e-6);
        assert_relative_eq!(bin_to_hz(1, 48_000, 4096), 11.71875, epsilon = 1e-4);
    }

    #[test]
    #[should_panic(expected = "length mismatch")]
    fn mismatch_panics() {
        let w = HannWindow::new(8);
        let buf = vec![0.0_f32; 16];
        let _ = magnitude_spectrum(&buf, &w);
    }

    #[test]
    #[should_panic(expected = "power of two")]
    fn non_pow2_panics() {
        let w = HannWindow::new(12);
        let buf = vec![0.0_f32; 12];
        let _ = magnitude_spectrum(&buf, &w);
    }
}

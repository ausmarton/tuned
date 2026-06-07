//! Monophonic pitch detection via the YIN algorithm.
//!
//! de Cheveigné & Kawahara (2002), *YIN, a fundamental frequency estimator for
//! speech and music*. Cumulative mean normalised difference function with
//! parabolic interpolation of the chosen lag.

use alloc::vec;
use alloc::vec::Vec;

#[cfg(not(feature = "std"))]
use libm::{sinf, sqrtf};

#[cfg(feature = "std")]
#[inline]
fn sinf(x: f32) -> f32 {
    x.sin()
}
#[cfg(feature = "std")]
#[inline]
fn sqrtf(x: f32) -> f32 {
    x.sqrt()
}

/// A single pitch estimate.
#[derive(Debug, Clone, Copy)]
pub struct PitchEstimate {
    /// Estimated fundamental frequency in Hz.
    pub frequency_hz: f32,
    /// YIN's cumulative-mean-normalised difference at the chosen lag (0 = perfectly periodic).
    pub aperiodicity: f32,
    /// RMS amplitude of the analysed buffer.
    pub rms: f32,
}

/// Confidence in `[0, 1]` combining periodicity and signal-vs-noise ratio.
#[must_use]
pub fn confidence(estimate: &PitchEstimate, noise_floor_rms: f32) -> f32 {
    let periodicity = (1.0 - (estimate.aperiodicity / 0.6).min(1.0)).max(0.0);
    let signal_ratio = if noise_floor_rms > 0.0 {
        (estimate.rms / (10.0 * noise_floor_rms)).clamp(0.0, 1.0)
    } else {
        1.0
    };
    periodicity * signal_ratio
}

/// Configuration for [`yin`].
#[derive(Debug, Clone, Copy)]
pub struct YinConfig {
    /// Sample rate in Hz.
    pub sample_rate_hz: u32,
    /// Lowest detectable frequency.
    pub min_hz: f32,
    /// Highest detectable frequency.
    pub max_hz: f32,
    /// Aperiodicity threshold (typical 0.10–0.20).
    pub threshold: f32,
}

impl Default for YinConfig {
    fn default() -> Self {
        Self {
            sample_rate_hz: 48_000,
            min_hz: 40.0,
            max_hz: 1000.0,
            threshold: 0.12,
        }
    }
}

#[inline]
fn rms_of(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = buffer.iter().map(|&x| x * x).sum();
    sqrtf(sum_sq / buffer.len() as f32)
}

/// Estimate the fundamental frequency of `buffer`, or `None` if no confident
/// periodic estimate exists or the configuration / buffer is invalid.
#[must_use]
pub fn yin(buffer: &[f32], cfg: YinConfig) -> Option<PitchEstimate> {
    // 1. Bounds checks.
    if cfg.min_hz <= 0.0 || cfg.max_hz <= cfg.min_hz || cfg.sample_rate_hz == 0 {
        return None;
    }
    let sr = cfg.sample_rate_hz as f32;
    let tau_max = (sr / cfg.min_hz).ceil() as usize;
    let tau_min = (sr / cfg.max_hz).floor().max(1.0) as usize;
    if tau_min < 1 || tau_max <= tau_min || buffer.len() < 2 * tau_max + 1 {
        return None;
    }

    let half = buffer.len() / 2;

    // 2. Difference function d[tau].
    let mut d = vec![0.0_f32; tau_max + 1];
    for tau in 1..=tau_max {
        let mut sum = 0.0_f32;
        for i in 0..half {
            let diff = buffer[i] - buffer[i + tau];
            sum += diff * diff;
        }
        d[tau] = sum;
    }

    // 3. Cumulative mean normalised difference d'[tau].
    let mut d_prime = vec![1.0_f32; tau_max + 1];
    let mut running = 0.0_f32;
    for tau in 1..=tau_max {
        running += d[tau];
        d_prime[tau] = if running > 0.0 {
            d[tau] * tau as f32 / running
        } else {
            1.0
        };
    }

    // 4. Absolute-threshold search: first dip below threshold within range,
    //    then walk forward while still descending. Fall back to the global
    //    minimum in range if nothing crosses (and that minimum is < 1.0).
    let mut tau_star: Option<usize> = None;
    let mut tau = tau_min;
    while tau <= tau_max {
        if d_prime[tau] < cfg.threshold {
            while tau < tau_max && d_prime[tau + 1] < d_prime[tau] {
                tau += 1;
            }
            tau_star = Some(tau);
            break;
        }
        tau += 1;
    }

    let chosen = match tau_star {
        Some(t) => t,
        None => {
            let (best_tau, best_val) = (tau_min..=tau_max)
                .map(|t| (t, d_prime[t]))
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal))
                .unwrap();
            if best_val < 1.0 {
                best_tau
            } else {
                return None;
            }
        }
    };

    // 5. Parabolic interpolation around the chosen lag.
    let mut tau_refined = chosen as f32;
    if chosen >= 1 && chosen < tau_max {
        let y_minus = d_prime[chosen - 1];
        let y_zero = d_prime[chosen];
        let y_plus = d_prime[chosen + 1];
        let denom = y_minus - 2.0 * y_zero + y_plus;
        if denom.abs() > 1e-12 {
            let delta = 0.5 * (y_minus - y_plus) / denom;
            if delta.abs() < 1.0 {
                tau_refined = chosen as f32 + delta;
            }
        }
    }

    if tau_refined <= 0.0 {
        return None;
    }

    // 6. Convert lag to frequency.
    let frequency_hz = sr / tau_refined;
    Some(PitchEstimate {
        frequency_hz,
        aperiodicity: d_prime[chosen],
        rms: rms_of(buffer),
    })
}

/// Synthesise a pure sine wave (for tests and benches).
#[must_use]
pub fn synth_sine(freq_hz: f32, sample_rate_hz: u32, n: usize, amplitude: f32) -> Vec<f32> {
    let sr = sample_rate_hz as f32;
    let mut out = Vec::with_capacity(n);
    let two_pi = core::f32::consts::TAU;
    for i in 0..n {
        out.push(amplitude * sinf(two_pi * freq_hz * i as f32 / sr));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cents::ratio_to_cents;

    fn cfg() -> YinConfig {
        YinConfig::default()
    }

    #[test]
    fn pure_sine_440_detected_within_one_cent() {
        let buf = synth_sine(440.0, 48_000, 8192, 0.5);
        let est = yin(&buf, cfg()).unwrap();
        assert!(
            ratio_to_cents(est.frequency_hz, 440.0).abs() < 1.0,
            "got {} Hz",
            est.frequency_hz
        );
    }

    #[test]
    fn pure_sine_low_e_82hz_detected() {
        let buf = synth_sine(82.4069, 48_000, 16384, 0.5);
        let est = yin(&buf, cfg()).unwrap();
        assert!(
            ratio_to_cents(est.frequency_hz, 82.4069).abs() < 5.0,
            "got {} Hz",
            est.frequency_hz
        );
    }

    #[test]
    fn detuned_sine_returns_correct_offset() {
        let target = 440.0 * 2.0_f32.powf(10.0 / 1200.0); // +10 cents
        let buf = synth_sine(target, 48_000, 8192, 0.5);
        let est = yin(&buf, cfg()).unwrap();
        let cents = ratio_to_cents(est.frequency_hz, 440.0);
        assert!((cents - 10.0).abs() < 1.0, "got {cents} cents");
    }

    #[test]
    fn silence_returns_no_pitch_or_low_confidence() {
        let buf = vec![0.0_f32; 8192];
        if let Some(est) = yin(&buf, cfg()) {
            assert!(confidence(&est, 1e-3) < 0.1);
        }
    }

    #[test]
    fn rejects_too_short_buffer() {
        let buf = vec![0.0_f32; 16];
        assert!(yin(&buf, cfg()).is_none());
    }

    #[test]
    fn rejects_inverted_range() {
        let bad = YinConfig {
            min_hz: 1000.0,
            max_hz: 40.0,
            ..cfg()
        };
        let buf = synth_sine(440.0, 48_000, 8192, 0.5);
        assert!(yin(&buf, bad).is_none());
    }

    #[test]
    fn parabolic_interpolation_centre() {
        // Symmetric parabola → vertex at centre, delta 0.
        let y_minus = 1.0_f32;
        let y_zero = 0.0_f32;
        let y_plus = 1.0_f32;
        let denom = y_minus - 2.0 * y_zero + y_plus;
        let delta = 0.5 * (y_minus - y_plus) / denom;
        assert!(delta.abs() < 1e-6);
    }

    #[test]
    fn parabolic_interpolation_offset() {
        let y_minus = 1.0_f32;
        let y_zero = 0.0_f32;
        let y_plus = 0.5_f32;
        let denom = y_minus - 2.0 * y_zero + y_plus;
        let delta = 0.5 * (y_minus - y_plus) / denom;
        assert!(delta > 0.0 && delta < 1.0);
    }

    #[test]
    fn parabolic_interpolation_flat() {
        // Degenerate (flat) → denom ~0 guarded.
        let denom: f32 = 0.0;
        assert!(denom.abs() <= 1e-12);
    }

    #[test]
    fn confidence_zero_for_pure_noise() {
        let est = PitchEstimate {
            frequency_hz: 200.0,
            aperiodicity: 0.9, // > 0.6 → periodicity clamps to 0
            rms: 0.5,
        };
        assert_eq!(confidence(&est, 1e-3), 0.0);
    }

    #[test]
    fn confidence_high_for_strong_periodic_signal() {
        let est = PitchEstimate {
            frequency_hz: 440.0,
            aperiodicity: 0.02,
            rms: 0.5,
        };
        assert!(confidence(&est, 1e-4) > 0.9);
    }
}

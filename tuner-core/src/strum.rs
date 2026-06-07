//! Strum analysis: per-string band-pass filtering followed by YIN.
//!
//! Each string gets its own narrow band-pass built from **three** cascaded RBJ
//! biquads (6th order, ~36 dB/oct). Two cascaded biquads (4th order) leaked
//! neighbouring strings badly enough to misdetect (HANDOFF.md §6.2). The first
//! half of every filtered buffer is discarded before YIN because the cascade's
//! transient ringing wrecks low-frequency detection otherwise.

use crate::cents::{classify, ratio_to_cents, Direction};
use crate::pitch::{yin, YinConfig};
use crate::tunings::Tuning;
use alloc::vec::Vec;

#[cfg(not(feature = "std"))]
use libm::{cosf, log2f, powf, sinf, sqrtf};

#[cfg(feature = "std")]
#[inline]
fn cosf(x: f32) -> f32 {
    x.cos()
}
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
#[cfg(feature = "std")]
#[inline]
fn log2f(x: f32) -> f32 {
    x.log2()
}
#[cfg(feature = "std")]
#[inline]
fn powf(b: f32, e: f32) -> f32 {
    b.powf(e)
}

/// Per-string result inside a [`StrumReport`].
#[derive(Debug, Clone)]
pub struct StringResult {
    /// Index of the string within the tuning (lowest = 0).
    pub string_index: usize,
    /// String name, e.g. `"E2"`.
    pub name: &'static str,
    /// Target frequency in Hz.
    pub target_hz: f32,
    /// Detected frequency, if any.
    pub detected_hz: Option<f32>,
    /// Cents offset from target, if detected.
    pub cents_off: Option<f32>,
    /// Tuning direction, if detected.
    pub direction: Option<Direction>,
    /// Detection confidence in `[0, 1]`.
    pub confidence: f32,
}

/// Result of analysing a whole strum.
#[derive(Debug, Clone)]
pub struct StrumReport {
    /// One entry per string, lowest first.
    pub strings: Vec<StringResult>,
}

impl StrumReport {
    /// Number of strings that were detected.
    #[must_use]
    pub fn detected_count(&self) -> usize {
        self.strings
            .iter()
            .filter(|s| s.detected_hz.is_some())
            .count()
    }

    /// Number of strings detected and within their in-tune window.
    #[must_use]
    pub fn in_tune_count(&self) -> usize {
        self.strings
            .iter()
            .filter(|s| s.direction == Some(Direction::InTune))
            .count()
    }
}

/// Configuration for [`analyse_strum`].
#[derive(Debug, Clone, Copy)]
pub struct StrumConfig {
    /// Sample rate in Hz.
    pub sample_rate_hz: u32,
    /// A4 reference frequency.
    pub a4_hz: f32,
    /// Symmetric in-tune window in cents (default 5.0).
    pub in_tune_window_cents: f32,
    /// Half-width of each string's band in cents (default 100.0).
    pub band_half_cents: f32,
    /// YIN threshold (default 0.15).
    pub yin_threshold: f32,
    /// Minimum confidence to accept a detection (default 0.20).
    pub min_confidence: f32,
}

impl Default for StrumConfig {
    fn default() -> Self {
        Self {
            sample_rate_hz: 48_000,
            a4_hz: 440.0,
            in_tune_window_cents: 5.0,
            band_half_cents: 100.0,
            yin_threshold: 0.15,
            min_confidence: 0.20,
        }
    }
}

#[inline]
fn rms_of(buf: &[f32]) -> f32 {
    if buf.is_empty() {
        return 0.0;
    }
    let s: f32 = buf.iter().map(|&x| x * x).sum();
    sqrtf(s / buf.len() as f32)
}

/// A normalised RBJ biquad (Direct Form I).
#[derive(Debug, Clone, Copy)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Biquad {
    fn bandpass(centre: f32, q: f32, sr: f32) -> Self {
        let w0 = core::f32::consts::TAU * centre / sr;
        let cos_w0 = cosf(w0);
        let alpha = sinf(w0) / (2.0 * q);
        let a0 = 1.0 + alpha;
        Self {
            b0: alpha / a0,
            b1: 0.0,
            b2: -alpha / a0,
            a1: -2.0 * cos_w0 / a0,
            a2: (1.0 - alpha) / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// Quality factor for a band-pass spanning `[low, high]`.
fn band_q(low: f32, high: f32) -> f32 {
    let bw_octaves = log2f(high / low);
    let q = (powf(2.0, bw_octaves) - 1.0).recip() * powf(2.0, bw_octaves / 2.0);
    q.clamp(0.3, 20.0)
}

/// Three-cascaded-biquad (6th-order) band-pass filter.
#[doc(hidden)]
#[must_use]
pub fn bandpass(input: &[f32], sample_rate_hz: u32, low_hz: f32, high_hz: f32) -> Vec<f32> {
    let sr = sample_rate_hz as f32;
    let centre = sqrtf(low_hz * high_hz);
    let q = band_q(low_hz, high_hz);
    let mut stages = [
        Biquad::bandpass(centre, q, sr),
        Biquad::bandpass(centre, q, sr),
        Biquad::bandpass(centre, q, sr),
    ];
    input
        .iter()
        .map(|&x| {
            let mut s = x;
            for stage in &mut stages {
                s = stage.process(s);
            }
            s
        })
        .collect()
}

/// Unused noise-floor helper retained for diagnostics.
#[allow(dead_code)]
fn estimate_noise_floor(buf: &[f32]) -> f32 {
    rms_of(buf)
}

/// Analyse a strum and report per-string tuning offsets.
#[must_use]
pub fn analyse_strum(buffer: &[f32], tuning: &Tuning, cfg: StrumConfig) -> StrumReport {
    let total_rms = rms_of(buffer);
    let silent = total_rms < 1e-4;

    let mut strings = Vec::with_capacity(tuning.strings.len());
    for (i, spec) in tuning.strings.iter().enumerate() {
        let target = spec.freq_hz(cfg.a4_hz);
        let low = target * powf(2.0, -cfg.band_half_cents / 1200.0);
        let high = target * powf(2.0, cfg.band_half_cents / 1200.0);

        let mut result = StringResult {
            string_index: i,
            name: spec.name,
            target_hz: target,
            detected_hz: None,
            cents_off: None,
            direction: None,
            confidence: 0.0,
        };

        if silent {
            strings.push(result);
            continue;
        }

        let filtered = bandpass(buffer, cfg.sample_rate_hz, low, high);
        // Skip the first half (filter warmup) before YIN.
        let start = if filtered.len() >= 4 {
            filtered.len() / 2
        } else {
            0
        };
        let analysis = &filtered[start..];

        let yin_cfg = YinConfig {
            sample_rate_hz: cfg.sample_rate_hz,
            min_hz: low * 0.9,
            max_hz: high * 1.1,
            threshold: cfg.yin_threshold,
        };

        if let Some(est) = yin(analysis, yin_cfg) {
            let periodicity = (1.0 - (est.aperiodicity / 0.6).min(1.0)).max(0.0);
            let band_ratio = (est.rms / total_rms / 0.10).clamp(0.0, 1.0);
            let confidence = periodicity * band_ratio;
            result.confidence = confidence;

            if confidence >= cfg.min_confidence {
                let cents = ratio_to_cents(est.frequency_hz, target);
                result.detected_hz = Some(est.frequency_hz);
                result.cents_off = Some(cents);
                result.direction = Some(classify(cents, cfg.in_tune_window_cents));
            }
        }

        strings.push(result);
    }

    StrumReport { strings }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::synth_sine;
    use crate::tunings::GUITAR_STANDARD;

    fn cfg() -> StrumConfig {
        StrumConfig::default()
    }

    fn mix(freqs: &[f32], n: usize) -> Vec<f32> {
        let mut buf = alloc::vec![0.0_f32; n];
        for &f in freqs {
            let s = synth_sine(f, 48_000, n, 0.5);
            for (b, x) in buf.iter_mut().zip(s.iter()) {
                *b += *x;
            }
        }
        buf
    }

    #[test]
    fn analyse_returns_one_per_string() {
        let buf = mix(&[110.0], 48_000);
        let r = analyse_strum(&buf, &GUITAR_STANDARD, cfg());
        assert_eq!(r.strings.len(), GUITAR_STANDARD.strings.len());
    }

    #[test]
    fn silence_produces_no_detections() {
        let buf = alloc::vec![0.0_f32; 48_000];
        let r = analyse_strum(&buf, &GUITAR_STANDARD, cfg());
        assert_eq!(r.detected_count(), 0);
    }

    #[test]
    fn perfectly_tuned_sines_per_string_are_in_tune() {
        let freqs: Vec<f32> = GUITAR_STANDARD
            .strings
            .iter()
            .map(|s| s.freq_hz(440.0))
            .collect();
        let buf = mix(&freqs, 48_000);
        let r = analyse_strum(&buf, &GUITAR_STANDARD, cfg());
        assert!(
            r.in_tune_count() >= 5,
            "only {} in tune of 6",
            r.in_tune_count()
        );
    }

    #[test]
    fn detuned_string_is_reported_in_correct_direction() {
        // D3 (string index 2) sharp by 20 cents, the rest in tune.
        let mut freqs: Vec<f32> = GUITAR_STANDARD
            .strings
            .iter()
            .map(|s| s.freq_hz(440.0))
            .collect();
        freqs[2] *= 2.0_f32.powf(20.0 / 1200.0);
        let buf = mix(&freqs, 48_000);
        let r = analyse_strum(&buf, &GUITAR_STANDARD, cfg());
        let d3 = &r.strings[2];
        assert_eq!(d3.direction, Some(Direction::Sharp));
        let cents = d3.cents_off.unwrap();
        assert!((cents - 20.0).abs() < 5.0, "D3 cents {cents}");
    }

    #[test]
    fn biquad_coeffs_have_expected_signs() {
        let bq = Biquad::bandpass(110.0, 5.0, 48_000.0);
        assert!(bq.b0 > 0.0);
        assert!(bq.b2 < 0.0);
        assert!((bq.b1).abs() < 1e-9);
    }

    #[test]
    fn bandpass_passes_centre_attenuates_far() {
        let centre = synth_sine(110.0, 48_000, 8192, 0.5);
        let far = synth_sine(440.0, 48_000, 8192, 0.5);
        let fc = bandpass(&centre, 48_000, 100.0, 121.0);
        let ff = bandpass(&far, 48_000, 100.0, 121.0);
        let rc = rms_of(&fc[4096..]);
        let rf = rms_of(&ff[4096..]);
        assert!(rc > rf * 5.0, "centre {rc} not >> far {rf}");
    }

    #[test]
    fn noise_floor_estimate_on_zero_buffer_is_zero() {
        assert_eq!(estimate_noise_floor(&[0.0_f32; 100]), 0.0);
    }
}

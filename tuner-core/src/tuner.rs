//! The [`Tuner`] facade: owns the audio buffers and produces snapshots,
//! strum reports, and chord recognitions.
//!
//! Two ring buffers are kept. The short `ring` (one frame) drives the
//! per-frame YIN pitch readout. A longer `analysis_buf` (~1.5 s) backs strum
//! and chord analysis, which need low frequencies to settle through the
//! band-pass cascade — the short ring is far too short for that (HANDOFF.md §6.3).

use crate::cents::{classify, Direction};
use crate::chord::{recognise, RecognitionResult};
use crate::chroma::compute_chroma;
use crate::fft::{magnitude_spectrum, HannWindow};
use crate::noise::{DcBlocker, NoiseFloor};
use crate::pitch::{yin, YinConfig};
use crate::strum::{analyse_strum, StrumConfig, StrumReport};
use crate::tunings::{self, closest_string, Tuning};
use crate::{TunerConfig, TunerError};
use alloc::vec;
use alloc::vec::Vec;

/// An immutable snapshot of the latest per-frame analysis.
#[derive(Debug, Clone)]
pub struct TunerSnapshot {
    /// Detected pitch in Hz, if any.
    pub pitch_hz: Option<f32>,
    /// Cents offset from the nearest string, if a pitch was detected.
    pub cents_off: Option<f32>,
    /// Tuning direction, if a pitch was detected.
    pub direction: Option<Direction>,
    /// Index of the nearest string in the active tuning.
    pub nearest_string: Option<usize>,
    /// Name of the nearest string.
    pub nearest_string_name: Option<&'static str>,
    /// Confidence in `[0, 1]`.
    pub confidence: f32,
}

impl TunerSnapshot {
    /// The empty snapshot (no pitch, zero confidence).
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            pitch_hz: None,
            cents_off: None,
            direction: None,
            nearest_string: None,
            nearest_string_name: None,
            confidence: 0.0,
        }
    }
}

/// Real-time tuner over a single audio stream.
pub struct Tuner {
    cfg: TunerConfig,
    tuning: &'static Tuning,
    ring: Vec<f32>,
    write_idx: usize,
    samples_since_hop: usize,
    analysis_buf: Vec<f32>,
    analysis_write_idx: usize,
    analysis_filled: bool,
    dc_blocker: DcBlocker,
    noise_floor: NoiseFloor,
    window: HannWindow,
    latest: TunerSnapshot,
}

impl Tuner {
    /// Create a tuner from a validated configuration.
    ///
    /// # Errors
    /// Returns [`TunerError`] if the config is invalid or the tuning id unknown.
    pub fn new(cfg: TunerConfig) -> Result<Self, TunerError> {
        cfg.validate()?;
        let tuning = tunings::lookup(cfg.active_tuning_id).ok_or(TunerError::UnknownTuning)?;
        let analysis_len = ((cfg.sample_rate_hz as usize * 3) / 2).max(cfg.frame_size * 4);
        let window = HannWindow::new(cfg.frame_size);
        Ok(Self {
            ring: vec![0.0_f32; cfg.frame_size],
            write_idx: 0,
            samples_since_hop: 0,
            analysis_buf: vec![0.0_f32; analysis_len],
            analysis_write_idx: 0,
            analysis_filled: false,
            dc_blocker: DcBlocker::new(),
            noise_floor: NoiseFloor::new(0.05),
            window,
            latest: TunerSnapshot::empty(),
            cfg,
            tuning,
        })
    }

    /// Switch the active tuning.
    ///
    /// # Errors
    /// Returns [`TunerError::UnknownTuning`] if `tuning_id` is not shipped.
    pub fn set_tuning(&mut self, tuning_id: &str) -> Result<(), TunerError> {
        self.tuning = tunings::lookup(tuning_id).ok_or(TunerError::UnknownTuning)?;
        Ok(())
    }

    /// The active tuning.
    #[must_use]
    pub const fn active_tuning(&self) -> &'static Tuning {
        self.tuning
    }

    /// The current configuration.
    #[must_use]
    pub const fn config(&self) -> &TunerConfig {
        &self.cfg
    }

    /// Push samples into both buffers (through the DC blocker), running a frame
    /// analysis every `hop_size` samples.
    pub fn push_samples(&mut self, samples: &[f32]) {
        for &raw in samples {
            let x = self.dc_blocker.process_sample(raw);

            self.ring[self.write_idx] = x;
            self.write_idx = (self.write_idx + 1) % self.ring.len();

            self.analysis_buf[self.analysis_write_idx] = x;
            self.analysis_write_idx = (self.analysis_write_idx + 1) % self.analysis_buf.len();
            if self.analysis_write_idx == 0 {
                self.analysis_filled = true;
            }

            self.samples_since_hop += 1;
            if self.samples_since_hop >= self.cfg.hop_size {
                self.samples_since_hop = 0;
                self.analyse_frame();
            }
        }
    }

    /// The latest per-frame snapshot.
    #[must_use]
    pub fn snapshot(&self) -> TunerSnapshot {
        self.latest.clone()
    }

    /// Ordered (oldest → newest) copy of the short ring.
    fn ring_linear(&self) -> Vec<f32> {
        let mut out = Vec::with_capacity(self.ring.len());
        out.extend_from_slice(&self.ring[self.write_idx..]);
        out.extend_from_slice(&self.ring[..self.write_idx]);
        out
    }

    /// Ordered (oldest → newest) copy of the valid portion of the analysis buffer.
    fn analysis_linear(&self) -> Vec<f32> {
        if self.analysis_filled {
            let mut out = Vec::with_capacity(self.analysis_buf.len());
            out.extend_from_slice(&self.analysis_buf[self.analysis_write_idx..]);
            out.extend_from_slice(&self.analysis_buf[..self.analysis_write_idx]);
            out
        } else {
            self.analysis_buf[..self.analysis_write_idx].to_vec()
        }
    }

    fn analyse_frame(&mut self) {
        let buf = self.ring_linear();
        let rms = rms_of(&buf);

        if rms < 1e-4 {
            self.noise_floor.update_if_quiet(rms);
            self.latest = TunerSnapshot::empty();
            return;
        }

        let lowest = self.tuning.strings[0].freq_hz(self.cfg.a4_hz) * 0.85;
        let highest = self.tuning.strings.last().unwrap().freq_hz(self.cfg.a4_hz) * 1.2;

        let yin_cfg = YinConfig {
            sample_rate_hz: self.cfg.sample_rate_hz,
            min_hz: lowest,
            max_hz: highest,
            threshold: self.cfg.yin_threshold,
        };

        match yin(&buf, yin_cfg) {
            Some(est) => {
                let periodicity = (1.0 - (est.aperiodicity / 0.6).min(1.0)).max(0.0);
                let signal_factor = (rms / 1e-3).clamp(0.0, 1.0);
                let conf = periodicity * signal_factor;

                if conf < 0.10 {
                    self.latest = TunerSnapshot {
                        confidence: conf,
                        ..TunerSnapshot::empty()
                    };
                    return;
                }

                let Some((idx, cents)) =
                    closest_string(self.tuning, est.frequency_hz, self.cfg.a4_hz)
                else {
                    self.latest = TunerSnapshot::empty();
                    return;
                };
                self.latest = TunerSnapshot {
                    pitch_hz: Some(est.frequency_hz),
                    cents_off: Some(cents),
                    direction: Some(classify(cents, 5.0)),
                    nearest_string: Some(idx),
                    nearest_string_name: Some(self.tuning.strings[idx].name),
                    confidence: conf,
                };
            }
            None => {
                self.noise_floor.update_if_quiet(rms);
                self.latest = TunerSnapshot::empty();
            }
        }
    }

    /// Analyse the long buffer as a strum.
    #[must_use]
    pub fn analyse_strum(&self) -> StrumReport {
        let buf = self.analysis_linear();
        let cfg = StrumConfig {
            sample_rate_hz: self.cfg.sample_rate_hz,
            a4_hz: self.cfg.a4_hz,
            ..StrumConfig::default()
        };
        analyse_strum(&buf, self.tuning, cfg)
    }

    /// Recognise a chord from the most recent `frame_size` samples of the long buffer.
    #[must_use]
    pub fn recognise_chord(&self) -> RecognitionResult {
        let buf = self.analysis_linear();
        let n = self.cfg.frame_size;
        let frame: &[f32] = if buf.len() >= n {
            &buf[buf.len() - n..]
        } else {
            &buf
        };
        if frame.len() != n {
            // Not enough data yet for an FFT frame.
            return RecognitionResult {
                candidates: Vec::new(),
                best: None,
            };
        }
        let mag = magnitude_spectrum(frame, &self.window);
        let chroma = compute_chroma(
            &mag,
            self.cfg.sample_rate_hz,
            n,
            70.0,
            5000.0,
            self.cfg.a4_hz,
        );
        recognise(&chroma, self.cfg.chord_min_score, self.cfg.chord_min_margin)
    }
}

#[inline]
fn rms_of(buf: &[f32]) -> f32 {
    if buf.is_empty() {
        return 0.0;
    }
    let s: f32 = buf.iter().map(|&x| x * x).sum();
    #[cfg(feature = "std")]
    {
        (s / buf.len() as f32).sqrt()
    }
    #[cfg(not(feature = "std"))]
    {
        libm::sqrtf(s / buf.len() as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::synth_sine;

    fn default_tuner() -> Tuner {
        Tuner::new(TunerConfig::default()).unwrap()
    }

    #[test]
    fn new_with_default_config_succeeds() {
        assert!(Tuner::new(TunerConfig::default()).is_ok());
    }

    #[test]
    fn new_rejects_unknown_tuning() {
        let cfg = TunerConfig {
            active_tuning_id: "does.not.exist",
            ..TunerConfig::default()
        };
        assert_eq!(Tuner::new(cfg).err(), Some(TunerError::UnknownTuning));
    }

    #[test]
    fn snapshot_after_silence_is_empty() {
        let mut t = default_tuner();
        t.push_samples(&vec![0.0_f32; 8192]);
        assert!(t.snapshot().pitch_hz.is_none());
    }

    #[test]
    fn detects_pitch_within_active_tuning_range() {
        let mut t = default_tuner();
        // G3 = 196 Hz, string index 3 of guitar.standard.
        let sine = synth_sine(196.0, 48_000, 16384, 0.5);
        t.push_samples(&sine);
        let snap = t.snapshot();
        let hz = snap.pitch_hz.expect("should detect a pitch");
        assert!((hz - 196.0).abs() < 1.0, "got {hz} Hz");
        assert_eq!(snap.nearest_string_name, Some("G3"));
    }

    #[test]
    fn detects_e2_for_guitar_low_string() {
        let mut t = default_tuner();
        let sine = synth_sine(82.4069, 48_000, 16384, 0.5);
        t.push_samples(&sine);
        let snap = t.snapshot();
        assert_eq!(snap.nearest_string_name, Some("E2"));
        assert!(snap.cents_off.unwrap().abs() < 5.0);
    }

    #[test]
    fn set_tuning_changes_active() {
        let mut t = default_tuner();
        assert_eq!(t.active_tuning().id, "guitar.standard");
        t.set_tuning("bass.standard").unwrap();
        assert_eq!(t.active_tuning().id, "bass.standard");
        assert!(t.set_tuning("nope").is_err());
    }

    #[test]
    fn analyse_strum_returns_one_per_string() {
        let mut t = default_tuner();
        let sine = synth_sine(110.0, 48_000, 48_000, 0.5);
        t.push_samples(&sine);
        let r = t.analyse_strum();
        assert_eq!(r.strings.len(), t.active_tuning().strings.len());
    }

    #[test]
    fn recognise_chord_on_silence_returns_no_best() {
        let mut t = default_tuner();
        t.push_samples(&vec![0.0_f32; 16384]);
        assert!(t.recognise_chord().best.is_none());
    }
}

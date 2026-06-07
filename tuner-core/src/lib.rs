//! # tuner-core
//!
//! Real-time pitch detection, strum analysis, and chord recognition for
//! guitar, bass, and guitarra portuguesa.
//!
//! ## Quick start
//! ```
//! use tuner_core::{Tuner, TunerConfig};
//! let mut tuner = Tuner::new(TunerConfig::default()).unwrap();
//! let silence = [0.0_f32; 4096];
//! tuner.push_samples(&silence);
//! let snapshot = tuner.snapshot();
//! assert!(snapshot.pitch_hz.is_none());
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(
    missing_docs,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    rust_2018_idioms,
    unreachable_pub
)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::module_name_repetitions,
    clippy::multiple_crate_versions,
    clippy::similar_names,
    clippy::suboptimal_flops,
    clippy::single_match_else,
    clippy::items_after_statements,
    clippy::option_if_let_else,
    clippy::if_not_else,
    clippy::doc_markdown,
    clippy::cast_lossless,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::float_cmp
)]

extern crate alloc;

pub mod cents;
pub mod chord;
pub mod chroma;
pub mod fft;
pub mod noise;
pub mod pitch;
pub mod strum;
pub mod tunings;

mod tuner;
pub use tuner::{Tuner, TunerSnapshot};

#[cfg(any(feature = "jni", feature = "wasm"))]
pub mod bindings;

use core::fmt;

/// Errors returned by the tuner core.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TunerError {
    /// A [`TunerConfig`] field was out of range. The string explains which.
    InvalidConfig(&'static str),
    /// The requested tuning id is not one of the shipped tunings.
    UnknownTuning,
    /// A supplied buffer was shorter than the algorithm requires.
    BufferTooShort {
        /// Length actually provided.
        got: usize,
        /// Minimum length required.
        required: usize,
    },
}

impl fmt::Display for TunerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(why) => write!(f, "invalid configuration: {why}"),
            Self::UnknownTuning => write!(f, "unknown tuning id"),
            Self::BufferTooShort { got, required } => {
                write!(f, "buffer too short: got {got} samples, need {required}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TunerError {}

/// Runtime configuration for a [`Tuner`].
#[derive(Debug, Clone, PartialEq)]
pub struct TunerConfig {
    /// Audio sample rate in Hz (default `48_000`).
    pub sample_rate_hz: u32,
    /// Analysis frame length in samples; must be a power of two (default `4096`).
    pub frame_size: usize,
    /// Hop between frames in samples (default `2048`).
    pub hop_size: usize,
    /// Reference pitch for A4 in Hz (default `440.0`).
    pub a4_hz: f32,
    /// YIN aperiodicity threshold (default `0.12`).
    pub yin_threshold: f32,
    /// Minimum cosine score for a confident chord match (default `0.85`).
    pub chord_min_score: f32,
    /// Minimum score margin over the runner-up for a confident chord (default `0.05`).
    pub chord_min_margin: f32,
    /// Id of the active tuning (default `"guitar.standard"`).
    pub active_tuning_id: &'static str,
    /// Whether to subtract the tracked noise floor when gating (default `true`).
    pub noise_subtraction: bool,
}

impl Default for TunerConfig {
    fn default() -> Self {
        Self {
            sample_rate_hz: 48_000,
            frame_size: 4096,
            hop_size: 2048,
            a4_hz: 440.0,
            yin_threshold: 0.12,
            chord_min_score: 0.85,
            chord_min_margin: 0.05,
            active_tuning_id: "guitar.standard",
            noise_subtraction: true,
        }
    }
}

impl TunerConfig {
    /// Validate the configuration.
    ///
    /// Not a `const fn`: float comparisons are not permitted in const contexts
    /// on the crate's MSRV (Rust 1.75).
    pub fn validate(&self) -> Result<(), TunerError> {
        if self.sample_rate_hz == 0 {
            return Err(TunerError::InvalidConfig("sample_rate_hz must be > 0"));
        }
        if self.frame_size == 0 || !self.frame_size.is_power_of_two() {
            return Err(TunerError::InvalidConfig(
                "frame_size must be a non-zero power of two",
            ));
        }
        if self.hop_size == 0 || self.hop_size > self.frame_size {
            return Err(TunerError::InvalidConfig(
                "hop_size must be in 1..=frame_size",
            ));
        }
        if !(self.a4_hz > 0.0 && self.a4_hz < 10_000.0) {
            return Err(TunerError::InvalidConfig("a4_hz must be in (0, 10_000)"));
        }
        if !(self.yin_threshold > 0.0 && self.yin_threshold < 1.0) {
            return Err(TunerError::InvalidConfig("yin_threshold must be in (0, 1)"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn default_config_is_valid() {
        assert!(TunerConfig::default().validate().is_ok());
    }

    #[test]
    fn rejects_zero_sample_rate() {
        let cfg = TunerConfig {
            sample_rate_hz: 0,
            ..TunerConfig::default()
        };
        assert!(matches!(cfg.validate(), Err(TunerError::InvalidConfig(_))));
    }

    #[test]
    fn rejects_non_power_of_two_frame() {
        let cfg = TunerConfig {
            frame_size: 4000,
            ..TunerConfig::default()
        };
        assert!(matches!(cfg.validate(), Err(TunerError::InvalidConfig(_))));
    }

    #[test]
    fn rejects_hop_larger_than_frame() {
        let cfg = TunerConfig {
            hop_size: 8192,
            ..TunerConfig::default()
        };
        assert!(matches!(cfg.validate(), Err(TunerError::InvalidConfig(_))));
    }

    #[test]
    fn rejects_out_of_range_a4() {
        let cfg = TunerConfig {
            a4_hz: 0.0,
            ..TunerConfig::default()
        };
        assert!(matches!(cfg.validate(), Err(TunerError::InvalidConfig(_))));
        let cfg = TunerConfig {
            a4_hz: 20_000.0,
            ..TunerConfig::default()
        };
        assert!(matches!(cfg.validate(), Err(TunerError::InvalidConfig(_))));
    }

    #[test]
    fn error_display_is_human_readable() {
        assert!(TunerError::UnknownTuning.to_string().contains("unknown"));
        assert!(TunerError::InvalidConfig("x").to_string().contains('x'));
        assert!(TunerError::BufferTooShort {
            got: 10,
            required: 20,
        }
        .to_string()
        .contains("20"));
    }
}

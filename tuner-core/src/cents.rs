//! Cents / MIDI math.
//!
//! All conversions assume twelve-tone equal temperament with a configurable
//! A4 reference frequency. The functions here are pure and `no_std`-friendly.

#[cfg(not(feature = "std"))]
use libm::{log2f, powf};

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

/// Difference between a measured and a target frequency, in cents.
///
/// Returns `0.0` if either input is non-positive (silence / invalid).
///
/// Formula: `1200 * log2(measured / target)`.
#[must_use]
pub fn ratio_to_cents(measured_hz: f32, target_hz: f32) -> f32 {
    if measured_hz <= 0.0 || target_hz <= 0.0 {
        return 0.0;
    }
    1200.0 * log2f(measured_hz / target_hz)
}

/// Convert a MIDI note number to its frequency in Hz.
#[must_use]
pub fn midi_to_hz(midi: u8, a4_hz: f32) -> f32 {
    a4_hz * powf(2.0, (f32::from(midi) - 69.0) / 12.0)
}

/// Convert a frequency to a (fractional) MIDI note number.
///
/// Returns `None` for non-positive input.
#[must_use]
pub fn hz_to_midi(hz: f32, a4_hz: f32) -> Option<f32> {
    if hz <= 0.0 || a4_hz <= 0.0 {
        return None;
    }
    Some(69.0 + 12.0 * log2f(hz / a4_hz))
}

/// Pitch class (0 = C … 11 = B) of a frequency, or `None` for non-positive input.
#[must_use]
pub fn pitch_class(hz: f32, a4_hz: f32) -> Option<u8> {
    let midi = hz_to_midi(hz, a4_hz)?;
    Some((midi.round() as i32).rem_euclid(12) as u8)
}

/// Tuning direction relative to a target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Measured pitch is below the target.
    Flat,
    /// Measured pitch is within the in-tune window.
    InTune,
    /// Measured pitch is above the target.
    Sharp,
}

/// Classify a cents offset into a [`Direction`] given a symmetric in-tune window.
#[must_use]
pub fn classify(cents: f32, in_tune_window_cents: f32) -> Direction {
    if cents.abs() <= in_tune_window_cents {
        Direction::InTune
    } else if cents < 0.0 {
        Direction::Flat
    } else {
        Direction::Sharp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn unison_is_zero_cents() {
        assert_relative_eq!(ratio_to_cents(440.0, 440.0), 0.0, epsilon = 1e-4);
    }

    #[test]
    fn octave_is_1200_cents() {
        assert_relative_eq!(ratio_to_cents(880.0, 440.0), 1200.0, epsilon = 1e-2);
        assert_relative_eq!(ratio_to_cents(220.0, 440.0), -1200.0, epsilon = 1e-2);
    }

    #[test]
    fn semitone_is_100_cents() {
        let semitone = 440.0 * 2.0_f32.powf(1.0 / 12.0);
        assert_relative_eq!(ratio_to_cents(semitone, 440.0), 100.0, epsilon = 1e-2);
    }

    #[test]
    fn ratio_to_cents_handles_non_positive() {
        assert_eq!(ratio_to_cents(0.0, 440.0), 0.0);
        assert_eq!(ratio_to_cents(440.0, 0.0), 0.0);
        assert_eq!(ratio_to_cents(-1.0, 440.0), 0.0);
    }

    #[test]
    fn a4_is_midi_69() {
        assert_relative_eq!(midi_to_hz(69, 440.0), 440.0, epsilon = 1e-3);
    }

    #[test]
    fn e2_frequency() {
        // E2 = MIDI 40
        assert_relative_eq!(midi_to_hz(40, 440.0), 82.4069, epsilon = 1e-3);
    }

    #[test]
    fn hz_to_midi_inverts_midi_to_hz() {
        for m in 21u8..=96 {
            let hz = midi_to_hz(m, 440.0);
            let back = hz_to_midi(hz, 440.0).unwrap();
            assert_relative_eq!(back, f32::from(m), epsilon = 1e-2);
        }
    }

    #[test]
    fn hz_to_midi_rejects_non_positive() {
        assert!(hz_to_midi(0.0, 440.0).is_none());
        assert!(hz_to_midi(-5.0, 440.0).is_none());
    }

    #[test]
    fn pitch_class_of_a4_is_9() {
        assert_eq!(pitch_class(440.0, 440.0), Some(9));
    }

    #[test]
    fn pitch_class_of_c4_is_0() {
        // C4 = MIDI 60
        assert_eq!(pitch_class(midi_to_hz(60, 440.0), 440.0), Some(0));
    }

    #[test]
    fn classify_windows() {
        assert_eq!(classify(0.0, 5.0), Direction::InTune);
        assert_eq!(classify(3.0, 5.0), Direction::InTune);
        assert_eq!(classify(-3.0, 5.0), Direction::InTune);
        assert_eq!(classify(10.0, 5.0), Direction::Sharp);
        assert_eq!(classify(-10.0, 5.0), Direction::Flat);
    }

    // ---- property tests ----
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn cents_self_is_zero(hz in 20.0f32..5000.0) {
            prop_assert!(ratio_to_cents(hz, hz).abs() < 1e-2);
        }

        #[test]
        fn cents_octave_invariant(hz in 20.0f32..2500.0) {
            // Doubling always adds ~1200 cents.
            let c = ratio_to_cents(hz * 2.0, hz);
            prop_assert!((c - 1200.0).abs() < 1e-1);
        }

        #[test]
        fn midi_round_trip_property(m in 21u8..=108) {
            let hz = midi_to_hz(m, 440.0);
            let back = hz_to_midi(hz, 440.0).unwrap();
            prop_assert!((back - f32::from(m)).abs() < 1e-2);
        }

        #[test]
        fn cents_is_antisymmetric(a in 20.0f32..5000.0, b in 20.0f32..5000.0) {
            let ab = ratio_to_cents(a, b);
            let ba = ratio_to_cents(b, a);
            prop_assert!((ab + ba).abs() < 1e-1);
        }
    }
}

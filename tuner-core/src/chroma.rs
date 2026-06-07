//! Log-magnitude chroma with triangular pitch-class interpolation.
//!
//! Müller (2007), *Information Retrieval for Music and Motion*. Each FFT bin's
//! log-magnitude energy is split between the two pitch classes it falls
//! between, weighted by fractional distance — **not** rounded to the nearest
//! semitone. The rounding approach put energy on the wrong side of a
//! pitch-class boundary when a partial fell between two bins straddling a
//! semitone line (see HANDOFF.md §6.1); triangular interpolation fixes it.

use crate::cents::hz_to_midi;
use crate::fft::bin_to_hz;

#[cfg(not(feature = "std"))]
use libm::{log1pf, sqrtf};

#[cfg(feature = "std")]
#[inline]
fn log1pf(x: f32) -> f32 {
    x.ln_1p()
}
#[cfg(feature = "std")]
#[inline]
fn sqrtf(x: f32) -> f32 {
    x.sqrt()
}

/// A 12-element pitch-class energy profile (index 0 = C … 11 = B).
pub type ChromaVector = [f32; 12];

/// Compute a chroma vector from a magnitude spectrum.
///
/// Bins outside `[min_hz, max_hz]` (or with non-positive magnitude) are ignored.
#[must_use]
pub fn compute_chroma(
    magnitudes: &[f32],
    sample_rate_hz: u32,
    fft_len: usize,
    min_hz: f32,
    max_hz: f32,
    a4_hz: f32,
) -> ChromaVector {
    let mut chroma = [0.0_f32; 12];
    for (bin, &mag) in magnitudes.iter().enumerate() {
        let hz = bin_to_hz(bin, sample_rate_hz, fft_len);
        if hz < min_hz || hz > max_hz || mag <= 0.0 {
            continue;
        }
        let Some(midi) = hz_to_midi(hz, a4_hz) else {
            continue;
        };
        let weight = log1pf(mag); // ln(1 + m) for numerical stability
        let pc_floor = midi.floor();
        let frac = midi - pc_floor;
        let lower = (pc_floor as i32).rem_euclid(12) as usize;
        let upper = (lower + 1) % 12;
        chroma[lower] += weight * (1.0 - frac);
        chroma[upper] += weight * frac;
    }
    normalise(&mut chroma);
    chroma
}

/// Normalise a chroma vector to unit sum (no-op for the zero vector).
pub fn normalise(chroma: &mut ChromaVector) {
    let sum: f32 = chroma.iter().sum();
    if sum > 0.0 {
        for c in chroma.iter_mut() {
            *c /= sum;
        }
    }
}

/// Cosine similarity of two chroma vectors, in `[0, 1]` for non-negative input.
/// Returns `0.0` if either vector is all zeros.
#[must_use]
pub fn cosine_similarity(a: &ChromaVector, b: &ChromaVector) -> f32 {
    let mut dot = 0.0_f32;
    let mut na = 0.0_f32;
    let mut nb = 0.0_f32;
    for i in 0..12 {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na <= 0.0 || nb <= 0.0 {
        return 0.0;
    }
    dot / (sqrtf(na) * sqrtf(nb))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fft::{magnitude_spectrum, HannWindow};
    use crate::pitch::synth_sine;
    use approx::assert_relative_eq;

    fn chroma_of_sine(freq: f32) -> ChromaVector {
        let fft_len = 4096;
        let sr = 48_000;
        let buf = synth_sine(freq, sr, fft_len, 1.0);
        let w = HannWindow::new(fft_len);
        let mag = magnitude_spectrum(&buf, &w);
        compute_chroma(&mag, sr, fft_len, 70.0, 5000.0, 440.0)
    }

    fn peak_pc(c: &ChromaVector) -> usize {
        c.iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0
    }

    #[test]
    fn chroma_sums_to_one_on_real_input() {
        let c = chroma_of_sine(440.0);
        let sum: f32 = c.iter().sum();
        assert_relative_eq!(sum, 1.0, epsilon = 1e-4);
    }

    #[test]
    fn chroma_of_a440_peaks_at_pc_9() {
        assert_eq!(peak_pc(&chroma_of_sine(440.0)), 9);
    }

    #[test]
    fn chroma_of_c4_peaks_at_pc_0() {
        // Regression test for the triangular-interpolation bug (§6.1).
        // C4 = 261.63 Hz must land on C (pc 0), not C# (pc 1).
        assert_eq!(peak_pc(&chroma_of_sine(261.6256)), 0);
    }

    #[test]
    fn silence_is_zero() {
        let c = compute_chroma(&[0.0_f32; 2049], 48_000, 4096, 70.0, 5000.0, 440.0);
        assert_eq!(c.iter().sum::<f32>(), 0.0);
    }

    #[test]
    fn cosine_self_is_one() {
        let c = chroma_of_sine(440.0);
        assert_relative_eq!(cosine_similarity(&c, &c), 1.0, epsilon = 1e-5);
    }

    #[test]
    fn cosine_orthogonal_is_zero() {
        let mut a = [0.0_f32; 12];
        let mut b = [0.0_f32; 12];
        a[0] = 1.0;
        b[6] = 1.0;
        assert_relative_eq!(cosine_similarity(&a, &b), 0.0, epsilon = 1e-6);
    }

    #[test]
    fn cosine_zero_vector_is_zero() {
        let z = [0.0_f32; 12];
        let mut a = [0.0_f32; 12];
        a[0] = 1.0;
        assert_eq!(cosine_similarity(&z, &a), 0.0);
    }

    #[test]
    fn normalise_zero_vector_is_noop() {
        let mut z = [0.0_f32; 12];
        normalise(&mut z);
        assert_eq!(z, [0.0_f32; 12]);
    }
}

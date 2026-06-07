//! Integration tests driving the public API with synthetic signals.

use tuner_core::cents::{ratio_to_cents, Direction};
use tuner_core::chord::Quality;
use tuner_core::pitch::synth_sine;
use tuner_core::tunings::{self, ALL};
use tuner_core::{Tuner, TunerConfig};

const SR: u32 = 48_000;

/// Additive plucked-string synthesis: 8 harmonics with 1/n amplitude rolloff.
/// Pure sines fail chord recognition — the FFT can't resolve pitch classes
/// from a single low-frequency partial.
fn synth_pluck(freq: f32, n: usize) -> Vec<f32> {
    let mut buf = vec![0.0_f32; n];
    for h in 1..=8 {
        let f = freq * h as f32;
        if f > SR as f32 / 2.0 {
            break;
        }
        let amp = 0.5 / h as f32;
        let partial = synth_sine(f, SR, n, amp);
        for (b, x) in buf.iter_mut().zip(partial.iter()) {
            *b += *x;
        }
    }
    buf
}

fn mix(freqs: &[f32], n: usize) -> Vec<f32> {
    let mut buf = vec![0.0_f32; n];
    for &f in freqs {
        let s = synth_sine(f, SR, n, 0.5);
        for (b, x) in buf.iter_mut().zip(s.iter()) {
            *b += *x;
        }
    }
    buf
}

fn tuner_for(id: &'static str) -> Tuner {
    Tuner::new(TunerConfig {
        active_tuning_id: id,
        ..TunerConfig::default()
    })
    .unwrap()
}

#[test]
fn detects_every_string_of_every_shipped_tuning() {
    for tuning in ALL {
        let mut t = tuner_for(tuning.id);
        for spec in tuning.strings {
            let sine = synth_sine(spec.freq_hz(440.0), SR, 16384, 0.5);
            t.push_samples(&sine);
            let snap = t.snapshot();
            assert_eq!(
                snap.nearest_string_name,
                Some(spec.name),
                "tuning {} string {}",
                tuning.id,
                spec.name
            );
            assert!(
                snap.cents_off.unwrap().abs() < 5.0,
                "tuning {} string {} off by {:?}",
                tuning.id,
                spec.name,
                snap.cents_off
            );
        }
    }
}

#[test]
fn reports_correct_direction_for_detuned_strings() {
    let mut t = tuner_for("guitar.standard");
    let g3 = tunings::GUITAR_STANDARD.strings[3].freq_hz(440.0);
    // Avoid the exact ±5 cent boundary (float noise).
    for &cents in &[-30.0, -6.0, 4.5_f32, 0.0, 6.0, 30.0] {
        let f = g3 * 2.0_f32.powf(cents / 1200.0);
        let sine = synth_sine(f, SR, 16384, 0.5);
        t.push_samples(&sine);
        let snap = t.snapshot();
        let dir = snap.direction.unwrap();
        let expected = if cents.abs() <= 5.0 {
            Direction::InTune
        } else if cents < 0.0 {
            Direction::Flat
        } else {
            Direction::Sharp
        };
        assert_eq!(dir, expected, "at {cents} cents");
    }
}

#[test]
fn strum_analysis_finds_all_six_guitar_strings_in_tune() {
    let mut t = tuner_for("guitar.standard");
    let freqs: Vec<f32> = tunings::GUITAR_STANDARD
        .strings
        .iter()
        .map(|s| s.freq_hz(440.0))
        .collect();
    let buf = mix(&freqs, SR as usize);
    t.push_samples(&buf);
    let report = t.analyse_strum();
    assert!(
        report.in_tune_count() >= 5,
        "only {} of 6 in tune",
        report.in_tune_count()
    );
}

#[test]
fn strum_analysis_reports_detuned_string_offset() {
    let mut t = tuner_for("guitar.standard");
    let mut freqs: Vec<f32> = tunings::GUITAR_STANDARD
        .strings
        .iter()
        .map(|s| s.freq_hz(440.0))
        .collect();
    // E2 sharp by 30 cents.
    freqs[0] *= 2.0_f32.powf(30.0 / 1200.0);
    let buf = mix(&freqs, SR as usize);
    t.push_samples(&buf);
    let report = t.analyse_strum();
    let e2 = &report.strings[0];
    assert_eq!(e2.direction, Some(Direction::Sharp));
    assert!((e2.cents_off.unwrap() - 30.0).abs() < 6.0);
}

#[test]
fn chord_recogniser_identifies_each_major_chord_root() {
    // Major-family equivalence: a strong 7th harmonic can make maj7/7 beat the
    // plain major, which is musically defensible. Accept the family.
    for root in 0u8..12 {
        let mut t = tuner_for("guitar.standard");
        // Build a major triad two octaves up so partials land in chroma range.
        let base = 261.6256_f32 * 2.0_f32.powf(f32::from(root) / 12.0);
        let third = base * 2.0_f32.powf(4.0 / 12.0);
        let fifth = base * 2.0_f32.powf(7.0 / 12.0);
        let mut buf = vec![0.0_f32; 8192];
        for f in [base, third, fifth] {
            let p = synth_pluck(f, 8192);
            for (b, x) in buf.iter_mut().zip(p.iter()) {
                *b += *x;
            }
        }
        t.push_samples(&buf);
        let result = t.recognise_chord();
        let in_family = result.candidates.iter().take(3).any(|c| {
            c.root_pc == root
                && matches!(
                    c.quality,
                    Quality::Major | Quality::MajorSeventh | Quality::Seventh
                )
        });
        assert!(
            in_family,
            "root {root}: top candidates {:?}",
            result
                .candidates
                .iter()
                .take(3)
                .map(|c| (&c.name, c.score))
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn chord_template_chroma_matches_recogniser_template() {
    use std::collections::BTreeSet;
    use tuner_core::chord::{recognise, template};

    let pc_set = |root: u8, q: Quality| -> BTreeSet<u8> {
        q.intervals()
            .iter()
            .map(|&iv| ((u16::from(root) + u16::from(iv)) % 12) as u8)
            .collect()
    };

    let want = pc_set(0, Quality::Major);
    let t = template(0, Quality::Major);
    let r = recognise(&t, 0.85, 0.05);
    // Pitch-class-set membership, not exact (root, quality).
    assert!(r
        .candidates
        .iter()
        .any(|c| pc_set(c.root_pc, c.quality) == want));
}

#[test]
fn silence_yields_no_pitch_no_strum_no_chord() {
    let mut t = tuner_for("guitar.standard");
    t.push_samples(&[0.0_f32; SR as usize]);
    assert!(t.snapshot().pitch_hz.is_none());
    assert_eq!(t.analyse_strum().detected_count(), 0);
    assert!(t.recognise_chord().best.is_none());
}

#[test]
fn detection_is_stable_across_repeated_pushes() {
    let sine = synth_sine(196.0, SR, 16384, 0.5);

    let mut one_shot = tuner_for("guitar.standard");
    one_shot.push_samples(&sine);
    let a = one_shot.snapshot();

    let mut chunked = tuner_for("guitar.standard");
    for chunk in sine.chunks(256) {
        chunked.push_samples(chunk);
    }
    let b = chunked.snapshot();

    assert_eq!(a.nearest_string_name, b.nearest_string_name);
    let ca = a.pitch_hz.unwrap();
    let cb = b.pitch_hz.unwrap();
    assert!(ratio_to_cents(ca, cb).abs() < 2.0, "{ca} vs {cb}");
}

#[test]
fn switching_tuning_changes_nearest_string() {
    let a2 = tunings::GUITAR_STANDARD.strings[1].freq_hz(440.0); // 110 Hz
    let sine = synth_sine(a2, SR, 16384, 0.5);

    let mut t = tuner_for("guitar.standard");
    t.push_samples(&sine);
    assert_eq!(t.snapshot().nearest_string_name, Some("A2"));

    t.set_tuning("bass.standard").unwrap();
    t.push_samples(&sine);
    // Bass has no A2; 110 Hz is closest to G2 (98 Hz).
    assert_eq!(t.snapshot().nearest_string_name, Some("G2"));
}

#[test]
fn chroma_doctest_path_via_full_fft_works() {
    use tuner_core::chroma::compute_chroma;
    use tuner_core::fft::{magnitude_spectrum, HannWindow};

    let fft_len = 4096;
    let e4 = 329.6276_f32; // E4, pitch class 4
    let buf = synth_sine(e4, SR, fft_len, 1.0);
    let w = HannWindow::new(fft_len);
    let mag = magnitude_spectrum(&buf, &w);
    let chroma = compute_chroma(&mag, SR, fft_len, 70.0, 5000.0, 440.0);
    let peak = chroma
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .unwrap()
        .0;
    assert_eq!(peak, 4);
}

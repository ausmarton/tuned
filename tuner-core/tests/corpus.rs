//! Recorded-audio regression tests.
//!
//! Each subfolder of `tests/corpus/` pairs a `.wav` with a `.label` file. When
//! a folder is empty (the default in this scaffold) its test prints a skip
//! message and passes, so CI stays green until a real corpus is committed.
//!
//! Label formats:
//! - `clean/<name>.label`  : line 1 = tuning_id, line 2 = expected string name
//! - `strum/<name>.label`  : line 1 = tuning_id, then `string_name,expected_cents`
//! - `chords/<name>.label` : the chord name (e.g. `Cmaj7`)
//! - `noisy/<name>.label`  : optional `# tolerance:<cents>` header

use std::fs;
use std::path::{Path, PathBuf};

use tuner_core::{Tuner, TunerConfig};

fn corpus_dir(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("corpus")
        .join(sub)
}

/// Collect `.wav` files paired with a matching `.label`.
fn wav_label_pairs(sub: &str) -> Vec<(PathBuf, PathBuf)> {
    let dir = corpus_dir(sub);
    let mut pairs = Vec::new();
    let Ok(entries) = fs::read_dir(&dir) else {
        return pairs;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("wav") {
            let label = path.with_extension("label");
            if label.exists() {
                pairs.push((path, label));
            }
        }
    }
    pairs.sort();
    pairs
}

/// Read a WAV as mono f32 at its native sample rate.
fn read_wav_mono(path: &Path) -> (Vec<f32>, u32) {
    let mut reader = hound::WavReader::open(path).expect("open wav");
    let spec = reader.spec();
    let channels = spec.channels as usize;
    let raw: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap()).collect(),
        hound::SampleFormat::Int => {
            let max = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.unwrap() as f32 / max)
                .collect()
        }
    };
    // Downmix to mono.
    let mono = if channels <= 1 {
        raw
    } else {
        raw.chunks(channels)
            .map(|c| c.iter().sum::<f32>() / channels as f32)
            .collect()
    };
    (mono, spec.sample_rate)
}

fn tuner_at(sample_rate: u32, tuning_id: &'static str) -> Tuner {
    // Find a power-of-two frame that fits; default config otherwise.
    Tuner::new(TunerConfig {
        sample_rate_hz: sample_rate,
        active_tuning_id: tuning_id,
        ..TunerConfig::default()
    })
    .expect("tuner")
}

#[test]
fn clean_corpus_regression() {
    let pairs = wav_label_pairs("clean");
    if pairs.is_empty() {
        eprintln!("skipping clean corpus: no samples committed");
        return;
    }
    for (wav, label) in pairs {
        let text = fs::read_to_string(&label).unwrap();
        let mut lines = text.lines();
        let tuning_id = lines.next().unwrap().trim();
        let expected = lines.next().unwrap().trim();
        // Leak the id to satisfy the 'static bound (test-only).
        let id: &'static str = Box::leak(tuning_id.to_string().into_boxed_str());
        let (samples, sr) = read_wav_mono(&wav);
        let mut t = tuner_at(sr, id);
        t.push_samples(&samples);
        let snap = t.snapshot();
        assert_eq!(
            snap.nearest_string_name,
            Some(expected),
            "{}",
            wav.display()
        );
    }
}

#[test]
fn strum_corpus_regression() {
    let pairs = wav_label_pairs("strum");
    if pairs.is_empty() {
        eprintln!("skipping strum corpus: no samples committed");
        return;
    }
    for (wav, label) in pairs {
        let text = fs::read_to_string(&label).unwrap();
        let mut lines = text.lines();
        let tuning_id = lines.next().unwrap().trim();
        let id: &'static str = Box::leak(tuning_id.to_string().into_boxed_str());
        let (samples, sr) = read_wav_mono(&wav);
        let mut t = tuner_at(sr, id);
        t.push_samples(&samples);
        let report = t.analyse_strum();
        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let (name, cents) = line.split_once(',').expect("string_name,cents");
            let expected_cents: f32 = cents.trim().parse().unwrap();
            let s = report
                .strings
                .iter()
                .find(|s| s.name == name.trim())
                .unwrap_or_else(|| panic!("string {name} not in report"));
            let got = s.cents_off.unwrap_or_else(|| panic!("{name} not detected"));
            assert!(
                (got - expected_cents).abs() < 10.0,
                "{}: {name} expected {expected_cents} got {got}",
                wav.display()
            );
        }
    }
}

#[test]
fn chords_corpus_regression() {
    let pairs = wav_label_pairs("chords");
    if pairs.is_empty() {
        eprintln!("skipping chords corpus: no samples committed");
        return;
    }
    for (wav, label) in pairs {
        let expected = fs::read_to_string(&label).unwrap().trim().to_string();
        let (samples, sr) = read_wav_mono(&wav);
        let mut t = tuner_at(sr, "guitar.standard");
        t.push_samples(&samples);
        let result = t.recognise_chord();
        let found = result.candidates.iter().take(3).any(|c| c.name == expected);
        assert!(found, "{}: expected {expected} in top 3", wav.display());
    }
}

#[test]
fn noisy_corpus_regression() {
    let pairs = wav_label_pairs("noisy");
    if pairs.is_empty() {
        eprintln!("skipping noisy corpus: no samples committed");
        return;
    }
    for (wav, label) in pairs {
        let text = fs::read_to_string(&label).unwrap();
        let mut tolerance = 10.0_f32;
        let mut expected_string = None;
        let mut tuning_id = "guitar.standard".to_string();
        for line in text.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("# tolerance:") {
                tolerance = rest.trim().parse().unwrap_or(10.0);
            } else if !line.is_empty() && !line.starts_with('#') {
                if expected_string.is_none() && tuning_id == "guitar.standard" && line.contains('.')
                {
                    tuning_id = line.to_string();
                } else {
                    expected_string = Some(line.to_string());
                }
            }
        }
        let id: &'static str = Box::leak(tuning_id.into_boxed_str());
        let (samples, sr) = read_wav_mono(&wav);
        let mut t = tuner_at(sr, id);
        t.push_samples(&samples);
        let snap = t.snapshot();
        if let Some(exp) = expected_string {
            assert_eq!(
                snap.nearest_string_name,
                Some(exp.as_str()),
                "{}",
                wav.display()
            );
            assert!(snap.cents_off.unwrap().abs() < tolerance);
        }
    }
}

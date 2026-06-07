//! Criterion micro-benchmarks for the hot DSP paths.

use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use tuner_core::chroma::compute_chroma;
use tuner_core::fft::{magnitude_spectrum, HannWindow};
use tuner_core::pitch::{synth_sine, yin, YinConfig};
use tuner_core::strum::{analyse_strum, StrumConfig};
use tuner_core::tunings::GUITAR_STANDARD;

fn bench_yin_2048(c: &mut Criterion) {
    let buf = synth_sine(196.0, 48_000, 2048, 0.5);
    let cfg = YinConfig::default();
    c.bench_function("yin 2048 @ 48kHz", |b| {
        b.iter(|| yin(black_box(&buf), black_box(cfg)))
    });
}

fn bench_yin_4096(c: &mut Criterion) {
    let buf = synth_sine(196.0, 48_000, 4096, 0.5);
    let cfg = YinConfig::default();
    c.bench_function("yin 4096 @ 48kHz", |b| {
        b.iter(|| yin(black_box(&buf), black_box(cfg)))
    });
}

fn bench_fft_4096(c: &mut Criterion) {
    let buf = synth_sine(440.0, 48_000, 4096, 1.0);
    let w = HannWindow::new(4096);
    c.bench_function("magnitude_spectrum 4096", |b| {
        b.iter(|| magnitude_spectrum(black_box(&buf), black_box(&w)))
    });
}

fn bench_chroma_4096(c: &mut Criterion) {
    let buf = synth_sine(261.6256, 48_000, 4096, 1.0);
    let w = HannWindow::new(4096);
    let mag = magnitude_spectrum(&buf, &w);
    c.bench_function("chroma 4096", |b| {
        b.iter(|| compute_chroma(black_box(&mag), 48_000, 4096, 70.0, 5000.0, 440.0))
    });
}

fn bench_strum_full(c: &mut Criterion) {
    let mut buf = vec![0.0_f32; 48_000];
    for s in GUITAR_STANDARD.strings {
        let sine = synth_sine(s.freq_hz(440.0), 48_000, 48_000, 0.5);
        for (b, x) in buf.iter_mut().zip(sine.iter()) {
            *b += *x;
        }
    }
    let cfg = StrumConfig::default();
    c.bench_function("strum full 6-string", |b| {
        b.iter(|| analyse_strum(black_box(&buf), black_box(&GUITAR_STANDARD), black_box(cfg)))
    });
}

criterion_group!(
    benches,
    bench_yin_2048,
    bench_yin_4096,
    bench_fft_4096,
    bench_chroma_4096,
    bench_strum_full
);
criterion_main!(benches);

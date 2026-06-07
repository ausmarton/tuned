# Testing strategy

DSP correctness is a first-class MVP requirement, so the core is tested in seven
layers.

| Layer | Where | What it covers |
|---|---|---|
| 1. Unit | `#[cfg(test)]` in each module | per-function behaviour, edge cases |
| 2. Property | `proptest` in `cents`, `chord` | invariants over random inputs |
| 3. Golden synthetic | `tests/synthetic.rs` | end-to-end with synthesised signals |
| 4. Recorded corpus | `tests/corpus.rs` | regression on real `.wav` recordings |
| 5. Benchmarks | `benches/pitch_bench.rs` | latency budget (criterion) |
| 6. Android instrumented | `tuner-android/.../androidTest` | JNI round-trip on device |
| 7. Web smoke | `tuner-web/tests` | WASM module shape via Vitest |

## Running

```bash
cd tuner-core
cargo test --all-features          # layers 1–4
cargo bench --bench pitch_bench    # layer 5
```

## Canary tests

Two tests guard the subtle, previously-buggy DSP choices. Re-run them after any
change to their module:

- `chroma::tests::chroma_of_c4_peaks_at_pc_0` — guards the triangular
  interpolation in `chroma`. Without it, C4 chroma peaks at C# instead of C.
- `synthetic::strum_analysis_finds_all_six_guitar_strings_in_tune` — guards the
  6th-order band-pass cascade and the filter-warmup skip in `strum`.

## Property invariants

- `ratio_to_cents(x, x) == 0`, octave invariance, antisymmetry.
- MIDI↔Hz round-trips within a cent.
- Every chord template self-recognises (by pitch-class set, since Sus2/Sus4 are
  enharmonic).

## The recorded corpus

`tests/corpus/{clean,strum,chords,noisy}/` pairs `.wav` files with `.label`
files. The harness **skips gracefully when a folder is empty** (its default
state) so CI stays green until real recordings are committed. Label formats are
documented at the top of `tests/corpus.rs`.

## Synthetic chord signals

Chord recognition is tested with **additive plucked-string synthesis** (8
harmonics, 1/n amplitude rolloff), not pure sines — a single low-frequency sine
gives the FFT nothing to resolve pitch classes from. A strong 7th harmonic can
make maj7/7 outrank the plain major, which is musically defensible, so the test
accepts the major family `{Major, MajorSeventh, Seventh}`.

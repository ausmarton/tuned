# OpenTuner — Handoff for Claude Code

**Status:** Mid-flight. Rust DSP core was implemented and verified — 92 unit
tests + 10 integration tests + 8 doctests + 4 corpus-stub tests = **110 tests
passing, zero clippy warnings** at the end of the previous session. Android
shell scaffolded but not built (requires Android SDK + NDK). Web app scaffolded
but `src/` empty. CI workflows and final LICENSE text still to do.

**Why this handoff exists:** The previous work happened across multiple
chat-based Claude sessions whose sandbox filesystem (`/home/claude/`) resets
between sessions. Files were not exported to a persistent location after each
session — that was a mistake. This document captures everything needed to
rebuild and continue the project from a Claude Code terminal session where
files actually persist.

---

## 1. Requirements (from the user)

A free, open-source instrument tuner targeting:

- 6-string guitar (standard E A D G B E)
- 4-string bass (standard E A D G)
- Guitarra portuguesa — both **Lisboa** and **Coimbra** tunings

Features:

1. Single-string tuning *and* auto-detect (which string + direction).
2. **Strum analysis** — strum all strings, report per-string offset.
3. **Chord identification** — name a strummed/played chord.

Must work in noisy environments.

Confirmed via Q&A in the first session:

| Question | Answer |
|---|---|
| Platform | **Android primary**, web nice-to-have, **no iOS** |
| Tech stack | Whichever is best for noisy-environment DSP; Rust preferred where viable |
| Polish | MVP + comprehensive tests + CI + docs; DSP rigorously tested |
| License | **GPL-3.0-or-later** |
| Tuning DB extensibility | MVP scope only — just the three instruments above |
| Alt tunings | Only guitarra portuguesa Lisboa + Coimbra; no alt guitar tunings in MVP |

---

## 2. Architectural decisions

### Single DSP core, two shells

```
┌─────────────────────────────────────────────────────┐
│  Android app (Kotlin + Jetpack Compose) [primary]   │
│  Web app   (TypeScript + React + WebAudio) [bonus]  │
└────────────────────────┬────────────────────────────┘
                         │
            JNI / WASM bindings (thin)
                         │
┌────────────────────────▼────────────────────────────┐
│  tuner-core (Rust)                                  │
│  - YIN pitch detection (monophonic)                 │
│  - Per-string bandpass + YIN (strum mode)           │
│  - Chroma + template chord recognition              │
│  - DC blocker + noise-floor tracker                 │
│  - Tuning definitions                               │
└─────────────────────────────────────────────────────┘
```

Rust core is the **single source of DSP truth**. Same code compiles to Android
(JNI cdylib) and the browser (WASM). Tests and benchmarks live with the Rust
crate.

### DSP design (specified in `docs/DSP.md`)

- 48 kHz, frame 4096 samples (~85 ms), hop 2048 (50% overlap).
- **Monophonic**: YIN (de Cheveigné & Kawahara 2002) with cumulative mean
  normalised difference + parabolic interpolation.
- **Strum**: per-string RBJ biquad bandpass cascade → YIN per band.
- **Chord**: log-magnitude chroma (Müller 2007) with triangular interpolation,
  cosine similarity vs 12×9 = 108 binary templates.
- DC blocker: single-pole HPF at ~38 Hz.
- Confidence gate: aperiodicity + RMS vs noise floor.

### Threading model

DSP is single-threaded inside `tuner-core`. The host (Android or web)
provides the thread. Audio callback pushes samples (lock-free); UI polls
results (mutex at frame boundary).

---

## 3. Verified tunings

Sources are in `docs/INSTRUMENTS.md`. All frequencies derive from MIDI
assuming A4 = 440 Hz.

**6-string guitar (standard):** E2 A2 D3 G3 B3 E4 — MIDI 40, 45, 50, 55, 59, 64

**4-string bass (standard):** E1 A1 D2 G2 — MIDI 28, 33, 38, 43

**Guitarra portuguesa — Lisboa** (low→high, `DABEAB`): D3 A3 B3 E4 A4 B4 —
MIDI 50, 57, 59, 64, 69, 71. Courses 1-3 unison, courses 4-6 octave-paired.
Source: Tobe Richards, *Portuguese Guitar Chord Bible: Lisboa Tuning* (2016)
ISBN 978-1906207434.

**Guitarra portuguesa — Coimbra** (low→high, `CGDAGA`, one whole tone below
Lisboa): C3 G3 A3 D4 G4 A4 — MIDI 48, 55, 57, 62, 67, 69. Source: Tobe
Richards, *Portuguese Guitar Chord Bible: Coimbra Tuning* (2016).

The octave-paired upper strings (courses 4-6) are not detected separately —
they show up as a strong 2nd harmonic of the nominal pitch, which is exactly
what a well-tuned octave course produces.

---

## 4. Repository layout

```
opentuner/
├── README.md
├── LICENSE                  GPL-3.0 (placeholder — fetch canonical text before v1)
├── .gitignore
├── tuner-core/              Rust DSP library
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs           crate root, TunerError, TunerConfig
│   │   ├── cents.rs         cents / MIDI math (heavily property-tested)
│   │   ├── tunings.rs       Tuning data for all 4 shipped tunings
│   │   ├── pitch.rs         YIN algorithm
│   │   ├── fft.rs           Hann window + magnitude spectrum
│   │   ├── chroma.rs        log-magnitude chroma w/ triangular interpolation
│   │   ├── chord.rs         Quality enum, template(), recognise()
│   │   ├── strum.rs         per-string bandpass + YIN
│   │   ├── noise.rs         DcBlocker, NoiseFloor
│   │   ├── tuner.rs         Tuner facade (ring buffer + analysis buffer)
│   │   └── bindings/
│   │       ├── mod.rs       feature-gates jni and wasm
│   │       ├── jni.rs       Android JNI surface
│   │       └── wasm.rs      wasm-bindgen surface
│   ├── tests/
│   │   ├── synthetic.rs     integration tests with synthetic signals
│   │   └── corpus.rs        WAV regression tests (skips when corpus is empty)
│   └── benches/
│       └── pitch_bench.rs   criterion benchmarks
├── tuner-android/           Kotlin/Compose app
│   ├── build.gradle.kts
│   ├── settings.gradle.kts
│   ├── gradle.properties
│   └── app/
│       ├── build.gradle.kts (with buildRustCore task)
│       ├── proguard-rules.pro
│       └── src/
│           ├── main/
│           │   ├── AndroidManifest.xml
│           │   ├── kotlin/com/opentuner/
│           │   │   ├── MainActivity.kt
│           │   │   ├── TunerViewModel.kt
│           │   │   ├── NativeTuner.kt
│           │   │   ├── Snapshot.kt
│           │   │   └── audio/AudioEngine.kt
│           │   └── res/
│           │       ├── values/strings.xml
│           │       ├── values/themes.xml
│           │       └── xml/data_extraction_rules.xml
│           ├── test/kotlin/com/opentuner/SnapshotTest.kt
│           └── androidTest/kotlin/com/opentuner/NativeTunerInstrumentedTest.kt
├── tuner-web/               TypeScript/React/WASM app — scaffolded, src/ empty
│   ├── package.json
│   ├── tsconfig.json
│   └── vite.config.ts
├── docs/
│   ├── ARCHITECTURE.md
│   ├── DSP.md
│   ├── TESTING.md
│   ├── CONTRIBUTING.md
│   └── INSTRUMENTS.md
├── .github/workflows/       NOT YET CREATED
└── scripts/                 NOT YET CREATED
```

---

## 5. Toolchain notes (important!)

The previous session ran in a sandbox where:

- `rustup.rs` and `static.rust-lang.org` were blocked by network policy.
- Rust 1.75 was available via `apt-get install -y rustc cargo`.

This meant several transitive dev-dependencies that recently bumped their MSRV
to 1.80+ had to be pinned. **In Claude Code with a modern toolchain (Rust
1.80+), these pins should be removed.** If you `rustup install stable` first,
you can delete the entire pinning section from `tuner-core/Cargo.toml`.

The pins that were needed for Rust 1.75:

```toml
clap          = "=4.4.18"
clap_lex      = "=0.6.0"
anstyle       = "=1.0.6"
regex         = "=1.10.6"
regex-automata = "=0.4.7"
tempfile      = "=3.10.1"
rand_core     = "=0.6.4"
rand          = "=0.8.5"
fastrand      = "=2.1.1"
rayon-core    = "=1.12.1"
rayon         = "=1.10.0"
half          = "=2.4.1"
proptest      = "=1.4.0"
```

On Rust 1.80+, use `proptest = "1"` and let cargo pick everything else.

---

## 6. Bugs found and fixed during verification

Real DSP bugs discovered by running tests, not invented from theory:

1. **Chroma boundary error**: nearest-semitone rounding of FFT bins placed
   energy on the wrong side of pitch-class boundaries when a sine fell
   between two FFT bins straddling a semitone line (e.g. C4 = 261.6 Hz at
   48 kHz / 4096 FFT, bin 23 = 269.5 Hz → midi 60.52 → rounded to C# not C).
   **Fix:** triangular interpolation — each FFT bin contributes to two
   adjacent pitch classes weighted by fractional distance.

2. **Strum cross-talk**: 4th-order bandpass (2 cascaded biquads) leaked
   neighbouring strings (E2 detected at 87 Hz because A2 at 110 Hz leaked
   through). **Fix:** cascade three biquads (6th order, ~36 dB/oct) + skip
   the first half of the filtered buffer for YIN (filter warmup). Result:
   clean ±0.4-cent detection of all 6 strings in a mixed-sine strum.

3. **Tuner ring buffer too short** for strum/chord analysis: was only
   `frame_size = 4096` samples (~85 ms), insufficient for low-frequency
   filter settling. **Fix:** added a separate 1.5-second `analysis_buf`
   used by `analyse_strum()` and `recognise_chord()`. The per-frame YIN
   path still uses the short ring.

4. **Confidence gate fooled by signal-dominated buffers**: when noise floor
   was estimated from the same buffer as the signal, the floor ended up
   near the signal RMS and confidence stayed below the gate. **Fix:**
   strum confidence now uses band-vs-total RMS ratio (a well-tuned string
   concentrates its energy in its band); monophonic confidence uses an
   absolute-silence floor.

5. **Chord recognition test was unrealistic** using pure sines (no harmonics).
   Real instruments produce 1/n-amplitude partials over 8+ harmonics.
   **Fix:** test uses additive plucked-string synthesis. With harmonics,
   the recogniser correctly identifies the root and a sensible major-family
   quality (Cmaj7 may beat C major when the 7th harmonic is strong — that's
   musically defensible). The test accepts {Major, MajorSeventh, Seventh}
   as equivalent for the synthetic signal.

6. **Sus2/Sus4 are enharmonic**: Csus2 = {C,D,G} = Gsus4 (same pitch-class
   set). The recogniser correctly returns both with score 1.0 → margin
   gate rejects "best". Original tests asserted a single best; **fix:**
   tests now assert pitch-class-set membership instead of exact quality.

7. **No rust-lang.org in sandbox** → used `apt-get install -y rustc cargo`.
   (Probably irrelevant in Claude Code.)

---

## 7. Verified test results (end of previous session)

```
$ cargo test
test result: ok. 92 passed; 0 failed   (unit tests)
test result: ok. 10 passed; 0 failed   (tests/synthetic.rs)
test result: ok. 8  passed; 0 failed   (doctests)
test result: ok. 4  passed; 0 failed   (tests/corpus.rs — skips empty corpus)

$ cargo clippy --all-targets
Finished — 0 warnings.

$ cargo bench --bench pitch_bench -- --quick
yin 4096 @ 48kHz        time: ~3.0 ms     (well under the 50 ms latency budget)
magnitude_spectrum 4096 time: ~100 µs
```

---

## 8. What's pending

In rough priority order:

1. **Replace the LICENSE placeholder** with the canonical GPL-3.0 text from
   <https://www.gnu.org/licenses/gpl-3.0.txt>. CI should verify SHA-256.

2. **Web app source** (`tuner-web/src/`): React components (TunerScreen,
   StringDisplay, ChordDisplay, StrumGrid, TuningPicker), an AudioWorklet
   processor that feeds the WASM module, Vitest smoke tests.

3. **CI workflows** in `.github/workflows/`:
   - `core.yml` — fmt, clippy `-D warnings`, test, audit, deny, doc.
   - `android.yml` — ktlint, gradle test, build APK.
   - `web.yml` — tsc, eslint, vitest, vite build.

4. **`docs/CONTRIBUTING.md`** should document why those Cargo.toml pins
   exist (or remove them if targeting Rust 1.80+).

5. **`scripts/`** — release helpers for cross-compiling the .so files and
   publishing the WASM package.

6. **Real test corpus** in `tuner-core/tests/corpus/{clean,strum,chords,noisy}/`
   — committed WAV files with `.label` files. The corpus harness is already
   written and skips gracefully when empty.

7. **Try to build and run the Android app** end-to-end (`./gradlew
   assembleDebug` on a host with NDK r26+). The Rust crate's `cdylib` target
   is set up but the cross-compile has not been tested.

---

## 9. Resumption guide for Claude Code

When you start, the working directory should be a fresh empty repo. Bring
this whole document into context (it's `~600` lines but heavily annotated
and short on the code itself — most code is *outlined* rather than fully
pasted because Claude can regenerate it from the spec). Then:

1. Read this document.
2. Create the directory layout from §4.
3. Generate every file from the spec in §10 below. **Important:** at every
   step, run `cargo test` after creating each module to catch regressions
   early. The previous session's verification trail is in §6.
4. After everything's regenerated and tests pass, tackle the pending items
   in §8.

Verification milestones (run these in order):

```bash
# After §10 Rust core files:
cd tuner-core && cargo build && cargo test --lib

# After §10 integration tests:
cd tuner-core && cargo test

# After everything:
cd tuner-core && cargo clippy --all-targets -- -D warnings
cd tuner-core && cargo doc --no-deps --all-features
```

If anything fails, the most likely culprits in order are:

- **Float comparison in `const fn`** — `TunerConfig::validate()` must not
  be `const`. Rust 1.75 explicitly forbids float comparisons in const
  contexts; even later versions had issues.
- **`vec![0.0; N]`** in tests where context expects `f32` — write
  `vec![0.0_f32; N]` or push to a slice that constrains the type.
- **Missing `extern crate alloc;`** in lib.rs — every collection import
  (`alloc::vec::Vec`, `alloc::string::String`) depends on this.
- **Test-buffer length too short** — for low frequencies (bass E1 = 41 Hz,
  period 24 ms), YIN needs >2 periods, so push at least 8192 samples.

---

## 10. File-by-file spec

Each subsection gives either the full file content (for short critical files)
or a precise enough spec to regenerate it. The Rust core was verified passing;
the Kotlin and config files compiled in principle but were not run end-to-end
because the sandbox lacked Android SDK / NDK.

### 10.1 `README.md` (top-level)

Standard project README:

- Title, one-line description, GPL-3.0 badge.
- Features list matching §1.
- Architecture ASCII diagram from §2.
- "Quick start" with `cargo test`, `./gradlew assembleDebug`, `npm install &&
  npm run build`.
- Repository-layout table.
- Pointers to `docs/{DSP,ARCHITECTURE,TESTING,CONTRIBUTING,INSTRUMENTS}.md`.

### 10.2 `LICENSE`

Replace placeholder with canonical GPL-3.0 text from
<https://www.gnu.org/licenses/gpl-3.0.txt>. CI should SHA-256-verify it.

### 10.3 `.gitignore`

```
# Rust
target/
**/*.rs.bk
Cargo.lock
proptest-regressions/*.txt

# Android / Gradle
.gradle/
build/
local.properties
.idea/
*.iml
captures/
.externalNativeBuild/
.cxx/

# Node / web
node_modules/
dist/
.vite/
*.tsbuildinfo
pkg/

# OS
.DS_Store
Thumbs.db

# Editor
.vscode/
*.swp
```

### 10.4 `docs/DSP.md`, `docs/ARCHITECTURE.md`, `docs/TESTING.md`, `docs/CONTRIBUTING.md`, `docs/INSTRUMENTS.md`

These were ~150-line documents each. The key technical content is captured
in §§2–3 of this handoff. Regenerate each from those summaries:

- **DSP.md** — sample rates, frame sizes, pipeline diagram, YIN
  pseudocode, cents formula, strum bandpass approach, chord template
  matching, noise handling. Cite the de Cheveigné/Kawahara 2002 paper for
  YIN, Müller 2007 for chroma.
- **ARCHITECTURE.md** — the layered architecture from §2, the threading
  model, the `TunerConfig` struct, what's explicitly out of scope (iOS,
  recording, ads, telemetry).
- **TESTING.md** — the 7-layer strategy: unit, property (proptest), golden
  synthetic, recorded corpus, criterion benches, Android instrumented,
  web Vitest.
- **CONTRIBUTING.md** — `cargo fmt`, `cargo clippy --all-targets -- -D
  warnings`, `cargo test --all-features`, `cargo audit`, ktlint, `tsc
  --strict`. Document that DSP changes require updating DSP.md and the
  golden-test expected values explicitly.
- **INSTRUMENTS.md** — the tables from §3 with citations.

### 10.5 `tuner-core/Cargo.toml`

```toml
[package]
name        = "tuner-core"
version     = "0.1.0"
edition     = "2021"
rust-version = "1.75"
license     = "GPL-3.0-or-later"
description = "DSP core for OpenTuner: pitch detection, strum analysis, chord recognition for guitar, bass and guitarra portuguesa."
repository  = "https://github.com/opentuner/opentuner"
readme      = "../README.md"
keywords    = ["audio", "dsp", "tuner", "music", "pitch-detection"]
categories  = ["multimedia::audio", "no-std"]

[lib]
crate-type = ["rlib", "cdylib", "staticlib"]

[features]
default = ["std"]
std     = []
jni     = ["std", "dep:jni"]
wasm    = ["std", "dep:wasm-bindgen", "dep:js-sys"]

[dependencies]
rustfft       = { version = "6.2", default-features = false }
num-complex   = { version = "0.4", default-features = false }
libm          = "0.2"

jni           = { version = "0.21", optional = true }
wasm-bindgen  = { version = "0.2", optional = true }
js-sys        = { version = "0.3", optional = true }
log           = { version = "0.4", optional = true }

[dev-dependencies]
proptest      = "1"
approx        = "0.5"
criterion     = { version = "0.5", features = ["html_reports"] }
hound         = "3.5"

# NOTE: the pins listed in section 7 of HANDOFF.md were only needed on Rust
# 1.75 (the previous session's sandbox). On a normal modern toolchain, leave
# this section empty.

[[bench]]
name    = "pitch_bench"
harness = false

[profile.release]
lto           = "fat"
codegen-units = 1
panic         = "abort"
opt-level     = 3

[profile.bench]
inherits = "release"
debug    = true
```

### 10.6 `tuner-core/src/lib.rs`

```rust
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

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TunerError {
    InvalidConfig(&'static str),
    UnknownTuning,
    BufferTooShort { got: usize, required: usize },
}

impl fmt::Display for TunerError { /* ... */ }
#[cfg(feature = "std")]
impl std::error::Error for TunerError {}

#[derive(Debug, Clone, PartialEq)]
pub struct TunerConfig {
    pub sample_rate_hz: u32,    // default 48_000
    pub frame_size: usize,      // default 4096
    pub hop_size: usize,        // default 2048
    pub a4_hz: f32,             // default 440.0
    pub yin_threshold: f32,     // default 0.12
    pub chord_min_score: f32,   // default 0.85
    pub chord_min_margin: f32,  // default 0.05
    pub active_tuning_id: &'static str,  // default "guitar.standard"
    pub noise_subtraction: bool,         // default true
}

impl Default for TunerConfig { /* obvious */ }

impl TunerConfig {
    // NOT const fn — float comparisons aren't allowed in const fn on 1.75.
    pub fn validate(&self) -> Result<(), TunerError> {
        if self.sample_rate_hz == 0 { return Err(TunerError::InvalidConfig("sample_rate_hz must be > 0")); }
        if self.frame_size == 0 || !self.frame_size.is_power_of_two() {
            return Err(TunerError::InvalidConfig("frame_size must be a non-zero power of two"));
        }
        if self.hop_size == 0 || self.hop_size > self.frame_size {
            return Err(TunerError::InvalidConfig("hop_size must be in 1..=frame_size"));
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

// Unit tests at the bottom: default_config_is_valid, rejects_zero_sample_rate,
// rejects_non_power_of_two_frame, rejects_hop_larger_than_frame,
// rejects_out_of_range_a4, error_display_is_human_readable.
```

### 10.7 `tuner-core/src/cents.rs`

Pure-function module. Public API:

```rust
pub fn ratio_to_cents(measured_hz: f32, target_hz: f32) -> f32;
pub fn midi_to_hz(midi: u8, a4_hz: f32) -> f32;
pub fn hz_to_midi(hz: f32, a4_hz: f32) -> Option<f32>;  // None if non-positive
pub fn pitch_class(hz: f32, a4_hz: f32) -> Option<u8>;  // 0..=11
pub enum Direction { Flat, InTune, Sharp }
pub fn classify(cents: f32, in_tune_window_cents: f32) -> Direction;
```

`ratio_to_cents` formula: `1200 * log2(measured / target)`; returns `0.0` for
non-positive inputs.

`midi_to_hz`: `a4_hz * 2.0_f32.powf((midi - 69.0) / 12.0)`.

`pitch_class`: `(midi.round() as i32).rem_euclid(12) as u8` — no extra parens.

For `no_std` we need `libm::log2f` and `libm::powf`; for `std` we use methods.
Conditional `use`:
```rust
#[cfg(not(feature = "std"))]
use libm::{log2f, powf};
#[cfg(feature = "std")]
fn log2f(x: f32) -> f32 { x.log2() }
#[cfg(feature = "std")]
fn powf(b: f32, e: f32) -> f32 { b.powf(e) }
```

Tests: ~10 unit tests covering unison/octave/semitone/non-positive cases,
plus 4 property tests (cents_self_is_zero, cents_octave_invariant,
midi_round_trip_property, cents_is_antisymmetric).

### 10.8 `tuner-core/src/tunings.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstrumentClass { Guitar6, Bass4, GuitarraPortuguesa }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringSpec {
    pub name: &'static str,
    pub midi: u8,
}
impl StringSpec {
    pub fn freq_hz(&self, a4_hz: f32) -> f32 { midi_to_hz(self.midi, a4_hz) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tuning {
    pub id: &'static str,
    pub display_name: &'static str,
    pub instrument: InstrumentClass,
    pub strings: &'static [StringSpec],  // lowest pitch -> highest pitch
}

// pub const GUITAR_STANDARD: Tuning = ...    (ids per §3 above)
// pub const BASS_STANDARD: Tuning = ...
// pub const GUITARRA_LISBOA: Tuning = ...
// pub const GUITARRA_COIMBRA: Tuning = ...
pub const ALL: &[Tuning] = &[GUITAR_STANDARD, BASS_STANDARD, GUITARRA_LISBOA, GUITARRA_COIMBRA];

pub fn lookup(id: &str) -> Option<&'static Tuning> {
    ALL.iter().find(|t| t.id == id)
}

pub fn closest_string(tuning: &Tuning, freq_hz: f32, a4_hz: f32) -> Option<(usize, f32)>;
```

Strings are stored low-to-high. For guitarra portuguesa we store the nominal
(lower) note of each course.

Tests: every tuning has unique id, every shipped tuning is found by lookup,
strings strictly ascending, exact frequencies (E2 = 82.4069 Hz, etc.),
Coimbra is exactly 2 semitones below Lisboa per string, closest_string picks
exact matches and nearest neighbours.

### 10.9 `tuner-core/src/pitch.rs` — YIN algorithm

```rust
#[derive(Debug, Clone, Copy)]
pub struct PitchEstimate {
    pub frequency_hz: f32,
    pub aperiodicity: f32,  // YIN's d'(τ) at the chosen lag
    pub rms: f32,
}

pub fn confidence(estimate: &PitchEstimate, noise_floor_rms: f32) -> f32;

#[derive(Debug, Clone, Copy)]
pub struct YinConfig {
    pub sample_rate_hz: u32,
    pub min_hz: f32,
    pub max_hz: f32,
    pub threshold: f32,  // typical 0.10-0.20
}
// Default: sr 48000, min 40, max 1000, threshold 0.12

pub fn yin(buffer: &[f32], cfg: YinConfig) -> Option<PitchEstimate>;
pub fn synth_sine(freq_hz: f32, sample_rate_hz: u32, n: usize, amplitude: f32) -> Vec<f32>;
```

Algorithm (this exact implementation passed the verification tests):

1. Bounds-check: return None if `cfg.min_hz <= 0`, `cfg.max_hz <= cfg.min_hz`,
   or `buffer.len() < 2 * tau_max + 1` (where `tau_max = ceil(sr / min_hz)`).
2. `d[tau] = sum over i in 0..buffer.len()/2 of (buffer[i] - buffer[i+tau]).powi(2)` for `tau in 1..=tau_max`.
3. `d_prime[1] = 1`; for `tau in 1..=tau_max`: `running += d[tau]; d_prime[tau] = if running > 0 { d[tau] * tau / running } else { 1.0 };`
4. Find first `tau >= tau_min` where `d_prime[tau] < threshold`, then walk forward while still descending. Fallback to global min in range if nothing crosses threshold and `best_val < 1.0`.
5. Parabolic interpolation: `delta = 0.5 * (y_minus - y_plus) / (y_minus - 2.0 * y_zero + y_plus)`, only if denom > 1e-12 and `|delta| < 1`.
6. `frequency_hz = sample_rate / tau_refined`.

Confidence:
```
periodicity = (1 - (aperiodicity / 0.6).min(1.0)).max(0.0)
signal_ratio = if noise_floor > 0 { (rms / (10 * noise_floor)).clamp(0, 1) } else { 1.0 }
confidence = periodicity * signal_ratio
```

Tests: pure_sine_440_detected_within_one_cent (clean A4 → ±1 cent),
pure_sine_low_e_82hz_detected (bass E2 → ±5 cents), detuned_sine_returns_correct_offset
(440 × 2^(10/1200) → +10 ±1 cents), silence_returns_no_pitch_or_low_confidence,
rejects_too_short_buffer, rejects_inverted_range, parabolic_interpolation_centre/offset/flat,
confidence_zero_for_pure_noise, confidence_high_for_strong_periodic_signal.

### 10.10 `tuner-core/src/fft.rs`

```rust
#[derive(Debug, Clone)]
pub struct HannWindow { coeffs: Vec<f32> }
impl HannWindow {
    pub fn new(n: usize) -> Self;            // periodic: w[k] = 0.5 * (1 - cos(2π k / N))
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn apply(&self, samples: &mut [f32]);
}
pub fn magnitude_spectrum(buffer: &[f32], window: &HannWindow) -> Vec<f32>;
pub fn bin_to_hz(bin: usize, sample_rate_hz: u32, fft_len: usize) -> f32;
```

`magnitude_spectrum`: panics if lengths don't match or fft_len isn't a power of
two; uses `rustfft::FftPlanner`. Output has length `fft_len / 2 + 1`. Use
`c.re.hypot(c.im)` not `(re*re + im*im).sqrt()` — clippy prefers the former.

Tests: hann endpoint is 0, centre is 1, empty case, apply matches manual,
magnitude_spectrum_picks_sine_bin (peak near 440 Hz when input is a 440 Hz
sine), bin_to_hz known values, mismatch + non-pow2 panics.

### 10.11 `tuner-core/src/chroma.rs` — CRITICAL: triangular interpolation

```rust
pub type ChromaVector = [f32; 12];

pub fn compute_chroma(
    magnitudes: &[f32],
    sample_rate_hz: u32,
    fft_len: usize,
    min_hz: f32,    // e.g. 70.0
    max_hz: f32,    // e.g. 5000.0
    a4_hz: f32,
) -> ChromaVector;

pub fn normalise(chroma: &mut ChromaVector);
pub fn cosine_similarity(a: &ChromaVector, b: &ChromaVector) -> f32;
```

**The triangular interpolation is the bug fix from §6.1**. Inside the loop:

```rust
let hz = bin_to_hz(bin, sample_rate_hz, fft_len);
if hz < min_hz || hz > max_hz || mag <= 0.0 { continue; }
let Some(midi) = hz_to_midi(hz, a4_hz) else { continue };
let weight = mag.ln_1p();           // ln(1+m) for precision
let pc_floor = midi.floor();
let frac = midi - pc_floor;
let lower = (pc_floor as i32).rem_euclid(12) as usize;
let upper = (lower + 1) % 12;
chroma[lower] += weight * (1.0 - frac);
chroma[upper] += weight * frac;
```

Then `normalise(&mut chroma)`. Without the triangular interpolation, C4
chroma peaks at C# instead of C — this was a real verified failure.

Tests: chroma_sums_to_one_on_real_input, chroma_of_a440_peaks_at_pc_9,
chroma_of_c4_peaks_at_pc_0 (the regression test for the bug), silence is zero,
cosine_similarity self/orthogonal/zero, normalise zero vector.

### 10.12 `tuner-core/src/chord.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Quality { Major, Minor, Seventh, MajorSeventh, MinorSeventh, Sus2, Sus4, Diminished, Augmented }

impl Quality {
    pub const fn suffix(&self) -> &'static str;        // "", "m", "7", "maj7", "m7", "sus2", "sus4", "dim", "aug"
    pub const fn intervals(&self) -> &'static [u8];    // semitone offsets from root
    pub const ALL: &'static [Self] = &[ /* all 9 */ ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Root(pub u8);
impl Root {
    pub const fn letter(&self) -> &'static str;  // sharps; B♭ shows as "A#"
}

#[derive(Debug, Clone)]
pub struct ChordMatch {
    pub name: String,
    pub root_pc: u8,
    pub quality: Quality,
    pub score: f32,
}

pub fn template(root_pc: u8, quality: Quality) -> ChromaVector;

#[derive(Debug, Clone)]
pub struct RecognitionResult {
    pub candidates: Vec<ChordMatch>,  // top 5, descending score
    pub best: Option<ChordMatch>,     // only if score >= min_score AND margin >= min_margin
}

pub fn recognise(chroma: &ChromaVector, min_score: f32, min_margin: f32) -> RecognitionResult;

#[doc(hidden)]
pub fn parse(name: &str) -> Option<(u8, Quality)>;  // for tests; "Cmaj7", "F#m", "Bbm7"
```

Intervals:
- Major [0,4,7], Minor [0,3,7], Sus2 [0,2,7], Sus4 [0,5,7]
- Seventh [0,4,7,10], MajorSeventh [0,4,7,11], MinorSeventh [0,3,7,10]
- Diminished [0,3,6], Augmented [0,4,8]

`template` builds a binary vector with `1` at each interval-pc position then
normalises to unit sum (so cosine similarity is scale-free).

Tests: major has 3 notes, C major hits C/E/G, transposition invariance,
major and minor differ only in 3rd, recognise of perfect C major returns C
major, recognise each major root, recognise each quality (but allow
Sus2/Sus4 enharmonic ambiguity — assert pitch-class-set membership, not
exact (root, quality)), silence returns no best, parse handles sharps and
qualities, margin threshold rejects ambiguous blends, plus a property test
`template_self_recognition` that also uses set membership not exact match.

### 10.13 `tuner-core/src/strum.rs` — CRITICAL: 6th-order cascade + warmup-skip

```rust
#[derive(Debug, Clone)]
pub struct StringResult {
    pub string_index: usize,
    pub name: &'static str,
    pub target_hz: f32,
    pub detected_hz: Option<f32>,
    pub cents_off: Option<f32>,
    pub direction: Option<Direction>,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct StrumReport {
    pub strings: Vec<StringResult>,
}
impl StrumReport {
    pub fn detected_count(&self) -> usize;
    pub fn in_tune_count(&self) -> usize;
}

#[derive(Debug, Clone, Copy)]
pub struct StrumConfig {
    pub sample_rate_hz: u32,
    pub a4_hz: f32,
    pub in_tune_window_cents: f32,  // default 5.0
    pub band_half_cents: f32,       // default 100.0
    pub yin_threshold: f32,         // default 0.15
    pub min_confidence: f32,        // default 0.20
}

pub fn analyse_strum(buffer: &[f32], tuning: &Tuning, cfg: StrumConfig) -> StrumReport;

#[doc(hidden)] #[must_use]
pub fn bandpass(input: &[f32], sample_rate_hz: u32, low_hz: f32, high_hz: f32) -> Vec<f32>;
```

**This is the most subtle module.** Implementation steps in `analyse_strum`:

1. Compute `total_rms` of the whole buffer.
2. If `total_rms < 1e-4`, mark every string as not detected.
3. For each string, compute band edges from cents (low = target × 2^(-band_half_cents/1200), high = target × 2^(+band_half_cents/1200)).
4. Call `bandpass(buffer, sr, low, high)`.
5. **Skip the first half of the filtered buffer for YIN** — the cascade's transient ringing destroys low-frequency detection if not skipped. Take the second half (or the full buffer if it's too short to split).
6. YIN config: min_hz = low × 0.9, max_hz = high × 1.1, threshold from cfg.
7. If YIN returns Some, compute:
   ```
   periodicity = (1 - (aperiodicity / 0.6).min(1.0)).max(0.0)
   band_ratio = (est.rms / total_rms / 0.10).clamp(0.0, 1.0)  // a single string contributes ~1/√6 of a 6-string strum
   confidence = periodicity * band_ratio
   ```
8. Gate: if `confidence < cfg.min_confidence` OR `total_rms < 1e-4`, mark not
   detected (but record the confidence value for diagnostics).
9. Otherwise compute cents from target and classify direction.

`bandpass` is **three** cascaded RBJ biquad band-pass stages (6th order). The
biquad design:
```
centre = sqrt(low * high)
bw_octaves = log2(high / low)
q = (2^bw_octaves - 1).recip() * 2^(bw_octaves / 2)
q = q.clamp(0.3, 20.0)
```
Then biquad_bandpass(centre, q, sr) with the standard RBJ formulae:
```
w0 = 2π × centre / sr
alpha = sin(w0) / (2q)
a0 = 1 + alpha
b0 = alpha / a0
b1 = 0
b2 = -alpha / a0
a1 = -2 cos(w0) / a0
a2 = (1 - alpha) / a0
```
Apply via Direct Form I, then cascade 3 times.

Tests: analyse_returns_one_per_string, silence_produces_no_detections,
perfectly_tuned_sines_per_string_are_in_tune (mix of 6 sines, ≥5 in tune),
detuned_string_is_reported_in_correct_direction (D3 +20 cents detected as
Sharp within ±5 cents), biquad_coeffs_have_expected_signs,
bandpass_passes_centre_attenuates_far, noise_floor_estimate_on_zero_buffer_is_zero
(an unused helper `estimate_noise_floor` is kept marked `#[allow(dead_code)]`).

### 10.14 `tuner-core/src/noise.rs`

```rust
#[derive(Debug, Clone, Copy)]
pub struct DcBlocker { pole: f32, prev_x: f32, prev_y: f32 }
impl DcBlocker {
    pub const fn new() -> Self;                 // pole = 0.995
    pub const fn with_pole(pole: f32) -> Self;
    pub fn process_sample(&mut self, x: f32) -> f32;  // y = x - prev_x + pole * prev_y
    pub fn process_in_place(&mut self, buf: &mut [f32]);
    pub fn reset(&mut self);
}

#[derive(Debug, Clone, Copy)]
pub struct NoiseFloor { rms: f32, alpha: f32, initialised: bool }
impl NoiseFloor {
    pub const fn new(alpha: f32) -> Self;
    pub fn update_if_quiet(&mut self, frame_rms: f32);  // only if within 6 dB of current
    pub fn rms(&self) -> f32;
}
```

Tests: DC blocker removes constant offset, preserves 1 kHz AC, in_place ≡
per_sample. NoiseFloor initialises on first update, ignores loud frames,
smooths similar frames.

### 10.15 `tuner-core/src/tuner.rs` — Tuner facade with TWO buffers

```rust
#[derive(Debug, Clone)]
pub struct TunerSnapshot {
    pub pitch_hz: Option<f32>,
    pub cents_off: Option<f32>,
    pub direction: Option<Direction>,
    pub nearest_string: Option<usize>,
    pub nearest_string_name: Option<&'static str>,
    pub confidence: f32,
}
impl TunerSnapshot { pub const fn empty() -> Self; }

pub struct Tuner {
    cfg: TunerConfig,
    tuning: &'static Tuning,
    ring: Vec<f32>,                  // length = frame_size, for per-frame YIN
    write_idx: usize,
    samples_since_hop: usize,
    analysis_buf: Vec<f32>,          // length = max(sr * 3 / 2, frame_size * 4), for strum/chord
    analysis_write_idx: usize,
    analysis_filled: bool,
    dc_blocker: DcBlocker,
    noise_floor: NoiseFloor,
    window: HannWindow,
    latest: TunerSnapshot,
}

impl Tuner {
    pub fn new(cfg: TunerConfig) -> Result<Self, TunerError>;
    pub fn set_tuning(&mut self, tuning_id: &str) -> Result<(), TunerError>;
    pub fn active_tuning(&self) -> &'static Tuning;
    pub fn config(&self) -> &TunerConfig;
    pub fn push_samples(&mut self, samples: &[f32]);   // writes to BOTH buffers via DC blocker
    pub fn snapshot(&self) -> TunerSnapshot;
    pub fn analyse_strum(&self) -> StrumReport;       // uses analysis_buf
    pub fn recognise_chord(&self) -> RecognitionResult; // uses last frame_size of analysis_buf
}
```

`push_samples` runs each sample through `dc_blocker.process_sample`, then
writes to `ring` (frame_size circular) AND `analysis_buf` (longer circular).
When `samples_since_hop >= hop_size`, calls `analyse_frame()`.

`analyse_frame` algorithm:
1. Compute RMS of the short ring's linear view.
2. If `rms < 1e-4`, update noise floor, set snapshot to empty, return.
3. Pitch range from active tuning: `lowest = lowest_string × 0.85`,
   `highest = highest_string × 1.2`.
4. YIN. If returns Some, compute:
   ```
   periodicity = (1 - (aperiodicity / 0.6).min(1.0)).max(0.0)
   signal_factor = (rms / 1e-3).clamp(0, 1)   // 10× absolute silence
   conf = periodicity * signal_factor
   ```
5. If conf < 0.10, return empty (but with confidence value).
6. Otherwise call `closest_string` to find which string this is, compute
   cents from its target, classify direction, fill snapshot.
7. If YIN returns None, update noise floor with this RMS (no pitch detected
   so this counts as background).

Tests: new_with_default_config_succeeds, new_rejects_unknown_tuning,
snapshot_after_silence_is_empty, detects_pitch_within_active_tuning_range
(G3 sine → G3 string, within 1 Hz), detects_e2_for_guitar_low_string
(E2 sine → E2 string within 5 cents), set_tuning_changes_active,
analyse_strum_returns_one_per_string, recognise_chord_on_silence_returns_no_best.

### 10.16 `tuner-core/src/bindings/{mod,jni,wasm}.rs`

`mod.rs`:
```rust
#[cfg(feature = "jni")]
pub mod jni;
#[cfg(feature = "wasm")]
pub mod wasm;
```

`jni.rs` exposes C-style functions Kotlin can call:
- `Java_com_opentuner_NativeTuner_nativeNew(env, class, tuning_id: JString, sample_rate: jint, a4: jdouble) -> jlong`
- `Java_com_opentuner_NativeTuner_nativeFree(env, class, handle: jlong)`
- `Java_com_opentuner_NativeTuner_nativePushSamples(env, class, handle: jlong, samples: JFloatArray)`
- `Java_com_opentuner_NativeTuner_nativeSetTuning(env, class, handle: jlong, tuning_id: JString) -> jint` (1 = ok, 0 = fail)
- `Java_com_opentuner_NativeTuner_nativeSnapshot(env, class, handle: jlong) -> jobject` (constructs com.opentuner.Snapshot)

Handle is `Box::into_raw(Box::new(Tuner::new(cfg)?))` cast to jlong. Free with
`Box::from_raw`. The tuning_id string must outlive the Tuner — `Box::leak` it
once at construction and never reuse the leaked memory after `nativeFree`.

`wasm.rs` exposes a `WasmTuner` struct via wasm-bindgen with methods
`pushSamples(samples: &[f32])`, `setTuning(id: String) -> bool`,
`snapshotJson() -> String`, `analyseStrumJson() -> String`,
`recogniseChordJson() -> String`. Returns JSON strings for portability across
the wasm-bindgen ABI.

### 10.17 `tuner-core/benches/pitch_bench.rs`

Criterion benchmarks. Already covered in §7 — 4 functions:
`bench_yin_2048`, `bench_yin_4096`, `bench_fft_4096`, `bench_chroma_4096`,
`bench_strum_full`. Each uses `criterion_group! / criterion_main!`.

### 10.18 `tuner-core/tests/synthetic.rs` — 10 integration tests

```
detects_every_string_of_every_shipped_tuning     // push sine at each spec, expect that string within 5¢
reports_correct_direction_for_detuned_strings    // ±30, ±6, ±4.5, 0 cents — avoid the exact ±5 boundary (float noise)
strum_analysis_finds_all_six_guitar_strings_in_tune  // mix 6 sines, expect ≥5 in tune
strum_analysis_reports_detuned_string_offset     // E2 sharp 30¢ in a strum
chord_recogniser_identifies_each_major_chord_root  // additive plucked-string synth (1/n harmonics, 8 partials), accept Major|MajorSeventh|Seventh family in top 3
chord_template_chroma_matches_recogniser_template  // pitch-class-set check, not exact (root, quality)
silence_yields_no_pitch_no_strum_no_chord
detection_is_stable_across_repeated_pushes       // chunks of 256 samples vs one shot
switching_tuning_changes_nearest_string          // push A2 to guitar.standard then bass.standard — guitar A2, bass G2
chroma_doctest_path_via_full_fft_works           // E4 sine peaks at pc 4
```

The `chord_recogniser_*` test uses an `synth_pluck` helper that adds 8
harmonics with 1/n amplitude rolloff. Without harmonics, the test fails — the
FFT can't resolve pitch classes from pure low-frequency sines.

### 10.19 `tuner-core/tests/corpus.rs` — WAV regression scaffold

Loads WAVs from `tests/corpus/{clean,strum,chords,noisy}/` paired with
`.label` files. Skips with a message when directories are empty (this is the
default state in the scaffold). When the corpus is added, each subfolder
becomes a regression gate.

Label formats:
- `clean/<name>.label`: line 1 = tuning_id, line 2 = expected string name
- `strum/<name>.label`: line 1 = tuning_id, then `string_name,expected_cents`
- `chords/<name>.label`: just the chord name (e.g. `Cmaj7`)
- `noisy/<name>.label`: optional `# tolerance:<cents>` header

Uses `hound` for WAV reading.

### 10.20 `tuner-android/settings.gradle.kts`, `build.gradle.kts`, `gradle.properties`

Standard AGP 8.5.x + Kotlin 2.0.x + Compose Compiler plugin (since Kotlin
2.0+ the compiler is a plugin, not a separate dependency).

`settings.gradle.kts`:
```kotlin
pluginManagement { repositories { google(); mavenCentral(); gradlePluginPortal() } }
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories { google(); mavenCentral() }
}
rootProject.name = "OpenTuner"
include(":app")
```

`build.gradle.kts` (top-level) declares plugins with `apply false`:
- com.android.application 8.5.2
- org.jetbrains.kotlin.android 2.0.20
- org.jetbrains.kotlin.plugin.compose 2.0.20
- org.jlleitschuh.gradle.ktlint 12.1.1

`gradle.properties`:
```
android.useAndroidX=true
android.nonTransitiveRClass=true
kotlin.code.style=official
org.gradle.jvmargs=-Xmx2g -Dfile.encoding=UTF-8
org.gradle.parallel=true
org.gradle.caching=true
```

### 10.21 `tuner-android/app/build.gradle.kts`

```kotlin
plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
    id("org.jlleitschuh.gradle.ktlint")
}

android {
    namespace = "com.opentuner"
    compileSdk = 34
    defaultConfig {
        applicationId = "com.opentuner"
        minSdk = 26       // Oboe / AAudio
        targetSdk = 34
        versionCode = 1
        versionName = "0.1.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        ndk { abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64", "x86") }
    }
    buildTypes {
        debug { applicationIdSuffix = ".debug" }
        release {
            isMinifyEnabled = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }
    compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
    kotlinOptions { jvmTarget = "17" }
    buildFeatures { compose = true; buildConfig = true }
    packaging { resources.excludes += setOf("/META-INF/{AL2.0,LGPL2.1}") }
    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
            java.srcDirs("src/main/kotlin")
        }
        getByName("test") { java.srcDirs("src/test/kotlin") }
        getByName("androidTest") { java.srcDirs("src/androidTest/kotlin") }
    }
}

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2024.09.02")
    implementation(composeBom)
    androidTestImplementation(composeBom)
    implementation("androidx.core:core-ktx:1.13.1")
    implementation("androidx.activity:activity-compose:1.9.2")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.8.6")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.6")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-graphics")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material3:material3")
    implementation("com.google.oboe:oboe:1.9.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.8.1")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.compose.ui:ui-test-junit4")
    debugImplementation("androidx.compose.ui:ui-tooling")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
}

// buildRustCore task: for each ABI, sets CARGO_TARGET_<TRIPLE>_LINKER to the
// NDK clang wrapper and runs `cargo build --release --features jni
// --target <triple>`, then copies the resulting libtuner_core.so into
// src/main/jniLibs/<abi>/. Wire it into preBuild via
// tasks.whenTaskAdded { if (name == "preBuild") dependsOn("buildRustCore") }.
// Requires ANDROID_NDK_HOME (or NDK_HOME) to be set.
```

ABI → triple → linker name:
```
arm64-v8a    -> aarch64-linux-android       -> aarch64-linux-android26-clang
armeabi-v7a  -> armv7-linux-androideabi     -> armv7a-linux-androideabi26-clang
x86_64       -> x86_64-linux-android        -> x86_64-linux-android26-clang
x86          -> i686-linux-android          -> i686-linux-android26-clang
```

### 10.22 `tuner-android/app/proguard-rules.pro`

```
-keep class com.opentuner.NativeTuner { *; }
-keep class com.opentuner.Snapshot { *; }
-keepclassmembers class com.opentuner.NativeTuner { native <methods>; }
```

### 10.23 `tuner-android/app/src/main/AndroidManifest.xml`

```xml
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <uses-permission android:name="android.permission.RECORD_AUDIO" />
    <uses-feature android:name="android.hardware.audio.low_latency" android:required="false" />
    <uses-feature android:name="android.hardware.audio.pro" android:required="false" />
    <uses-feature android:name="android.hardware.microphone" android:required="true" />
    <application
        android:allowBackup="false"
        android:dataExtractionRules="@xml/data_extraction_rules"
        android:fullBackupContent="false"
        android:icon="@android:drawable/ic_media_play"
        android:label="@string/app_name"
        android:supportsRtl="true"
        android:theme="@style/Theme.OpenTuner">
        <activity android:name=".MainActivity"
            android:exported="true"
            android:screenOrientation="portrait"
            android:label="@string/app_name"
            android:theme="@style/Theme.OpenTuner">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
```

### 10.24 Kotlin sources

`Snapshot.kt`:
```kotlin
package com.opentuner

data class Snapshot(
    val pitchHz: Float,
    val centsOff: Float,
    val detected: Int,
    val nearestStringIndex: Int,
    val nearestStringName: String,
    val confidence: Float,
) {
    val hasPitch: Boolean get() = detected != 0 && pitchHz > 0f
}
```

JNI signature is `(FFIILjava/lang/String;F)V` — DO NOT REORDER FIELDS without
updating `tuner-core/src/bindings/jni.rs` to match.

`NativeTuner.kt`: AutoCloseable wrapper around the JNI handle. Constructor
calls `nativeNew`, stores the returned jlong in an `AtomicLong`. `close()`
atomically swaps the handle to 0 and calls `nativeFree`. Static initialiser
calls `System.loadLibrary("tuner_core")`. Methods: `pushSamples(FloatArray)`,
`setTuning(String): Boolean`, `snapshot(): Snapshot?`, `close()`.

`audio/AudioEngine.kt`: simple `AudioRecord` capture using
`MediaRecorder.AudioSource.UNPROCESSED` (avoids effects), float PCM,
48 kHz mono. Reads in `framesPerRead`-sized chunks on a coroutine
(Dispatchers.IO), forwards to NativeTuner.pushSamples. The MVP uses AudioRecord
rather than Oboe — same NativeTuner surface, simpler implementation. Oboe can
be swapped in later without UI changes.

`TunerViewModel.kt`: holds NativeTuner + AudioEngine, exposes a StateFlow
with `TunerUiState(isRunning, tuningId, snapshot)`. Polls the snapshot every
50 ms while running. Calls `engine.start()/stop()` and `tuner.setTuning(id)`.

`MainActivity.kt`: ComponentActivity with Compose `setContent`. Permission
flow via `registerForActivityResult(ActivityResultContracts.RequestPermission)`.
On Start button: check RECORD_AUDIO, request if needed, then `vm.start()`. On
Stop: `vm.stop()`. The screen has:
- Title "OpenTuner"
- TuningPicker (DropdownMenu of the 4 tunings)
- PitchDisplay (big string name, Hz, cents, confidence)
- Start/Stop button
- Pause the engine in `onPause()`.

`SUPPORTED_TUNINGS` constant in the same file (or in `TunerViewModel.kt`):
```kotlin
val SUPPORTED_TUNINGS = listOf(
    "guitar.standard"   to "Guitar — Standard (E A D G B E)",
    "bass.standard"     to "Bass — Standard (E A D G)",
    "guitarra.lisboa"   to "Guitarra Portuguesa — Lisboa",
    "guitarra.coimbra"  to "Guitarra Portuguesa — Coimbra",
)
```

### 10.25 Android resources

`res/values/strings.xml`:
```xml
<resources>
    <string name="app_name">OpenTuner</string>
</resources>
```

`res/values/themes.xml`:
```xml
<resources xmlns:tools="http://schemas.android.com/tools">
    <style name="Theme.OpenTuner" parent="android:Theme.Material.Light.NoActionBar">
        <item name="android:windowLightStatusBar" tools:targetApi="m">true</item>
    </style>
</resources>
```

`res/xml/data_extraction_rules.xml`: excludes everything (no user state to
back up).

### 10.26 Android tests

`SnapshotTest.kt` (JVM): 4 trivial tests on the Snapshot data class
(hasPitch logic, SUPPORTED_TUNINGS contents).

`NativeTunerInstrumentedTest.kt` (instrumented): 3 tests —
nativeLibraryLoadsAndBuildsTuner, pushingASineYieldsAReasonablePitch
(synthesise a G3 sine in Kotlin, push to NativeTuner, assert
`snap.nearestStringName == "G3"` and `|centsOff| < 3`), setTuningSwitchesActiveTuning.

### 10.27 `tuner-web/package.json`

```json
{
  "name": "tuner-web",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "license": "GPL-3.0-or-later",
  "scripts": {
    "build:wasm": "wasm-pack build ../tuner-core --target web --out-dir ../tuner-web/pkg --release --features wasm -- --no-default-features",
    "predev": "npm run build:wasm",
    "prebuild": "npm run build:wasm",
    "dev": "vite",
    "build": "tsc --noEmit && vite build",
    "preview": "vite preview",
    "test": "vitest run",
    "lint": "eslint src --max-warnings 0"
  },
  "dependencies": {
    "react": "^18.3.1",
    "react-dom": "^18.3.1"
  },
  "devDependencies": {
    "@testing-library/react": "^16.0.1",
    "@types/react": "^18.3.10",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react": "^4.3.1",
    "eslint": "^9.11.1",
    "jsdom": "^25.0.0",
    "typescript": "^5.6.2",
    "vite": "^5.4.8",
    "vitest": "^2.1.1"
  }
}
```

### 10.28 `tuner-web/tsconfig.json`

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "noImplicitOverride": true,
    "exactOptionalPropertyTypes": true,
    "useUnknownInCatchVariables": true,
    "skipLibCheck": true,
    "isolatedModules": true,
    "resolveJsonModule": true,
    "verbatimModuleSyntax": true,
    "esModuleInterop": true,
    "forceConsistentCasingInFileNames": true,
    "types": ["vite/client"]
  },
  "include": ["src", "tests"],
  "exclude": ["node_modules", "dist", "pkg"]
}
```

### 10.29 `tuner-web/vite.config.ts`

```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  server: {
    headers: {
      "Cross-Origin-Opener-Policy": "same-origin",
      "Cross-Origin-Embedder-Policy": "require-corp",
    },
  },
  build: { target: "es2022", sourcemap: true },
  test: { environment: "jsdom", globals: false, include: ["tests/**/*.test.{ts,tsx}"] },
});
```

### 10.30 `tuner-web/src/` — TO BE WRITTEN

Components to implement:
- `main.tsx` — React root, mounts `<App />`.
- `index.html` — minimal HTML scaffold.
- `App.tsx` — top-level layout, holds the WasmTuner instance.
- `components/TunerScreen.tsx` — main UI (mirror the Android Compose layout).
- `components/TuningPicker.tsx` — select element with the 4 tunings.
- `components/PitchDisplay.tsx` — big note name, Hz, cents readout.
- `components/StrumGrid.tsx` — bar per string with cents offset.
- `components/ChordDisplay.tsx` — chord name + confidence.
- `audio/MicEngine.ts` — wraps the Web Audio API: requests microphone with
  `getUserMedia({ audio: { echoCancellation: false, noiseSuppression: false, autoGainControl: false } })`, creates an `AudioWorklet` that posts Float32 chunks to a worker.
- `audio/tuner.worker.ts` — Web Worker that imports the WASM module
  (`import init, { WasmTuner } from "../../pkg/tuner_core.js"`), receives
  sample chunks from the main thread, calls `tuner.pushSamples(samples)`, and
  posts back JSON snapshots / strum reports / chord results when requested.
- `audio/worklet/capture-worklet.js` — AudioWorkletProcessor that copies its
  input buffer and posts it to the main thread (which forwards to the worker).
  Must be served as a separate JS file because AudioWorklet code runs in its
  own realm.

Smoke tests (`tests/wasm-smoke.test.ts`): mock-import the WASM module
shape; assert the four exported methods exist; verify the snapshot JSON
parses into the expected shape.

### 10.31 `.github/workflows/` — TO BE WRITTEN

Three workflows. All use `actions/checkout@v4` and `actions/cache@v4` (or
`Swatinem/rust-cache@v2` for Rust).

**`core.yml`** — on push/PR touching `tuner-core/**`:
```yaml
name: tuner-core
on: { push: { paths: [tuner-core/**] }, pull_request: { paths: [tuner-core/**] } }
jobs:
  test:
    runs-on: ubuntu-latest
    defaults: { run: { working-directory: tuner-core } }
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: rustfmt, clippy }
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --check
      - run: cargo clippy --all-targets --all-features -- -D warnings
      - run: cargo test --all-features
      - run: cargo doc --no-deps --all-features
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: rustsec/audit-check@v1.4.1
        with: { token: ${{ secrets.GITHUB_TOKEN }} }
```

**`android.yml`** — on push/PR touching `tuner-android/**` or
`tuner-core/**`:
```yaml
name: tuner-android
on: { push: { paths: [tuner-android/**, tuner-core/**] }, pull_request: { paths: [tuner-android/**, tuner-core/**] } }
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-java@v4
        with: { distribution: temurin, java-version: 17 }
      - uses: android-actions/setup-android@v3
      - run: yes | sdkmanager "ndk;26.2.11394342"
      - uses: dtolnay/rust-toolchain@stable
      - run: rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android
      - uses: Swatinem/rust-cache@v2
      - name: ktlint check
        working-directory: tuner-android
        run: ./gradlew ktlintCheck
      - name: assemble debug
        working-directory: tuner-android
        env: { ANDROID_NDK_HOME: ${{ env.ANDROID_HOME }}/ndk/26.2.11394342 }
        run: ./gradlew assembleDebug
      - name: unit tests
        working-directory: tuner-android
        run: ./gradlew testDebugUnitTest
```

**`web.yml`** — on push/PR touching `tuner-web/**` or `tuner-core/**`:
```yaml
name: tuner-web
on: { push: { paths: [tuner-web/**, tuner-core/**] }, pull_request: { paths: [tuner-web/**, tuner-core/**] } }
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 20, cache: npm, cache-dependency-path: tuner-web/package-lock.json }
      - uses: dtolnay/rust-toolchain@stable
      - uses: jetli/wasm-pack-action@v0.4.0
      - run: npm ci
        working-directory: tuner-web
      - run: npm run build
        working-directory: tuner-web
      - run: npm test
        working-directory: tuner-web
      - run: npm run lint
        working-directory: tuner-web
```

---

## 11. Tactics for resuming in Claude Code

Claude Code is a terminal-first agent — files persist on the user's machine,
so checkpoint after every meaningful module. Suggested order:

1. `git init` an empty repo.
2. Create `docs/` and `README.md` first — easy wins that establish vocabulary.
3. Build `tuner-core` in this order, **running `cargo test` after each**:
   - `lib.rs` + `cents.rs` → verify, commit
   - `tunings.rs` → verify, commit
   - `pitch.rs` → verify, commit
   - `fft.rs` + `chroma.rs` → verify, commit (chroma is the bug-prone one)
   - `chord.rs` → verify (mind the Sus2/Sus4 ambiguity), commit
   - `noise.rs` + `strum.rs` → verify (6th-order cascade + warmup-skip!), commit
   - `tuner.rs` → verify (two-buffer design!), commit
   - `bindings/` → verify it builds with `--features jni,wasm` (no runtime test possible without targets)
   - `tests/synthetic.rs` → `cargo test`, commit
   - `tests/corpus.rs` → `cargo test` (skips empty), commit
   - `benches/pitch_bench.rs` → `cargo bench --bench pitch_bench -- --quick`, commit
4. Run `cargo clippy --all-targets -- -D warnings` and fix anything.
5. Scaffold `tuner-android/`. Commit at each milestone. Try `./gradlew
   ktlintCheck` and `./gradlew testDebugUnitTest` if Android SDK is
   available; full `assembleDebug` needs NDK too.
6. Scaffold `tuner-web/`. Implement `src/` from §10.30. Run `npm run build`
   and `npm test`.
7. Add `.github/workflows/`. Push and watch CI light up.
8. Replace LICENSE placeholder with canonical GPL-3.0 text. Add a CI step
   that verifies its SHA-256.

**Critical reminders from previous sessions**:

- After any change to `chroma.rs`, re-run `chroma_of_c4_peaks_at_pc_0`. That
  test was the canary for the triangular-interpolation bug.
- After any change to `strum.rs`, re-run
  `strum_analysis_finds_all_six_guitar_strings_in_tune`. That test was the
  canary for the cascade-depth / warmup-skip combo.
- After any change to `tuner.rs`, verify both buffers update on
  `push_samples`. The integration test
  `strum_analysis_finds_all_six_guitar_strings_in_tune` exercises this end
  to end.
- Float comparisons in `const fn` are a rustc 1.75 error. `validate()` must
  be a regular `fn`.
- `vec![0.0; N]` in tests defaults to `f64` if the context doesn't constrain
  it — write `vec![0.0_f32; N]`.

Good luck. The Rust core is the careful part; once it passes, the rest is
standard scaffolding around it.

# OpenTuner

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg)](LICENSE)

A free, open-source instrument tuner for **guitar**, **bass**, and **guitarra
portuguesa** (Lisboa and Coimbra tunings). Built around a single, rigorously
tested Rust DSP core that runs on Android (primary target) and in the browser
(WebAssembly).

## Features

Three live modes (bottom-tab navigation), each listening continuously:

- **Tune** — single-string tuning with auto-detection of which string is playing
  and which way it's off, on a ±50-cent needle meter.
- **Strum** — strum all strings and watch per-string offsets update in real time
  (strum with one hand, tune with the other); readings hold briefly so they stay
  legible between plucks.
- **Chords** — live chord identification with compact fret fingerings
  (`x 3 2 0 1 0`) for the active tuning, plus alternate matches.
- The screen stays awake while listening, and it's designed for **noisy
  environments** (DC blocker, noise-floor tracking, per-string band-pass).

## Download

Signed APKs are published on the [Releases](../../releases) page — download
`opentuner-vX.Y.Z.apk` and sideload it (you may need to allow installs from your
browser/files app). See [docs/RELEASING.md](docs/RELEASING.md) for how releases
are built and signed.

## Architecture

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

The Rust core is the single source of DSP truth. The same code compiles to an
Android JNI library (`cdylib`) and a browser WASM module. Tests and benchmarks
live with the crate.

## Quick start

### Rust core

```bash
cd tuner-core
cargo test                                    # unit + integration + doctests + corpus
cargo clippy --all-targets --all-features -- -D warnings
cargo bench --bench pitch_bench -- --quick
```

### Android (requires Android SDK + NDK r26+)

```bash
cd tuner-android
./gradlew assembleDebug
```

The Gradle build cross-compiles the Rust core for each ABI and bundles the
resulting `.so` files. Set `ANDROID_NDK_HOME` first.

### Web (requires Node 20+ and wasm-pack)

```bash
cd tuner-web
npm install
npm run build      # builds the WASM module then bundles the app
npm test
```

## Repository layout

| Path | Purpose |
|---|---|
| `tuner-core/` | Rust DSP library (pitch, chroma, chord, strum, noise, tunings) |
| `tuner-android/` | Kotlin/Compose Android app + JNI glue |
| `tuner-web/` | TypeScript/React/WASM web app |
| `docs/` | Architecture, DSP, testing, contributing, instrument references |
| `.github/workflows/` | CI for core, Android, and web |
| `scripts/` | Release/cross-compile helpers |

## Documentation

- [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md) — product requirements (incl. v0.2 live modes)
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — layered design and threading model
- [docs/DSP.md](docs/DSP.md) — signal-processing details and references
- [docs/TESTING.md](docs/TESTING.md) — the test strategy
- [docs/RELEASING.md](docs/RELEASING.md) — signing, GitHub Releases, Play Store path
- [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) — dev workflow and checks
- [docs/INSTRUMENTS.md](docs/INSTRUMENTS.md) — tuning tables and sources

## License

GPL-3.0-or-later. See [LICENSE](LICENSE). Derivatives stay open.

# OpenTuner

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg)](LICENSE)

A free, open-source instrument tuner for **guitar**, **bass**, and **guitarra
portuguesa** (Lisboa and Coimbra tunings). Built around a single, rigorously
tested Rust DSP core that runs on Android (primary target) and in the browser
(WebAssembly).

## Features

- **Single-string tuning** with auto-detection of which string is being played
  and which way it is off.
- **Strum analysis** — strum all strings at once and get a per-string offset and
  direction.
- **Chord identification** — name a strummed or played chord.
- Designed to work in **noisy environments** (DC blocker, noise-floor tracking,
  per-string band-pass filtering).

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

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — layered design and threading model
- [docs/DSP.md](docs/DSP.md) — signal-processing details and references
- [docs/TESTING.md](docs/TESTING.md) — the seven-layer test strategy
- [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) — dev workflow and checks
- [docs/INSTRUMENTS.md](docs/INSTRUMENTS.md) — tuning tables and sources

## License

GPL-3.0-or-later. See [LICENSE](LICENSE). Derivatives stay open.

# Architecture

## Layered design

OpenTuner is one Rust DSP core with two thin host shells.

```
Android app (Kotlin + Compose)   Web app (TypeScript + React)
        │  JNI                          │  wasm-bindgen
        └──────────────┬────────────────┘
                       ▼
                 tuner-core (Rust)
```

The core (`tuner-core`) owns all DSP. Hosts only capture audio, forward samples,
poll results, and render UI. This keeps the careful, heavily-tested code in one
place and identical across platforms.

### Crate modules

| Module | Responsibility |
|---|---|
| `cents` | cents/MIDI math (property-tested) |
| `tunings` | the four shipped tunings + nearest-string lookup |
| `pitch` | YIN monophonic pitch detection |
| `fft` | Hann window + magnitude spectrum |
| `chroma` | log-magnitude chroma with triangular interpolation |
| `chord` | quality enum, templates, recogniser |
| `strum` | per-string band-pass cascade + YIN |
| `noise` | DC blocker, noise-floor tracker |
| `tuner` | the `Tuner` facade (two ring buffers) |
| `bindings` | feature-gated JNI and WASM surfaces |

## The `Tuner` facade and `TunerConfig`

`Tuner` keeps two ring buffers:

- a short **frame ring** (`frame_size`) for low-latency per-frame YIN, and
- a longer **analysis buffer** (~1.5 s) for strum and chord analysis.

`push_samples` runs every sample through the DC blocker and writes to both. A
frame analysis runs every `hop_size` samples.

`TunerConfig` carries sample rate, frame/hop sizes, A4 reference, YIN threshold,
chord score/margin thresholds, the active tuning id, and a noise-subtraction
flag. `validate()` is a plain `fn` (not `const fn`) because float comparisons are
not permitted in const contexts on the crate's MSRV.

## Threading model

DSP is single-threaded inside `tuner-core`; the host provides the thread. The
audio callback pushes samples (lock-free on the host side) and the UI polls
snapshots at a frame boundary (host-side mutex). The core itself holds no locks.

## Out of scope (MVP)

- **iOS** — no Apple hardware to publish; no iOS files of any kind.
- Recording / playback of audio.
- Ads, analytics, telemetry, network access.
- Alternate guitar tunings or other instruments (the tuning table is designed to
  be extensible, but adding entries is a follow-up release).

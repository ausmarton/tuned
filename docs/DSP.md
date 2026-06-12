# DSP design

This document describes the signal-processing pipeline in `tuner-core`. It is
the reference that golden-test expected values are derived from — **any change
here must be reflected in the tests and vice versa.**

## Sample rates and frames

- Sample rate: **48 kHz** (mono).
- Analysis frame: **4096 samples** (~85 ms).
- Hop: **2048 samples** (50% overlap).
- A longer **~1.5 s analysis buffer** backs strum and chord analysis, which need
  low frequencies to settle through the band-pass cascade.

## Pipeline

```
input ─► DC blocker ─► ring buffer (per-frame YIN)    ─► snapshot
                   └─► analysis buffer ─► strum bandpass+YIN ─► strum report
                                      └─► Hann FFT ─► chroma ─► chord recogniser
```

## Monophonic pitch — YIN

YIN (de Cheveigné & Kawahara, 2002, *YIN, a fundamental frequency estimator for
speech and music*, JASA 111(4)).

```
1. d[τ]  = Σ_i (x[i] − x[i+τ])²                         (difference function)
2. d'[τ] = d[τ]·τ / Σ_{j≤τ} d[j]   ,  d'[0] = 1          (cumulative mean normalised)
3. pick first τ ≥ τ_min with d'[τ] < threshold, walk down to local min
   (fallback: global min in range if it is < 1.0)
4. parabolic interpolation around τ for sub-sample accuracy
5. f0 = sample_rate / τ_refined
```

`τ_max = ceil(sr / min_hz)`; the buffer must hold `2·τ_max + 1` samples.
Default threshold 0.12.

### Confidence

```
periodicity  = max(0, 1 − min(1, aperiodicity / 0.6))
signal_ratio = clamp(rms / (10·noise_floor), 0, 1)      (1 if noise_floor == 0)
confidence   = periodicity · signal_ratio
```

## Cents

`cents = 1200 · log2(measured / target)`, returning 0 for non-positive input.
MIDI↔Hz uses `f = a4 · 2^((m − 69)/12)` with configurable A4 (default 440 Hz).

## Strum — per-string band-pass + YIN

Each string is isolated with a band-pass centred on its target, then YIN runs on
the filtered signal.

- Band edges: `target · 2^(±band_half_cents/1200)` (default ±100 cents).
- Filter: **three cascaded RBJ biquad band-passes (6th order, ~36 dB/oct)**. Two
  biquads (4th order) leaked neighbouring strings and caused misdetection.
- **The first half of the filtered buffer is discarded before YIN** — the
  cascade's transient ringing wrecks low-frequency detection otherwise.
- Per-string confidence uses a band-vs-total RMS ratio: a well-tuned string
  concentrates its energy in its own band.

RBJ band-pass (constant skirt gain, peak gain = Q):

```
w0    = 2π·centre/sr ,  α = sin(w0)/(2Q)
b0=α, b1=0, b2=−α ,  a0=1+α, a1=−2cos(w0), a2=1−α        (then divide by a0)
Q     = clamp( 2^(bw/2) / (2^bw − 1) , 0.3, 20 ) ,  bw = log2(high/low)
```

## Chord — log-magnitude chroma + templates

Chroma (Müller, 2007, *Information Retrieval for Music and Motion*).

- Hann-windowed FFT magnitude → map each in-band bin to a (fractional) MIDI
  number → pitch class.
- **Triangular interpolation**: each bin contributes `ln(1+m)` energy split
  between the two adjacent pitch classes by fractional distance. Nearest-semitone
  rounding put energy on the wrong side of a pitch-class boundary when a partial
  fell between two bins straddling a semitone line (e.g. C4 rounding to C#).
- Normalise to unit sum.
- Match against **12 roots × 9 qualities = 108** binary templates by cosine
  similarity. Qualities: major, minor, 7, maj7, m7, sus2, sus4, dim, aug.
- A confident "best" requires `score ≥ min_score` (0.85) **and** a margin over
  the runner-up `≥ min_margin` (0.05). Enharmonic chords (Csus2 == Gsus4) tie and
  so correctly report no single winner.

## Chord voicings (fret diagrams)

`fretboard.rs` turns a recognised chord into playable fingerings for the active
tuning by searching the fretboard. Per string, the candidate frets are those in
`0..=max_fret` whose pitch class is in the chord, plus "muted"; a depth-first
search assembles voicings, pruned by hand span and finger count. A voicing is
kept when only chord tones sound, every chord tone is covered, and the shape is
playable (span ≤ 4 frets, ≤ 4 fingered strings, ≥ max(3, #tones) sounding).

Voicings are ranked toward idiomatic shapes — in priority: no interior muted
strings, root in the bass, lowest position, fewest fingers, fuller, least total
fret distance — and the top few are returned. The search is tuning-agnostic
(guitar, bass, guitarra portuguesa) and cheap enough to run live. The compact
display form (`x 3 2 0 1 0`) is the voicing's fret-per-string array directly.

## Live display smoothing (UI layer)

Real-time strum/chord modes poll the same `analyse_strum()` / `recognise_chord()`
continuously (~10 Hz). Smoothing lives in each shell (not the stateless core),
with identical parameters:

- **Strum:** per-string hold of the last reading with `confidence ≥ 0.2` for up
  to 2 s, fading with age — so a string stays readable between pluck transients.
- **Chord:** a new best is shown only after it persists ≥ 250 ms (debounce); the
  last chord is held up to 1.5 s across brief gaps — preventing flicker.

## Noise handling

- **DC blocker**: single-pole high-pass `y[n] = x[n] − x[n−1] + p·y[n−1]`,
  `p = 0.995` (≈38 Hz corner at 48 kHz).
- **Noise floor**: exponentially-smoothed RMS that only updates on frames within
  6 dB of the current estimate, so sustained notes don't raise it.

## Latency budget

All hot paths run well inside a 50 ms budget on a developer machine: YIN/4096
≈ 1 ms, FFT/4096 ≈ 55 µs, chroma ≈ 4 µs, full 6-string strum ≈ 12 ms.

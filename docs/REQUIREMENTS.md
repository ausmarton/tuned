# Requirements

The original MVP intent is captured in the root `REQUIREMENTS.md`. This document
records the **v0.2** product requirements precisely enough to build and test
against. (The platform priority is unchanged: Android first, web at parity,
no iOS.)

## Modes

The app presents three modes via bottom-tab navigation; each listens
continuously while open. The microphone stays live as long as a mode is on and
the app is foreground.

### Tune (single string)

- Continuous per-frame pitch detection.
- Shows the nearest string of the active tuning, the detected frequency, the
  cents offset, and direction (flat / in tune / sharp), with a ±50-cent needle
  meter, colour-coded (green in tune, orange off).
- "In tune" window: ±5 cents.
- Idle (`—`) on silence / low confidence.

### Strum (live, all strings)

- Analyses the rolling ~1.5 s buffer at ~10 Hz — **no manual trigger**. The
  player strums continuously with the right hand and tunes with the left while
  watching real-time feedback, so the loudest part of a strum is never missed.
- Shows **every** string of the active tuning with its latest cents offset,
  direction, and a per-string needle.
- **Smoothing:** each string holds its last confident reading for up to 2 s and
  fades toward neutral as the reading ages; a reading counts only when its
  confidence ≥ 0.2. This keeps the display legible between pluck transients.

### Chords (live)

- Recognises the chord from the rolling buffer at ~10 Hz.
- Displays the current best chord name and confidence; shows up to two alternate
  matches within a 0.08 score margin.
- **Smoothing:** a new best chord is shown only after it persists ≥ 250 ms
  (debounce); the last shown chord is held up to 1.5 s across brief gaps, so the
  readout doesn't flicker.
- **Fret diagrams:** for the displayed chord (and alternates) shows compact
  numeric fingerings for the **active tuning** — `x 3 2 0 1 0` style, labelled
  with string names. Up to three voicings per chord. Tunings with no playable
  voicing (e.g. most chords on bass) show the name only.

## Keep awake

- While any mode is listening and the app is foreground, the screen is kept on
  (Android `keepScreenOn`; web Screen Wake Lock). This lets the player focus on
  the instrument and fixes the bug where screen-off made the mic unavailable and
  required an app restart. The lock is released when listening stops or the app
  is backgrounded, and re-acquired on resume.

## Voicing generation (rigour)

A voicing is "playable" when, on the active tuning:
- only chord tones sound (no out-of-chord notes),
- every chord tone is present,
- the hand span between the lowest and highest fretted note ≤ 4 frets,
- at most 4 fingered (fret > 0) strings,
- at least `max(3, number of chord tones)` strings sound.

Voicings are ranked toward idiomatic shapes: no interior muted strings, root in
the bass, low on the neck, few fingers, fuller. (See `docs/DSP.md`.)

## Distribution

- Signed release APKs are published as GitHub Releases on `v*` tags (see
  `docs/RELEASING.md`). Play Store is a documented follow-up.

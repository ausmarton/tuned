# Instruments and tunings

All frequencies derive from MIDI assuming A4 = 440 Hz
(`f = 440 · 2^((m − 69)/12)`). Strings are listed lowest pitch first.

## 6-string guitar — standard

| String | Note | MIDI | Hz |
|---|---|---|---|
| 1 | E2 | 40 | 82.41 |
| 2 | A2 | 45 | 110.00 |
| 3 | D3 | 50 | 146.83 |
| 4 | G3 | 55 | 196.00 |
| 5 | B3 | 59 | 246.94 |
| 6 | E4 | 64 | 329.63 |

## 4-string bass — standard

| String | Note | MIDI | Hz |
|---|---|---|---|
| 1 | E1 | 28 | 41.20 |
| 2 | A1 | 33 | 55.00 |
| 3 | D2 | 38 | 73.42 |
| 4 | G2 | 43 | 98.00 |

## Guitarra portuguesa — Lisboa

Tuning `DABEAB` (low→high): D3 A3 B3 E4 A4 B4 — MIDI 50, 57, 59, 64, 69, 71.
Courses 1–3 are unison pairs; courses 4–6 are octave-paired. We store the
nominal (lower) note of each course; an octave partner appears as a strong 2nd
harmonic, which is exactly what a well-tuned octave course produces.

| Course | Note | MIDI | Hz |
|---|---|---|---|
| 1 | D3 | 50 | 146.83 |
| 2 | A3 | 57 | 220.00 |
| 3 | B3 | 59 | 246.94 |
| 4 | E4 | 64 | 329.63 |
| 5 | A4 | 69 | 440.00 |
| 6 | B4 | 71 | 493.88 |

Source: Tobe Richards, *The Portuguese Guitar Chord Bible: Lisboa Tuning* (2016),
ISBN 978-1906207434.

## Guitarra portuguesa — Coimbra

Tuning `CGDAGA` (low→high): C3 G3 A3 D4 G4 A4 — MIDI 48, 55, 57, 62, 67, 69.
Exactly one whole tone (two semitones) below Lisboa on every string.

| Course | Note | MIDI | Hz |
|---|---|---|---|
| 1 | C3 | 48 | 130.81 |
| 2 | G3 | 55 | 196.00 |
| 3 | A3 | 57 | 220.00 |
| 4 | D4 | 62 | 293.66 |
| 5 | G4 | 67 | 392.00 |
| 6 | A4 | 69 | 440.00 |

Source: Tobe Richards, *The Portuguese Guitar Chord Bible: Coimbra Tuning* (2016).

## Scope

The MVP ships exactly these four tunings. The data model
(`pub const ALL: &[Tuning]`) is designed to be extended, but adding instruments
or alternate guitar tunings is a follow-up release.

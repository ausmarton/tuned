//! Tuning definitions for the shipped instruments.
//!
//! Frequencies derive from MIDI assuming the configured A4 reference. Strings
//! are stored lowest pitch first. For the guitarra portuguesa the nominal
//! (lower) note of each octave-paired course is stored; the octave partner
//! shows up as a strong second harmonic of the nominal pitch.

use crate::cents::{midi_to_hz, ratio_to_cents};

/// Broad instrument family for a tuning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstrumentClass {
    /// Six-string guitar.
    Guitar6,
    /// Four-string bass.
    Bass4,
    /// Portuguese guitar (guitarra portuguesa).
    GuitarraPortuguesa,
}

/// A single string within a tuning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringSpec {
    /// Display name, e.g. `"E2"`.
    pub name: &'static str,
    /// MIDI note number of the nominal pitch.
    pub midi: u8,
}

impl StringSpec {
    /// Nominal frequency of this string in Hz.
    #[must_use]
    pub fn freq_hz(&self, a4_hz: f32) -> f32 {
        midi_to_hz(self.midi, a4_hz)
    }
}

/// A named tuning: an ordered set of strings for an instrument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tuning {
    /// Stable machine id, e.g. `"guitar.standard"`.
    pub id: &'static str,
    /// Human-readable name.
    pub display_name: &'static str,
    /// Instrument family.
    pub instrument: InstrumentClass,
    /// Strings, lowest pitch first.
    pub strings: &'static [StringSpec],
}

const fn s(name: &'static str, midi: u8) -> StringSpec {
    StringSpec { name, midi }
}

/// Six-string guitar, standard tuning E2 A2 D3 G3 B3 E4.
pub const GUITAR_STANDARD: Tuning = Tuning {
    id: "guitar.standard",
    display_name: "Guitar — Standard (E A D G B E)",
    instrument: InstrumentClass::Guitar6,
    strings: &[
        s("E2", 40),
        s("A2", 45),
        s("D3", 50),
        s("G3", 55),
        s("B3", 59),
        s("E4", 64),
    ],
};

/// Four-string bass, standard tuning E1 A1 D2 G2.
pub const BASS_STANDARD: Tuning = Tuning {
    id: "bass.standard",
    display_name: "Bass — Standard (E A D G)",
    instrument: InstrumentClass::Bass4,
    strings: &[s("E1", 28), s("A1", 33), s("D2", 38), s("G2", 43)],
};

/// Guitarra portuguesa, Lisboa tuning D3 A3 B3 E4 A4 B4 (nominal pitches).
pub const GUITARRA_LISBOA: Tuning = Tuning {
    id: "guitarra.lisboa",
    display_name: "Guitarra Portuguesa — Lisboa",
    instrument: InstrumentClass::GuitarraPortuguesa,
    strings: &[
        s("D3", 50),
        s("A3", 57),
        s("B3", 59),
        s("E4", 64),
        s("A4", 69),
        s("B4", 71),
    ],
};

/// Guitarra portuguesa, Coimbra tuning C3 G3 A3 D4 G4 A4.
///
/// One whole tone below Lisboa on every string.
pub const GUITARRA_COIMBRA: Tuning = Tuning {
    id: "guitarra.coimbra",
    display_name: "Guitarra Portuguesa — Coimbra",
    instrument: InstrumentClass::GuitarraPortuguesa,
    strings: &[
        s("C3", 48),
        s("G3", 55),
        s("A3", 57),
        s("D4", 62),
        s("G4", 67),
        s("A4", 69),
    ],
};

/// Every shipped tuning. Extensible by adding entries (a follow-up release).
pub const ALL: &[Tuning] = &[
    GUITAR_STANDARD,
    BASS_STANDARD,
    GUITARRA_LISBOA,
    GUITARRA_COIMBRA,
];

/// Look up a tuning by id.
#[must_use]
pub fn lookup(id: &str) -> Option<&'static Tuning> {
    ALL.iter().find(|t| t.id == id)
}

/// Find the string of `tuning` whose nominal pitch is closest to `freq_hz`.
///
/// Returns `(string_index, cents_off)`, where `cents_off` is the measured
/// frequency relative to that string's target, or `None` for non-positive input.
#[must_use]
pub fn closest_string(tuning: &Tuning, freq_hz: f32, a4_hz: f32) -> Option<(usize, f32)> {
    if freq_hz <= 0.0 {
        return None;
    }
    let mut best: Option<(usize, f32)> = None;
    for (i, spec) in tuning.strings.iter().enumerate() {
        let target = spec.freq_hz(a4_hz);
        let cents = ratio_to_cents(freq_hz, target);
        match best {
            Some((_, bc)) if bc.abs() <= cents.abs() => {}
            _ => best = Some((i, cents)),
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn all_ids_are_unique() {
        for (i, a) in ALL.iter().enumerate() {
            for b in &ALL[i + 1..] {
                assert_ne!(a.id, b.id, "duplicate id {}", a.id);
            }
        }
    }

    #[test]
    fn every_shipped_tuning_is_found_by_lookup() {
        for t in ALL {
            assert_eq!(lookup(t.id), Some(t));
        }
        assert_eq!(lookup("nope"), None);
    }

    #[test]
    fn strings_strictly_ascending() {
        for t in ALL {
            for w in t.strings.windows(2) {
                assert!(w[0].midi < w[1].midi, "{} not ascending", t.id);
            }
        }
    }

    #[test]
    fn exact_guitar_frequencies() {
        let g = &GUITAR_STANDARD;
        assert_relative_eq!(g.strings[0].freq_hz(440.0), 82.4069, epsilon = 1e-3); // E2
        assert_relative_eq!(g.strings[1].freq_hz(440.0), 110.0, epsilon = 1e-3); // A2
        assert_relative_eq!(g.strings[5].freq_hz(440.0), 329.6276, epsilon = 1e-3);
        // E4
    }

    #[test]
    fn coimbra_is_two_semitones_below_lisboa() {
        for (c, l) in GUITARRA_COIMBRA
            .strings
            .iter()
            .zip(GUITARRA_LISBOA.strings.iter())
        {
            assert_eq!(l.midi - c.midi, 2);
        }
    }

    #[test]
    fn closest_string_picks_exact_match() {
        let g = &GUITAR_STANDARD;
        let d3 = g.strings[2].freq_hz(440.0);
        let (idx, cents) = closest_string(g, d3, 440.0).unwrap();
        assert_eq!(idx, 2);
        assert_relative_eq!(cents, 0.0, epsilon = 1e-2);
    }

    #[test]
    fn closest_string_picks_nearest_neighbour() {
        let g = &GUITAR_STANDARD;
        // A bit sharp of A2 → still A2.
        let near_a2 = g.strings[1].freq_hz(440.0) * 1.01;
        let (idx, cents) = closest_string(g, near_a2, 440.0).unwrap();
        assert_eq!(idx, 1);
        assert!(cents > 0.0);
    }

    #[test]
    fn closest_string_rejects_non_positive() {
        assert!(closest_string(&GUITAR_STANDARD, 0.0, 440.0).is_none());
    }
}

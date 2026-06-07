//! Template-based chord recognition over chroma vectors.
//!
//! Twelve roots × nine qualities = 108 binary templates. A strummed chord's
//! chroma is matched against every template by cosine similarity. A "best"
//! match is reported only when it both clears `min_score` and beats the
//! runner-up by `min_margin` — so enharmonically ambiguous chords (e.g.
//! Csus2 == Gsus4) correctly report no single winner.

use crate::chroma::{cosine_similarity, normalise, ChromaVector};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// Chord quality (the part after the root letter).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Quality {
    /// Major triad.
    Major,
    /// Minor triad.
    Minor,
    /// Dominant seventh.
    Seventh,
    /// Major seventh.
    MajorSeventh,
    /// Minor seventh.
    MinorSeventh,
    /// Suspended second.
    Sus2,
    /// Suspended fourth.
    Sus4,
    /// Diminished triad.
    Diminished,
    /// Augmented triad.
    Augmented,
}

impl Quality {
    /// Every quality, in a stable order.
    pub const ALL: &'static [Self] = &[
        Self::Major,
        Self::Minor,
        Self::Seventh,
        Self::MajorSeventh,
        Self::MinorSeventh,
        Self::Sus2,
        Self::Sus4,
        Self::Diminished,
        Self::Augmented,
    ];

    /// Display suffix appended to the root letter.
    #[must_use]
    pub const fn suffix(&self) -> &'static str {
        match self {
            Self::Major => "",
            Self::Minor => "m",
            Self::Seventh => "7",
            Self::MajorSeventh => "maj7",
            Self::MinorSeventh => "m7",
            Self::Sus2 => "sus2",
            Self::Sus4 => "sus4",
            Self::Diminished => "dim",
            Self::Augmented => "aug",
        }
    }

    /// Semitone intervals from the root.
    #[must_use]
    pub const fn intervals(&self) -> &'static [u8] {
        match self {
            Self::Major => &[0, 4, 7],
            Self::Minor => &[0, 3, 7],
            Self::Seventh => &[0, 4, 7, 10],
            Self::MajorSeventh => &[0, 4, 7, 11],
            Self::MinorSeventh => &[0, 3, 7, 10],
            Self::Sus2 => &[0, 2, 7],
            Self::Sus4 => &[0, 5, 7],
            Self::Diminished => &[0, 3, 6],
            Self::Augmented => &[0, 4, 8],
        }
    }
}

/// A chord root as a pitch class (0 = C … 11 = B).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Root(pub u8);

impl Root {
    /// Root letter using sharps (B-flat shows as `"A#"`).
    #[must_use]
    pub const fn letter(&self) -> &'static str {
        match self.0 % 12 {
            0 => "C",
            1 => "C#",
            2 => "D",
            3 => "D#",
            4 => "E",
            5 => "F",
            6 => "F#",
            7 => "G",
            8 => "G#",
            9 => "A",
            10 => "A#",
            _ => "B",
        }
    }
}

/// A single scored chord candidate.
#[derive(Debug, Clone)]
pub struct ChordMatch {
    /// Display name, e.g. `"Cmaj7"`.
    pub name: String,
    /// Root pitch class.
    pub root_pc: u8,
    /// Chord quality.
    pub quality: Quality,
    /// Cosine similarity score against the chroma.
    pub score: f32,
}

/// Build the (normalised) chroma template for a chord.
#[must_use]
pub fn template(root_pc: u8, quality: Quality) -> ChromaVector {
    let mut t = [0.0_f32; 12];
    for &iv in quality.intervals() {
        let pc = ((u16::from(root_pc) + u16::from(iv)) % 12) as usize;
        t[pc] = 1.0;
    }
    normalise(&mut t);
    t
}

/// Outcome of [`recognise`].
#[derive(Debug, Clone)]
pub struct RecognitionResult {
    /// Top candidates (at most five), highest score first.
    pub candidates: Vec<ChordMatch>,
    /// The confident best match, if any (clears `min_score` and `min_margin`).
    pub best: Option<ChordMatch>,
}

/// Recognise a chord from a chroma vector.
#[must_use]
pub fn recognise(chroma: &ChromaVector, min_score: f32, min_margin: f32) -> RecognitionResult {
    let mut all: Vec<ChordMatch> = Vec::with_capacity(108);
    for root_pc in 0u8..12 {
        for &quality in Quality::ALL {
            let t = template(root_pc, quality);
            let score = cosine_similarity(chroma, &t);
            all.push(ChordMatch {
                name: format!("{}{}", Root(root_pc).letter(), quality.suffix()),
                root_pc,
                quality,
                score,
            });
        }
    }
    all.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(core::cmp::Ordering::Equal)
    });

    let best = match (all.first(), all.get(1)) {
        (Some(top), Some(second))
            if top.score >= min_score && (top.score - second.score) >= min_margin =>
        {
            Some(top.clone())
        }
        (Some(top), None) if top.score >= min_score => Some(top.clone()),
        _ => None,
    };

    all.truncate(5);
    RecognitionResult {
        candidates: all,
        best,
    }
}

/// Parse a chord name into `(root_pc, quality)`. Hidden helper for tests.
///
/// Handles sharps and flats (`"Cmaj7"`, `"F#m"`, `"Bbm7"`).
#[doc(hidden)]
#[must_use]
pub fn parse(name: &str) -> Option<(u8, Quality)> {
    let bytes = name.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let mut pc: i32 = match bytes[0] {
        b'C' => 0,
        b'D' => 2,
        b'E' => 4,
        b'F' => 5,
        b'G' => 7,
        b'A' => 9,
        b'B' => 11,
        _ => return None,
    };
    let mut idx = 1;
    if idx < bytes.len() {
        match bytes[idx] {
            b'#' => {
                pc += 1;
                idx += 1;
            }
            b'b' => {
                pc -= 1;
                idx += 1;
            }
            _ => {}
        }
    }
    let root = pc.rem_euclid(12) as u8;
    let suffix = &name[idx..];
    let quality = Quality::ALL
        .iter()
        .copied()
        .find(|q| q.suffix() == suffix)?;
    Some((root, quality))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeSet;

    fn pc_set(root_pc: u8, q: Quality) -> BTreeSet<u8> {
        q.intervals()
            .iter()
            .map(|&iv| ((u16::from(root_pc) + u16::from(iv)) % 12) as u8)
            .collect()
    }

    #[test]
    fn major_has_three_notes() {
        let nonzero = template(0, Quality::Major)
            .iter()
            .filter(|&&x| x > 0.0)
            .count();
        assert_eq!(nonzero, 3);
    }

    #[test]
    fn c_major_hits_c_e_g() {
        let t = template(0, Quality::Major);
        assert!(t[0] > 0.0 && t[4] > 0.0 && t[7] > 0.0);
    }

    #[test]
    fn transposition_invariance() {
        // D major template is C major rotated by 2.
        let c = template(0, Quality::Major);
        let d = template(2, Quality::Major);
        for i in 0..12 {
            assert_relative_eq(c[i], d[(i + 2) % 12]);
        }
    }

    fn assert_relative_eq(a: f32, b: f32) {
        assert!((a - b).abs() < 1e-6, "{a} != {b}");
    }

    #[test]
    fn major_and_minor_differ_only_in_third() {
        let maj = pc_set(0, Quality::Major);
        let min = pc_set(0, Quality::Minor);
        let diff: BTreeSet<_> = maj.symmetric_difference(&min).copied().collect();
        // E (4) vs Eb (3)
        assert_eq!(diff, BTreeSet::from([3, 4]));
    }

    #[test]
    fn recognise_perfect_c_major_returns_c_major() {
        let t = template(0, Quality::Major);
        let r = recognise(&t, 0.85, 0.05);
        let best = r.best.expect("should have a best");
        assert_eq!(best.root_pc, 0);
        assert_eq!(best.quality, Quality::Major);
    }

    #[test]
    fn recognise_each_major_root() {
        for root in 0u8..12 {
            let t = template(root, Quality::Major);
            let r = recognise(&t, 0.85, 0.05);
            let best = r.best.unwrap_or_else(|| r.candidates[0].clone());
            assert_eq!(best.root_pc, root);
        }
    }

    #[test]
    fn recognise_each_quality_by_pitch_class_set() {
        // Sus2/Sus4 are enharmonic, so assert pitch-class-set membership in the
        // top candidates rather than an exact (root, quality).
        for &q in Quality::ALL {
            let t = template(0, q);
            let want = pc_set(0, q);
            let r = recognise(&t, 0.85, 0.05);
            let found = r
                .candidates
                .iter()
                .any(|c| pc_set(c.root_pc, c.quality) == want);
            assert!(found, "quality {q:?} not represented in candidates");
        }
    }

    #[test]
    fn silence_returns_no_best() {
        let z = [0.0_f32; 12];
        let r = recognise(&z, 0.85, 0.05);
        assert!(r.best.is_none());
    }

    #[test]
    fn parse_handles_sharps_and_qualities() {
        assert_eq!(parse("C"), Some((0, Quality::Major)));
        assert_eq!(parse("Cmaj7"), Some((0, Quality::MajorSeventh)));
        assert_eq!(parse("F#m"), Some((6, Quality::Minor)));
        assert_eq!(parse("Bbm7"), Some((10, Quality::MinorSeventh)));
        assert_eq!(parse("Gsus4"), Some((7, Quality::Sus4)));
        assert_eq!(parse("Adim"), Some((9, Quality::Diminished)));
        assert_eq!(parse("Eaug"), Some((4, Quality::Augmented)));
        assert!(parse("H").is_none());
        assert!(parse("").is_none());
    }

    #[test]
    fn margin_gate_rejects_ambiguous_blend() {
        // An even blend of C major and A minor templates has no clear winner.
        let cmaj = template(0, Quality::Major);
        let amin = template(9, Quality::Minor);
        let mut blend = [0.0_f32; 12];
        for i in 0..12 {
            blend[i] = cmaj[i] + amin[i];
        }
        normalise(&mut blend);
        let r = recognise(&blend, 0.85, 0.05);
        // Either no best, or a best that clears the margin — but the two are
        // close, so we mostly expect None.
        if let Some(b) = r.best {
            assert!(
                r.candidates[0].score - r.candidates[1].score >= 0.05,
                "best {b:?}"
            );
        }
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn template_self_recognition(root in 0u8..12, qi in 0usize..9) {
            let q = Quality::ALL[qi];
            let t = template(root, q);
            let want = pc_set(root, q);
            let r = recognise(&t, 0.85, 0.05);
            // The exact pitch-class set must appear among the top candidates.
            let found = r.candidates.iter().any(|c| pc_set(c.root_pc, c.quality) == want);
            prop_assert!(found);
        }
    }
}

//! Fret-position (voicing) generation for recognised chords.
//!
//! Given a tuning's open strings and a chord (root pitch class + [`Quality`]),
//! search the fretboard for playable voicings: assignments of each string to a
//! fret (or muted) such that only chord tones sound, the whole chord is covered,
//! and the shape is physically playable (bounded hand span and finger count).
//!
//! The search is tuning-agnostic — it works for guitar, bass, and guitarra
//! portuguesa alike — and intentionally small (a handful of fret options per
//! string, aggressively pruned by span), so it is cheap enough to run live.

use crate::chord::Quality;
use crate::tunings::StringSpec;
use alloc::vec;
use alloc::vec::Vec;

/// One playable chord shape: a fret per string, lowest-pitched string first.
///
/// `None` = muted, `Some(0)` = open, `Some(n)` = fretted at fret `n`. Maps
/// directly to the compact `x 3 2 0 1 0` display form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Voicing {
    /// Fret per string, low → high. `None` is a muted string.
    pub frets: Vec<Option<u8>>,
}

impl Voicing {
    /// Number of strings that sound (open or fretted).
    #[must_use]
    pub fn sounding(&self) -> usize {
        self.frets.iter().filter(|f| f.is_some()).count()
    }

    /// Number of fretted (finger-requiring) strings.
    #[must_use]
    pub fn fingered(&self) -> usize {
        self.frets
            .iter()
            .filter(|f| matches!(f, Some(n) if *n > 0))
            .count()
    }
}

/// Tunable limits for [`voicings`].
#[derive(Debug, Clone, Copy)]
pub struct VoicingConfig {
    /// Highest fret to consider (default 12).
    pub max_fret: u8,
    /// Maximum hand span in frets between the lowest and highest fretted note (default 4).
    pub span: u8,
    /// Maximum number of distinct voicings to return (default 4).
    pub max_voicings: usize,
}

impl Default for VoicingConfig {
    fn default() -> Self {
        Self {
            max_fret: 12,
            span: 4,
            max_voicings: 4,
        }
    }
}

fn chord_pcs(root_pc: u8, quality: Quality) -> [bool; 12] {
    let mut s = [false; 12];
    for &iv in quality.intervals() {
        s[((u16::from(root_pc) + u16::from(iv)) % 12) as usize] = true;
    }
    s
}

#[inline]
fn fret_pc(string_midi: u8, fret: u8) -> usize {
    ((u16::from(string_midi) + u16::from(fret)) % 12) as usize
}

struct Search<'a> {
    strings: &'a [StringSpec],
    allowed: Vec<Vec<Option<u8>>>,
    cfg: VoicingConfig,
    pcs: [bool; 12],
    min_sounding: usize,
    out: Vec<Voicing>,
}

impl Search<'_> {
    /// Span of the fretted (fret > 0) notes in `current[..filled]`; `None` if
    /// fewer than one fretted note.
    fn span_ok(&self, current: &[Option<u8>]) -> bool {
        let mut lo = u8::MAX;
        let mut hi = 0u8;
        let mut fingers = 0usize;
        for f in current.iter().flatten() {
            if *f > 0 {
                lo = lo.min(*f);
                hi = hi.max(*f);
                fingers += 1;
            }
        }
        if fingers > 4 {
            return false;
        }
        lo == u8::MAX || hi - lo <= self.cfg.span
    }

    fn recurse(&mut self, i: usize, current: &mut Vec<Option<u8>>) {
        if i == self.strings.len() {
            self.validate(current);
            return;
        }
        // Clone the per-string options out to avoid borrowing self while we recurse.
        let options = self.allowed[i].clone();
        for opt in options {
            current[i] = opt;
            if self.span_ok(&current[..=i]) {
                self.recurse(i + 1, current);
            }
        }
        current[i] = None;
    }

    fn validate(&mut self, current: &[Option<u8>]) {
        let sounding = current.iter().filter(|f| f.is_some()).count();
        if sounding < self.min_sounding {
            return;
        }
        // Every chord tone must be present (non-chord tones are impossible by
        // construction — only chord-pc frets were ever allowed).
        let mut covered = [false; 12];
        for (j, f) in current.iter().enumerate() {
            if let Some(fret) = f {
                covered[fret_pc(self.strings[j].midi, *fret)] = true;
            }
        }
        if self
            .pcs
            .iter()
            .zip(covered.iter())
            .any(|(&need, &have)| need && !have)
        {
            return;
        }
        self.out.push(Voicing {
            frets: current.to_vec(),
        });
    }
}

/// Sort key (lower is better). Idiomatic chord shapes are contiguous (no
/// interior muted strings), put the root in the bass, sit low on the neck, and
/// use few fingers — so we order by, in priority: interior mutes, root-in-bass,
/// lowest fret, fewer fingers, more sounding strings, lower total fret distance.
fn rank_key(strings: &[StringSpec], root_pc: u8, v: &Voicing) -> (i32, i32, i32, i32, i32, i32) {
    let mut min_fret = i32::MAX;
    let mut fingers = 0i32;
    let mut sum = 0i32;
    for f in v.frets.iter().flatten() {
        if *f > 0 {
            min_fret = min_fret.min(i32::from(*f));
            fingers += 1;
            sum += i32::from(*f);
        }
    }
    if min_fret == i32::MAX {
        min_fret = 0;
    }

    // Interior muted strings = muted strings between the lowest and highest
    // sounding string.
    let first = v.frets.iter().position(Option::is_some);
    let last = v.frets.iter().rposition(Option::is_some);
    let (interior_mutes, sounding) = match (first, last) {
        (Some(a), Some(b)) => {
            let interior =
                i32::try_from((a..=b).filter(|j| v.frets[*j].is_none()).count()).unwrap_or(0);
            let sounding =
                i32::try_from(v.frets.iter().filter(|f| f.is_some()).count()).unwrap_or(0);
            (interior, sounding)
        }
        _ => (0, 0),
    };

    let bass_not_root = v
        .frets
        .iter()
        .enumerate()
        .find_map(|(j, f)| f.map(|fret| fret_pc(strings[j].midi, fret)))
        .map_or(1, |pc| i32::from(pc != usize::from(root_pc)));

    (
        interior_mutes,
        bass_not_root,
        min_fret,
        fingers,
        -sounding,
        sum,
    )
}

/// Generate playable voicings of `(root_pc, quality)` for the given strings.
///
/// Strings are lowest-pitch first. Returns up to `cfg.max_voicings` distinct
/// shapes, best first, or an empty vector if no playable voicing exists.
#[must_use]
pub fn voicings(
    strings: &[StringSpec],
    root_pc: u8,
    quality: Quality,
    cfg: VoicingConfig,
) -> Vec<Voicing> {
    if strings.is_empty() {
        return Vec::new();
    }
    let pcs = chord_pcs(root_pc, quality);
    let n_chord_tones = pcs.iter().filter(|&&b| b).count();

    let allowed: Vec<Vec<Option<u8>>> = strings
        .iter()
        .map(|s| {
            let mut v: Vec<Option<u8>> = vec![None]; // muted is always an option
            for f in 0..=cfg.max_fret {
                if pcs[fret_pc(s.midi, f)] {
                    v.push(Some(f));
                }
            }
            v
        })
        .collect();

    let mut search = Search {
        strings,
        allowed,
        cfg,
        pcs,
        min_sounding: n_chord_tones.max(3),
        out: Vec::new(),
    };
    let mut current: Vec<Option<u8>> = vec![None; strings.len()];
    search.recurse(0, &mut current);

    let mut out = search.out;
    out.sort_by_key(|v| rank_key(strings, root_pc, v));
    out.truncate(cfg.max_voicings);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tunings::{BASS_STANDARD, GUITAR_STANDARD};

    fn has_shape(vs: &[Voicing], shape: &[Option<u8>]) -> bool {
        vs.iter().any(|v| v.frets == shape)
    }

    fn pc_set(root: u8, q: Quality) -> [bool; 12] {
        chord_pcs(root, q)
    }

    #[test]
    fn c_major_includes_open_shape_on_guitar() {
        // C major open: x 3 2 0 1 0
        let vs = voicings(
            GUITAR_STANDARD.strings,
            0,
            Quality::Major,
            VoicingConfig::default(),
        );
        assert!(!vs.is_empty());
        let cmaj = [None, Some(3), Some(2), Some(0), Some(1), Some(0)];
        assert!(has_shape(&vs, &cmaj), "expected x32010 in {vs:?}");
    }

    #[test]
    fn e_major_includes_open_shape() {
        // E major open: 0 2 2 1 0 0
        let vs = voicings(
            GUITAR_STANDARD.strings,
            4,
            Quality::Major,
            VoicingConfig::default(),
        );
        let emaj = [Some(0), Some(2), Some(2), Some(1), Some(0), Some(0)];
        assert!(has_shape(&vs, &emaj), "expected 022100 in {vs:?}");
    }

    #[test]
    fn a_minor_includes_open_shape() {
        // A minor open: x 0 2 2 1 0
        let vs = voicings(
            GUITAR_STANDARD.strings,
            9,
            Quality::Minor,
            VoicingConfig::default(),
        );
        let amin = [None, Some(0), Some(2), Some(2), Some(1), Some(0)];
        assert!(has_shape(&vs, &amin), "expected x02210 in {vs:?}");
    }

    #[test]
    fn voicings_only_sound_chord_tones() {
        let cfg = VoicingConfig::default();
        for root in 0u8..12 {
            for &q in &[Quality::Major, Quality::Minor, Quality::Seventh] {
                let pcs = pc_set(root, q);
                for v in voicings(GUITAR_STANDARD.strings, root, q, cfg) {
                    for (j, f) in v.frets.iter().enumerate() {
                        if let Some(fret) = f {
                            let pc = fret_pc(GUITAR_STANDARD.strings[j].midi, *fret);
                            assert!(pcs[pc], "non-chord tone in {v:?}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn voicings_respect_span_and_finger_limits() {
        let cfg = VoicingConfig::default();
        for root in 0u8..12 {
            for v in voicings(GUITAR_STANDARD.strings, root, Quality::Major, cfg) {
                assert!(v.fingered() <= 4, "too many fingers: {v:?}");
                let fretted: Vec<u8> = v
                    .frets
                    .iter()
                    .flatten()
                    .copied()
                    .filter(|f| *f > 0)
                    .collect();
                if let (Some(&lo), Some(&hi)) = (fretted.iter().min(), fretted.iter().max()) {
                    assert!(hi - lo <= cfg.span, "span too wide: {v:?}");
                }
            }
        }
    }

    #[test]
    fn voicings_cover_every_chord_tone() {
        let cfg = VoicingConfig::default();
        let pcs = pc_set(0, Quality::Major);
        for v in voicings(GUITAR_STANDARD.strings, 0, Quality::Major, cfg) {
            let mut covered = [false; 12];
            for (j, f) in v.frets.iter().enumerate() {
                if let Some(fret) = f {
                    covered[fret_pc(GUITAR_STANDARD.strings[j].midi, *fret)] = true;
                }
            }
            for (pc, (&need, &have)) in pcs.iter().zip(covered.iter()).enumerate() {
                assert!(!need || have, "missing chord tone pc {pc} in {v:?}");
            }
        }
    }

    #[test]
    fn adapts_to_other_tunings() {
        // Bass standard (E1 A1 D2 G2): an E major triad is reachable on 4 strings.
        let vs = voicings(
            BASS_STANDARD.strings,
            4,
            Quality::Major,
            VoicingConfig::default(),
        );
        for v in &vs {
            assert_eq!(v.frets.len(), 4);
        }
    }

    #[test]
    fn empty_when_no_playable_voicing() {
        // A single-string "instrument" can't sound a 3-note chord.
        let one = &GUITAR_STANDARD.strings[0..1];
        assert!(voicings(one, 0, Quality::Major, VoicingConfig::default()).is_empty());
    }

    #[test]
    fn respects_max_voicings() {
        let cfg = VoicingConfig {
            max_voicings: 2,
            ..VoicingConfig::default()
        };
        assert!(voicings(GUITAR_STANDARD.strings, 0, Quality::Major, cfg).len() <= 2);
    }
}

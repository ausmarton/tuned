//! Hand-rolled JSON serialisation shared by the JNI and WASM surfaces.
//!
//! Results cross both FFI boundaries as JSON strings — the simplest portable
//! shape for a Kotlin / JavaScript caller to parse. Kept dependency-free.

use crate::cents::Direction;
use crate::chord::{ChordMatch, RecognitionResult};
use crate::fretboard::{voicings, Voicing, VoicingConfig};
use crate::strum::StrumReport;
use crate::tunings::Tuning;
use crate::TunerSnapshot;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

fn opt_f32(v: Option<f32>) -> String {
    v.map_or_else(|| "null".into(), |x| format!("{x}"))
}

const fn dir_str(d: Option<Direction>) -> &'static str {
    match d {
        Some(Direction::Flat) => "\"flat\"",
        Some(Direction::InTune) => "\"in_tune\"",
        Some(Direction::Sharp) => "\"sharp\"",
        None => "null",
    }
}

fn opt_str(v: Option<&str>) -> String {
    v.map_or_else(|| "null".into(), |s| format!("\"{s}\""))
}

/// Serialise a per-frame snapshot. (Used by the WASM surface; the JNI surface
/// builds a `Snapshot` object directly.)
#[cfg_attr(not(feature = "wasm"), allow(dead_code))]
#[must_use]
pub(super) fn snapshot_json(s: &TunerSnapshot) -> String {
    format!(
        "{{\"pitchHz\":{},\"centsOff\":{},\"direction\":{},\"nearestString\":{},\"nearestStringName\":{},\"confidence\":{}}}",
        opt_f32(s.pitch_hz),
        opt_f32(s.cents_off),
        dir_str(s.direction),
        s.nearest_string
            .map_or_else(|| "null".into(), |i| alloc::string::ToString::to_string(&i)),
        opt_str(s.nearest_string_name),
        s.confidence,
    )
}

/// Serialise a strum report.
#[must_use]
pub(super) fn strum_json(report: &StrumReport) -> String {
    let parts: Vec<String> = report
        .strings
        .iter()
        .map(|s| {
            format!(
                "{{\"index\":{},\"name\":\"{}\",\"targetHz\":{},\"detectedHz\":{},\"centsOff\":{},\"direction\":{},\"confidence\":{}}}",
                s.string_index,
                s.name,
                s.target_hz,
                opt_f32(s.detected_hz),
                opt_f32(s.cents_off),
                dir_str(s.direction),
                s.confidence,
            )
        })
        .collect();
    format!("{{\"strings\":[{}]}}", parts.join(","))
}

fn voicing_json(v: &Voicing) -> String {
    let parts: Vec<String> = v
        .frets
        .iter()
        .map(|f| f.map_or_else(|| "null".into(), |x| alloc::string::ToString::to_string(&x)))
        .collect();
    format!("[{}]", parts.join(","))
}

/// Serialise one chord candidate, attaching fret voicings for `tuning` when
/// `with_voicings` is set (only the top candidates carry voicings, to bound work).
fn candidate_json(c: &ChordMatch, tuning: &Tuning, with_voicings: bool) -> String {
    let voicings_json = if with_voicings {
        let vs = voicings(
            tuning.strings,
            c.root_pc,
            c.quality,
            VoicingConfig::default(),
        );
        vs.iter().map(voicing_json).collect::<Vec<_>>().join(",")
    } else {
        String::new()
    };
    format!(
        "{{\"name\":\"{}\",\"score\":{},\"rootPc\":{},\"quality\":\"{}\",\"voicings\":[{}]}}",
        c.name,
        c.score,
        c.root_pc,
        c.quality.suffix(),
        voicings_json,
    )
}

/// Serialise a chord recognition result, including fret voicings (for the active
/// `tuning`) on the top few candidates and the best match.
#[must_use]
pub(super) fn chord_json(result: &RecognitionResult, tuning: &Tuning) -> String {
    // Compute voicings only for the leading candidates to keep live recognition cheap.
    const WITH_VOICINGS: usize = 3;
    let cands: Vec<String> = result
        .candidates
        .iter()
        .enumerate()
        .map(|(i, c)| candidate_json(c, tuning, i < WITH_VOICINGS))
        .collect();
    let best = result
        .best
        .as_ref()
        .map_or_else(|| "null".into(), |b| candidate_json(b, tuning, true));
    // String labels of the active tuning, so callers can label voicing columns
    // without duplicating the tuning table.
    let strings: Vec<String> = tuning
        .strings
        .iter()
        .map(|s| format!("\"{}\"", s.name))
        .collect();
    format!(
        "{{\"candidates\":[{}],\"best\":{},\"strings\":[{}]}}",
        cands.join(","),
        best,
        strings.join(",")
    )
}

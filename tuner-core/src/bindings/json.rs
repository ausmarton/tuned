//! Hand-rolled JSON serialisation shared by the JNI and WASM surfaces.
//!
//! Results cross both FFI boundaries as JSON strings — the simplest portable
//! shape for a Kotlin / JavaScript caller to parse. Kept dependency-free.

use crate::cents::Direction;
use crate::chord::RecognitionResult;
use crate::strum::StrumReport;
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

/// Serialise a chord recognition result.
#[must_use]
pub(super) fn chord_json(result: &RecognitionResult) -> String {
    let cands: Vec<String> = result
        .candidates
        .iter()
        .map(|c| format!("{{\"name\":\"{}\",\"score\":{}}}", c.name, c.score))
        .collect();
    let best = result.best.as_ref().map_or_else(
        || "null".into(),
        |b| format!("{{\"name\":\"{}\",\"score\":{}}}", b.name, b.score),
    );
    format!("{{\"candidates\":[{}],\"best\":{}}}", cands.join(","), best)
}

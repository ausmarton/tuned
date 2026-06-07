//! Browser (wasm-bindgen) surface.
//!
//! Results cross the ABI as JSON strings — the simplest portable shape for the
//! web worker to `JSON.parse`. The `wasm` feature implies `std`.

use crate::cents::Direction;
use crate::{Tuner, TunerConfig, TunerSnapshot};
use wasm_bindgen::prelude::*;

/// A tuner instance usable from JavaScript.
#[wasm_bindgen]
pub struct WasmTuner {
    inner: Tuner,
}

fn opt_f32(v: Option<f32>) -> String {
    v.map_or_else(|| "null".into(), |x| format!("{x}"))
}

fn dir_str(d: Option<Direction>) -> String {
    match d {
        Some(Direction::Flat) => "\"flat\"".into(),
        Some(Direction::InTune) => "\"in_tune\"".into(),
        Some(Direction::Sharp) => "\"sharp\"".into(),
        None => "null".into(),
    }
}

fn opt_str(v: Option<&str>) -> String {
    v.map_or_else(|| "null".into(), |s| format!("\"{s}\""))
}

fn snapshot_json(s: &TunerSnapshot) -> String {
    format!(
        "{{\"pitchHz\":{},\"centsOff\":{},\"direction\":{},\"nearestString\":{},\"nearestStringName\":{},\"confidence\":{}}}",
        opt_f32(s.pitch_hz),
        opt_f32(s.cents_off),
        dir_str(s.direction),
        s.nearest_string.map_or_else(|| "null".into(), |i| i.to_string()),
        opt_str(s.nearest_string_name),
        s.confidence,
    )
}

#[wasm_bindgen]
impl WasmTuner {
    /// Construct a tuner. Throws (as a JS error string) on invalid config.
    #[wasm_bindgen(constructor)]
    pub fn new(tuning_id: &str, sample_rate: u32, a4: f32) -> Result<Self, JsValue> {
        // Resolve to a 'static id from the shipped table.
        let id_static: &'static str =
            crate::tunings::lookup(tuning_id).map_or("guitar.standard", |t| t.id);
        let cfg = TunerConfig {
            sample_rate_hz: if sample_rate > 0 { sample_rate } else { 48_000 },
            a4_hz: a4,
            active_tuning_id: id_static,
            ..TunerConfig::default()
        };
        Tuner::new(cfg)
            .map(|inner| Self { inner })
            .map_err(|e| JsValue::from_str(&format!("{e}")))
    }

    /// Push a chunk of mono float samples.
    #[wasm_bindgen(js_name = pushSamples)]
    pub fn push_samples(&mut self, samples: &[f32]) {
        self.inner.push_samples(samples);
    }

    /// Switch the active tuning; returns `false` if the id is unknown.
    #[wasm_bindgen(js_name = setTuning)]
    pub fn set_tuning(&mut self, id: &str) -> bool {
        self.inner.set_tuning(id).is_ok()
    }

    /// Latest per-frame snapshot as JSON.
    #[wasm_bindgen(js_name = snapshotJson)]
    #[must_use]
    pub fn snapshot_json(&self) -> String {
        snapshot_json(&self.inner.snapshot())
    }

    /// Strum report as JSON.
    #[wasm_bindgen(js_name = analyseStrumJson)]
    #[must_use]
    pub fn analyse_strum_json(&self) -> String {
        let report = self.inner.analyse_strum();
        let mut parts = Vec::with_capacity(report.strings.len());
        for s in &report.strings {
            parts.push(format!(
                "{{\"index\":{},\"name\":\"{}\",\"targetHz\":{},\"detectedHz\":{},\"centsOff\":{},\"direction\":{},\"confidence\":{}}}",
                s.string_index,
                s.name,
                s.target_hz,
                opt_f32(s.detected_hz),
                opt_f32(s.cents_off),
                dir_str(s.direction),
                s.confidence,
            ));
        }
        format!("{{\"strings\":[{}]}}", parts.join(","))
    }

    /// Chord recognition as JSON.
    #[wasm_bindgen(js_name = recogniseChordJson)]
    #[must_use]
    pub fn recognise_chord_json(&self) -> String {
        let result = self.inner.recognise_chord();
        let cands: Vec<String> = result
            .candidates
            .iter()
            .map(|c| format!("{{\"name\":\"{}\",\"score\":{}}}", c.name, c.score))
            .collect();
        let best = result.best.map_or_else(
            || "null".into(),
            |b| format!("{{\"name\":\"{}\",\"score\":{}}}", b.name, b.score),
        );
        format!("{{\"candidates\":[{}],\"best\":{}}}", cands.join(","), best)
    }
}

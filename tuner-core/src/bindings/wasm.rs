//! Browser (wasm-bindgen) surface.
//!
//! Results cross the ABI as JSON strings (see [`super::json`]). The `wasm`
//! feature implies `std`.

use super::json;
use crate::{Tuner, TunerConfig};
use wasm_bindgen::prelude::*;

/// A tuner instance usable from JavaScript.
#[wasm_bindgen]
pub struct WasmTuner {
    inner: Tuner,
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
        json::snapshot_json(&self.inner.snapshot())
    }

    /// Strum report as JSON.
    #[wasm_bindgen(js_name = analyseStrumJson)]
    #[must_use]
    pub fn analyse_strum_json(&self) -> String {
        json::strum_json(&self.inner.analyse_strum())
    }

    /// Chord recognition as JSON.
    #[wasm_bindgen(js_name = recogniseChordJson)]
    #[must_use]
    pub fn recognise_chord_json(&self) -> String {
        json::chord_json(&self.inner.recognise_chord())
    }
}

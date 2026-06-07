//! Foreign-function bindings. Each submodule is gated behind its feature so the
//! default `rlib` build pulls in neither `jni` nor `wasm-bindgen`.

#[cfg(feature = "jni")]
pub mod jni;

#[cfg(feature = "wasm")]
pub mod wasm;

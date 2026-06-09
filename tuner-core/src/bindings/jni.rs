//! Android JNI surface.
//!
//! Mirrors `com.opentuner.NativeTuner`. The native handle is a leaked
//! `Box<Tuner>` cast to `jlong`; `nativeFree` reclaims it. The `Snapshot`
//! constructor signature `(FFIILjava/lang/String;F)V` must stay in lockstep
//! with `Snapshot.kt`.

use super::json;
use crate::tunings;
use crate::{Tuner, TunerConfig};
use jni::objects::{JClass, JFloatArray, JString, JValue};
use jni::sys::{jdouble, jfloat, jint, jlong, jobject, jstring};
use jni::JNIEnv;

/// Reconstruct a `&mut Tuner` from a raw handle. Caller guarantees validity.
unsafe fn tuner_from_handle<'a>(handle: jlong) -> Option<&'a mut Tuner> {
    if handle == 0 {
        None
    } else {
        Some(&mut *(handle as *mut Tuner))
    }
}

/// Construct a native `Tuner` and return an opaque handle (0 on failure).
#[no_mangle]
pub extern "system" fn Java_com_opentuner_NativeTuner_nativeNew<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    tuning_id: JString<'local>,
    sample_rate: jint,
    a4: jdouble,
) -> jlong {
    let requested: String = match env.get_string(&tuning_id) {
        Ok(s) => s.into(),
        Err(_) => return 0,
    };
    // Resolve to a 'static id from the shipped table (no leaking required).
    let id_static: &'static str = tunings::lookup(&requested).map_or("guitar.standard", |t| t.id);

    let cfg = TunerConfig {
        sample_rate_hz: if sample_rate > 0 {
            sample_rate as u32
        } else {
            48_000
        },
        a4_hz: a4 as f32,
        active_tuning_id: id_static,
        ..TunerConfig::default()
    };

    match Tuner::new(cfg) {
        Ok(tuner) => Box::into_raw(Box::new(tuner)) as jlong,
        Err(_) => 0,
    }
}

/// Free a handle previously returned by `nativeNew`.
#[no_mangle]
pub extern "system" fn Java_com_opentuner_NativeTuner_nativeFree<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    handle: jlong,
) {
    if handle != 0 {
        // SAFETY: handle came from Box::into_raw in nativeNew and is freed once.
        unsafe {
            drop(Box::from_raw(handle as *mut Tuner));
        }
    }
}

/// Push a Java `float[]` of samples into the tuner.
#[no_mangle]
pub extern "system" fn Java_com_opentuner_NativeTuner_nativePushSamples<'local>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
    handle: jlong,
    samples: JFloatArray<'local>,
) {
    // SAFETY: handle is a live Tuner for the duration of this call.
    let Some(tuner) = (unsafe { tuner_from_handle(handle) }) else {
        return;
    };
    let Ok(len) = env.get_array_length(&samples) else {
        return;
    };
    if len <= 0 {
        return;
    }
    let len = len as usize;
    let mut buf = vec![0.0_f32; len];
    if env.get_float_array_region(&samples, 0, &mut buf).is_ok() {
        tuner.push_samples(&buf);
    }
}

/// Switch the active tuning; returns 1 on success, 0 on failure.
#[no_mangle]
pub extern "system" fn Java_com_opentuner_NativeTuner_nativeSetTuning<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    handle: jlong,
    tuning_id: JString<'local>,
) -> jint {
    let Some(tuner) = (unsafe { tuner_from_handle(handle) }) else {
        return 0;
    };
    let id: String = match env.get_string(&tuning_id) {
        Ok(s) => s.into(),
        Err(_) => return 0,
    };
    jint::from(tuner.set_tuning(&id).is_ok())
}

/// Build a `com.opentuner.Snapshot` from the latest analysis (null on failure).
#[no_mangle]
pub extern "system" fn Java_com_opentuner_NativeTuner_nativeSnapshot<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> jobject {
    let null = core::ptr::null_mut();
    let Some(tuner) = (unsafe { tuner_from_handle(handle) }) else {
        return null;
    };
    let snap = tuner.snapshot();

    let pitch = snap.pitch_hz.unwrap_or(0.0) as jfloat;
    let cents = snap.cents_off.unwrap_or(0.0) as jfloat;
    let detected: jint = jint::from(snap.pitch_hz.is_some());
    let idx: jint = snap
        .nearest_string
        .map_or(-1, |i| jint::try_from(i).unwrap_or(-1));
    let conf = snap.confidence as jfloat;
    let name = snap.nearest_string_name.unwrap_or("");

    let Ok(name_obj) = env.new_string(name) else {
        return null;
    };

    match env.new_object(
        "com/opentuner/Snapshot",
        "(FFIILjava/lang/String;F)V",
        &[
            JValue::Float(pitch),
            JValue::Float(cents),
            JValue::Int(detected),
            JValue::Int(idx),
            JValue::Object(&name_obj),
            JValue::Float(conf),
        ],
    ) {
        Ok(obj) => obj.into_raw(),
        Err(_) => null,
    }
}

/// Build a Java string from `s`, or return a null `jstring` on failure.
fn new_jstring(env: &JNIEnv<'_>, s: &str) -> jstring {
    match env.new_string(s) {
        Ok(js) => js.into_raw(),
        Err(_) => core::ptr::null_mut(),
    }
}

/// Analyse the buffered audio as a strum; returns a JSON string (see
/// [`super::json::strum_json`]).
#[no_mangle]
pub extern "system" fn Java_com_opentuner_NativeTuner_nativeAnalyseStrumJson<'local>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> jstring {
    let Some(tuner) = (unsafe { tuner_from_handle(handle) }) else {
        return new_jstring(&env, "{\"strings\":[]}");
    };
    let payload = json::strum_json(&tuner.analyse_strum());
    new_jstring(&env, &payload)
}

/// Recognise a chord from the buffered audio; returns a JSON string (see
/// [`super::json::chord_json`]).
#[no_mangle]
pub extern "system" fn Java_com_opentuner_NativeTuner_nativeRecogniseChordJson<'local>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
    handle: jlong,
) -> jstring {
    let Some(tuner) = (unsafe { tuner_from_handle(handle) }) else {
        return new_jstring(&env, "{\"candidates\":[],\"best\":null}");
    };
    let payload = json::chord_json(&tuner.recognise_chord());
    new_jstring(&env, &payload)
}

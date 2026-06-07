package com.opentuner

/**
 * Result of one analysis frame, mirroring `tuner_core::TunerSnapshot`.
 *
 * The constructor signature `(FFIILjava/lang/String;F)V` is referenced directly
 * from `tuner-core/src/bindings/jni.rs`. DO NOT reorder fields without updating
 * the JNI side to match.
 */
data class Snapshot(
    val pitchHz: Float,
    val centsOff: Float,
    val detected: Int,
    val nearestStringIndex: Int,
    val nearestStringName: String,
    val confidence: Float,
) {
    val hasPitch: Boolean get() = detected != 0 && pitchHz > 0f
}

package com.opentuner

import java.util.concurrent.atomic.AtomicLong

/**
 * Thin, thread-safe wrapper around the native `tuner-core` handle.
 *
 * The handle is an opaque `jlong` returned by [nativeNew]. [close] swaps it to 0
 * atomically and frees it exactly once, so concurrent calls after close are safe
 * no-ops.
 */
class NativeTuner(
    tuningId: String = "guitar.standard",
    sampleRate: Int = 48_000,
    a4: Double = 440.0,
) : AutoCloseable {
    private val handle = AtomicLong(nativeNew(tuningId, sampleRate, a4))

    init {
        require(handle.get() != 0L) { "failed to create native tuner" }
    }

    fun pushSamples(samples: FloatArray) {
        val h = handle.get()
        if (h != 0L) nativePushSamples(h, samples)
    }

    fun setTuning(tuningId: String): Boolean {
        val h = handle.get()
        return h != 0L && nativeSetTuning(h, tuningId) == 1
    }

    fun snapshot(): Snapshot? {
        val h = handle.get()
        return if (h != 0L) nativeSnapshot(h) else null
    }

    override fun close() {
        val h = handle.getAndSet(0L)
        if (h != 0L) nativeFree(h)
    }

    private external fun nativeNew(
        tuningId: String,
        sampleRate: Int,
        a4: Double,
    ): Long

    private external fun nativeFree(handle: Long)

    private external fun nativePushSamples(
        handle: Long,
        samples: FloatArray,
    )

    private external fun nativeSetTuning(
        handle: Long,
        tuningId: String,
    ): Int

    private external fun nativeSnapshot(handle: Long): Snapshot?

    companion object {
        init {
            System.loadLibrary("tuner_core")
        }
    }
}

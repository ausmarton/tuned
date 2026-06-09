package com.opentuner

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.sin

/**
 * Exercises the JNI surface on a device/emulator: the native library loads, a
 * synthesised sine is detected, and tuning switching works.
 */
@RunWith(AndroidJUnit4::class)
class NativeTunerInstrumentedTest {
    private fun sine(
        freq: Double,
        sampleRate: Int,
        n: Int,
    ): FloatArray = FloatArray(n) { i -> (0.5 * sin(2.0 * PI * freq * i / sampleRate)).toFloat() }

    @Test
    fun nativeLibraryLoadsAndBuildsTuner() {
        NativeTuner().use { tuner ->
            assertNotNull(tuner.snapshot())
        }
    }

    @Test
    fun pushingASineYieldsAReasonablePitch() {
        NativeTuner().use { tuner ->
            // G3 = 196 Hz, string index 3 of guitar.standard.
            tuner.pushSamples(sine(196.0, 48_000, 16384))
            val snap = tuner.snapshot()
            assertNotNull(snap)
            assertTrue(snap!!.hasPitch)
            assertEquals("G3", snap.nearestStringName)
            assertTrue("cents off ${snap.centsOff}", abs(snap.centsOff) < 3f)
        }
    }

    @Test
    fun setTuningSwitchesActiveTuning() {
        NativeTuner().use { tuner ->
            assertTrue(tuner.setTuning("bass.standard"))
            assertTrue(!tuner.setTuning("does.not.exist"))
        }
    }

    private fun mix(
        freqs: DoubleArray,
        sampleRate: Int,
        n: Int,
    ): FloatArray {
        val out = FloatArray(n)
        for (f in freqs) {
            val s = sine(f, sampleRate, n)
            for (i in 0 until n) out[i] += s[i]
        }
        return out
    }

    @Test
    fun strumAnalysisFindsAllSixGuitarStrings() {
        NativeTuner().use { tuner ->
            val guitar = doubleArrayOf(82.4069, 110.0, 146.8324, 196.0, 246.9417, 329.6276)
            tuner.pushSamples(mix(guitar, 48_000, 48_000))
            val report = tuner.analyseStrum()
            assertNotNull(report)
            val inTune = report!!.strings.count { it.direction == "in_tune" }
            assertTrue("only $inTune in tune", inTune >= 5)
        }
    }

    @Test
    fun chordRecognitionReturnsCandidates() {
        NativeTuner().use { tuner ->
            // C major (C4/E4/G4) with harmonics.
            val cmaj = doubleArrayOf(261.6256, 329.6276, 392.0)
            val n = 16384
            val buf = FloatArray(n)
            for (base in cmaj) {
                for (h in 1..8) {
                    val f = base * h
                    if (f > 24_000) break
                    val s = FloatArray(n) { i -> (0.5 / h * sin(2.0 * PI * f * i / 48_000)).toFloat() }
                    for (i in 0 until n) buf[i] += s[i]
                }
            }
            tuner.pushSamples(buf)
            val result = tuner.recogniseChord()
            assertNotNull(result)
            assertTrue(result!!.candidates.isNotEmpty())
        }
    }
}

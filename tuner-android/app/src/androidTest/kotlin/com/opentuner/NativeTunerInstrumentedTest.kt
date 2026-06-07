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
    private fun sine(freq: Double, sampleRate: Int, n: Int): FloatArray =
        FloatArray(n) { i -> (0.5 * sin(2.0 * PI * freq * i / sampleRate)).toFloat() }

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
}

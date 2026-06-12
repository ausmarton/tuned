package com.opentuner

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test

class SmoothingTest {
    private fun string(
        cents: Float?,
        dir: String?,
        conf: Float,
    ) = StrumReport(listOf(TuningString(0, "E2", 82f, cents?.let { 82f }, cents, dir, conf)))

    @Test
    fun strumHoldsLastConfidentReadingThenDrops() {
        val sm = StrumSmoother(holdMs = 1000, minConfidence = 0.2f)
        assertEquals(3f, sm.update(string(3f, "sharp", 0.9f), 0)[0].centsOff)
        // Silent frame within the hold window keeps the last reading.
        assertEquals(3f, sm.update(string(null, null, 0f), 500)[0].centsOff)
        // Beyond the hold window the reading drops.
        assertNull(sm.update(string(null, null, 0f), 2000)[0].centsOff)
    }

    @Test
    fun strumIgnoresLowConfidence() {
        val sm = StrumSmoother(holdMs = 1000, minConfidence = 0.5f)
        assertNull(sm.update(string(3f, "sharp", 0.1f), 0)[0].centsOff)
    }

    private fun chord(name: String?) =
        ChordResult(
            candidates = name?.let { listOf(ChordCandidate(it, 0.9f, 0, "", emptyList())) } ?: emptyList(),
            best = name?.let { ChordCandidate(it, 0.9f, 0, "", emptyList()) },
            strings = emptyList(),
        )

    @Test
    fun chordDebouncesBeforeShowing() {
        val sm = ChordSmoother(debounceMs = 250, holdMs = 1000)
        assertNull(sm.update(chord("C"), 0)) // pending starts
        assertNull(sm.update(chord("C"), 100)) // not stable yet
        assertNotNull(sm.update(chord("C"), 300)) // stable → shown
    }

    @Test
    fun chordHoldsAcrossBriefGapThenExpires() {
        val sm = ChordSmoother(debounceMs = 0, holdMs = 1000)
        assertNull(sm.update(chord("C"), 0)) // pending
        assertNotNull(sm.update(chord("C"), 10)) // confirmed → shown
        assertNotNull(sm.update(chord(null), 500)) // held across gap
        assertNull(sm.update(chord(null), 1100)) // expired
    }
}

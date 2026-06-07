package com.opentuner

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SnapshotTest {
    @Test
    fun hasPitchTrueWhenDetectedAndPositive() {
        val s = Snapshot(196f, 2f, 1, 3, "G3", 0.9f)
        assertTrue(s.hasPitch)
    }

    @Test
    fun hasPitchFalseWhenNotDetected() {
        val s = Snapshot(196f, 2f, 0, 3, "G3", 0.9f)
        assertFalse(s.hasPitch)
    }

    @Test
    fun hasPitchFalseWhenPitchNonPositive() {
        val s = Snapshot(0f, 0f, 1, -1, "", 0f)
        assertFalse(s.hasPitch)
    }

    @Test
    fun supportedTuningsContainsAllFour() {
        assertEquals(4, SUPPORTED_TUNINGS.size)
        val ids = SUPPORTED_TUNINGS.map { it.first }
        assertTrue(ids.contains("guitar.standard"))
        assertTrue(ids.contains("bass.standard"))
        assertTrue(ids.contains("guitarra.lisboa"))
        assertTrue(ids.contains("guitarra.coimbra"))
    }
}

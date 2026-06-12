package com.opentuner

/**
 * A per-string strum reading after temporal smoothing. [ageMs] is how long ago
 * the (held) reading was last confidently observed; the UI fades the indicator
 * toward neutral as it ages.
 */
data class SmoothedString(
    val name: String,
    val centsOff: Float?,
    val direction: String?,
    val ageMs: Long,
    val confidence: Float,
)

/**
 * Holds each string's last confident reading for up to [holdMs] so the live
 * strum display stays legible between pluck transients instead of flickering to
 * "—". A reading counts only when its confidence clears [minConfidence].
 */
class StrumSmoother(
    private val holdMs: Long = 2000,
    private val minConfidence: Float = 0.2f,
) {
    private data class Last(
        val cents: Float,
        val direction: String,
        val confidence: Float,
        val atMs: Long,
    )

    private val last = HashMap<Int, Last>()

    fun update(
        report: StrumReport,
        nowMs: Long,
    ): List<SmoothedString> =
        report.strings.map { s ->
            if (s.centsOff != null && s.direction != null && s.confidence >= minConfidence) {
                last[s.index] = Last(s.centsOff, s.direction, s.confidence, nowMs)
            }
            val l = last[s.index]
            if (l != null && nowMs - l.atMs <= holdMs) {
                SmoothedString(s.name, l.cents, l.direction, nowMs - l.atMs, l.confidence)
            } else {
                SmoothedString(s.name, null, null, Long.MAX_VALUE, 0f)
            }
        }

    fun reset() = last.clear()
}

/**
 * Debounces the live chord display: a new best chord is only shown once it has
 * persisted for [debounceMs], and the last shown chord is held for up to
 * [holdMs] across brief silences, so the readout doesn't flicker.
 */
class ChordSmoother(
    private val debounceMs: Long = 250,
    private val holdMs: Long = 1500,
) {
    private var pendingName: String? = null
    private var pendingSince: Long = 0
    private var displayed: ChordResult? = null
    private var displayedAt: Long = 0

    fun update(
        result: ChordResult,
        nowMs: Long,
    ): ChordResult? {
        val best = result.best
        if (best != null) {
            if (best.name == pendingName) {
                if (nowMs - pendingSince >= debounceMs) {
                    displayed = result
                    displayedAt = nowMs
                }
            } else {
                pendingName = best.name
                pendingSince = nowMs
            }
        } else {
            pendingName = null
        }
        return if (displayed != null && nowMs - displayedAt <= holdMs) displayed else null
    }

    fun reset() {
        pendingName = null
        displayed = null
    }
}

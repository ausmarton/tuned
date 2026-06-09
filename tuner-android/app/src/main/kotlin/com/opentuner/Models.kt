package com.opentuner

import org.json.JSONObject

/** Per-string result from strum analysis (mirrors the native JSON). */
data class TuningString(
    val index: Int,
    val name: String,
    val targetHz: Float,
    val detectedHz: Float?,
    val centsOff: Float?,
    val direction: String?,
    val confidence: Float,
)

data class StrumReport(val strings: List<TuningString>)

data class ChordCandidate(val name: String, val score: Float)

data class ChordResult(
    val candidates: List<ChordCandidate>,
    val best: ChordCandidate?,
)

/** Parses the JSON strings produced by the native (JNI) tuner surface. */
object TunerJson {
    private fun JSONObject.floatOrNull(key: String): Float? = if (isNull(key)) null else getDouble(key).toFloat()

    private fun JSONObject.stringOrNull(key: String): String? = if (isNull(key)) null else getString(key)

    fun parseStrum(json: String): StrumReport {
        val arr = JSONObject(json).getJSONArray("strings")
        val strings =
            (0 until arr.length()).map { i ->
                val o = arr.getJSONObject(i)
                TuningString(
                    index = o.getInt("index"),
                    name = o.getString("name"),
                    targetHz = o.getDouble("targetHz").toFloat(),
                    detectedHz = o.floatOrNull("detectedHz"),
                    centsOff = o.floatOrNull("centsOff"),
                    direction = o.stringOrNull("direction"),
                    confidence = o.getDouble("confidence").toFloat(),
                )
            }
        return StrumReport(strings)
    }

    fun parseChord(json: String): ChordResult {
        val root = JSONObject(json)
        val arr = root.getJSONArray("candidates")
        val candidates =
            (0 until arr.length()).map { i ->
                val o = arr.getJSONObject(i)
                ChordCandidate(o.getString("name"), o.getDouble("score").toFloat())
            }
        val best =
            if (root.isNull("best")) {
                null
            } else {
                val o = root.getJSONObject("best")
                ChordCandidate(o.getString("name"), o.getDouble("score").toFloat())
            }
        return ChordResult(candidates, best)
    }
}

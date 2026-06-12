package com.opentuner

import org.json.JSONArray
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

/** A playable chord shape: one fret per string (null = muted, 0 = open). */
data class Voicing(val frets: List<Int?>)

data class ChordCandidate(
    val name: String,
    val score: Float,
    val rootPc: Int,
    val quality: String,
    val voicings: List<Voicing>,
)

data class ChordResult(
    val candidates: List<ChordCandidate>,
    val best: ChordCandidate?,
    /** String labels of the active tuning, low → high (for voicing columns). */
    val strings: List<String>,
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

    private fun parseVoicings(arr: JSONArray): List<Voicing> =
        (0 until arr.length()).map { i ->
            val inner = arr.getJSONArray(i)
            Voicing(
                (0 until inner.length()).map { j ->
                    if (inner.isNull(j)) null else inner.getInt(j)
                },
            )
        }

    private fun parseCandidate(o: JSONObject): ChordCandidate =
        ChordCandidate(
            name = o.getString("name"),
            score = o.getDouble("score").toFloat(),
            rootPc = o.optInt("rootPc", -1),
            quality = o.optString("quality", ""),
            voicings = if (o.has("voicings")) parseVoicings(o.getJSONArray("voicings")) else emptyList(),
        )

    fun parseChord(json: String): ChordResult {
        val root = JSONObject(json)
        val arr = root.getJSONArray("candidates")
        val candidates = (0 until arr.length()).map { parseCandidate(arr.getJSONObject(it)) }
        val best = if (root.isNull("best")) null else parseCandidate(root.getJSONObject("best"))
        val stringsArr = root.optJSONArray("strings")
        val strings =
            if (stringsArr == null) {
                emptyList()
            } else {
                (0 until stringsArr.length()).map { stringsArr.getString(it) }
            }
        return ChordResult(candidates, best, strings)
    }
}

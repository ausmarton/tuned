package com.opentuner

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlin.math.abs

private val GREEN = Color(0xFF2E7D32)
private val ORANGE = Color(0xFFE65100)

fun directionColor(cents: Float?): Color =
    when {
        cents == null -> Color.Gray
        abs(cents) <= 5f -> GREEN
        else -> ORANGE
    }

@Composable
fun TuningPicker(
    currentId: String,
    onSelect: (String) -> Unit,
) {
    var expanded by remember { mutableStateOf(false) }
    val label = SUPPORTED_TUNINGS.firstOrNull { it.first == currentId }?.second ?: currentId

    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        TextButton(onClick = { expanded = true }) { Text(label) }
        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            SUPPORTED_TUNINGS.forEach { (id, name) ->
                DropdownMenuItem(
                    text = { Text(name) },
                    onClick = {
                        onSelect(id)
                        expanded = false
                    },
                )
            }
        }
    }
}

// ---- Tune ----

@Composable
fun TuneScreen(snapshot: Snapshot?) {
    val hasPitch = snapshot != null && snapshot.hasPitch
    val name = if (hasPitch) snapshot!!.nearestStringName else "—"
    val cents = if (hasPitch) snapshot!!.centsOff else null
    val hz = if (hasPitch) snapshot!!.pitchHz else null
    val confidence = snapshot?.confidence ?: 0f
    val color = directionColor(cents)

    Column(horizontalAlignment = Alignment.CenterHorizontally, modifier = Modifier.fillMaxWidth()) {
        Text(name, fontSize = 72.sp, fontWeight = FontWeight.Bold, color = color, textAlign = TextAlign.Center)
        Text(hz?.let { "%.1f Hz".format(it) } ?: " ")
        Text(
            cents?.let {
                val dir =
                    when {
                        abs(it) <= 5f -> "in tune"
                        it < 0 -> "♭ flat"
                        else -> "♯ sharp"
                    }
                "%+.1f cents · %s".format(it, dir)
            } ?: "play a note",
            color = color,
        )
        Spacer(Modifier.height(12.dp))
        CentsBar(cents, height = 64.dp)
        Spacer(Modifier.height(8.dp))
        Text("confidence %.0f%%".format(confidence * 100))
    }
}

/** Horizontal ±50-cent meter with a moving needle. */
@Composable
fun CentsBar(
    cents: Float?,
    height: androidx.compose.ui.unit.Dp = 48.dp,
) {
    val color = directionColor(cents)
    Canvas(
        modifier =
            Modifier
                .fillMaxWidth()
                .height(height),
    ) {
        val w = size.width
        val h = size.height
        val midY = h / 2f
        val usable = w * 0.9f
        val left = (w - usable) / 2f
        val centerX = w / 2f

        drawLine(Color.LightGray, Offset(left, midY), Offset(left + usable, midY), strokeWidth = 4f)
        listOf(-50f, -25f, 0f, 25f, 50f).forEach { c ->
            val x = centerX + (c / 50f) * (usable / 2f)
            val tickH = if (c == 0f) h * 0.4f else h * 0.22f
            drawLine(
                if (c == 0f) GREEN else Color.Gray,
                Offset(x, midY - tickH),
                Offset(x, midY + tickH),
                strokeWidth = if (c == 0f) 5f else 3f,
            )
        }
        if (cents != null) {
            val clamped = cents.coerceIn(-50f, 50f)
            val x = centerX + (clamped / 50f) * (usable / 2f)
            drawLine(color, Offset(x, midY - h * 0.45f), Offset(x, midY + h * 0.45f), strokeWidth = 8f)
        }
    }
}

// ---- Strum ----

@Composable
fun StrumScreen(strings: List<SmoothedString>) {
    if (strings.isEmpty()) {
        Text("Strum your instrument — each string updates live as it rings.")
        return
    }
    Column(modifier = Modifier.fillMaxWidth()) {
        Text("Strum and tune — readings hold for a moment so you can adjust.")
        Spacer(Modifier.height(12.dp))
        strings.forEach { s ->
            val fade =
                if (s.centsOff == null) 0.35f else (1f - s.ageMs / 2000f).coerceIn(0.4f, 1f)
            Row(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .alpha(fade)
                        .padding(vertical = 6.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(s.name, modifier = Modifier.width(44.dp), fontWeight = FontWeight.Bold)
                Text(
                    s.centsOff?.let { "%+.1f¢".format(it) } ?: "—",
                    modifier = Modifier.width(64.dp),
                    color = directionColor(s.centsOff),
                )
                Column(modifier = Modifier.weight(1f)) {
                    CentsBar(s.centsOff, height = 28.dp)
                }
            }
        }
    }
}

// ---- Chords ----

@Composable
fun ChordScreen(chord: ChordResult?) {
    if (chord?.best == null) {
        Text("Play a chord — the name and fingerings appear here.")
        return
    }
    val best = chord.best
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        modifier = Modifier.fillMaxWidth().verticalScroll(rememberScrollState()),
    ) {
        Text(best.name, fontSize = 48.sp, fontWeight = FontWeight.Bold)
        Text("confidence %.0f%%".format(best.score * 100))
        Spacer(Modifier.height(12.dp))
        best.voicings.take(3).forEach { v ->
            VoicingLine(chord.strings, v)
            Spacer(Modifier.height(8.dp))
        }

        val alternates =
            chord.candidates.filter { it.name != best.name && it.score >= best.score - 0.08f }.take(2)
        if (alternates.isNotEmpty()) {
            Spacer(Modifier.height(8.dp))
            Text("Other matches", style = MaterialTheme.typography.titleSmall)
            alternates.forEach { c ->
                Spacer(Modifier.height(6.dp))
                Text("${c.name}  (%.0f%%)".format(c.score * 100))
                c.voicings.firstOrNull()?.let { VoicingLine(chord.strings, it) }
            }
        }
    }
}

/** Compact numeric chord shape: a fret per string under its label (x = muted). */
@Composable
fun VoicingLine(
    strings: List<String>,
    voicing: Voicing,
) {
    Row(horizontalArrangement = Arrangement.Center) {
        voicing.frets.forEachIndexed { i, f ->
            Column(
                horizontalAlignment = Alignment.CenterHorizontally,
                modifier = Modifier.padding(horizontal = 6.dp),
            ) {
                Text(
                    strings.getOrElse(i) { "" },
                    fontSize = 11.sp,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    f?.toString() ?: "x",
                    fontWeight = FontWeight.Bold,
                    fontFamily = FontFamily.Monospace,
                )
            }
        }
    }
}

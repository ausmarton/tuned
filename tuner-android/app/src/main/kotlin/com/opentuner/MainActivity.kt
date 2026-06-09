package com.opentuner

import android.Manifest
import android.content.pm.PackageManager
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalLifecycleOwner
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.viewmodel.compose.viewModel
import kotlin.math.abs

class MainActivity : ComponentActivity() {
    private var pendingStart: (() -> Unit)? = null

    private val requestPermission =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            if (granted) pendingStart?.invoke()
            pendingStart = null
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme {
                Surface(modifier = Modifier.fillMaxSize()) {
                    val vm: TunerViewModel = viewModel()
                    TunerScreen(
                        vm = vm,
                        onStart = { ensurePermissionThen { vm.start() } },
                    )
                }
            }
        }
    }

    private fun ensurePermissionThen(action: () -> Unit) {
        val granted =
            checkSelfPermission(Manifest.permission.RECORD_AUDIO) ==
                PackageManager.PERMISSION_GRANTED
        if (granted) {
            action()
        } else {
            pendingStart = action
            requestPermission.launch(Manifest.permission.RECORD_AUDIO)
        }
    }
}

@Composable
fun TunerScreen(
    vm: TunerViewModel,
    onStart: () -> Unit,
) {
    val state by vm.state.collectAsState()

    // Pause capture when the app goes to the background.
    val lifecycleOwner = LocalLifecycleOwner.current
    DisposableEffect(lifecycleOwner) {
        val observer =
            LifecycleEventObserver { _, event ->
                if (event == Lifecycle.Event.ON_STOP) vm.stop()
            }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
    }

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .verticalScroll(rememberScrollState())
                .padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text("OpenTuner", style = MaterialTheme.typography.headlineMedium)
        Spacer(Modifier.height(16.dp))

        TuningPicker(currentId = state.tuningId, onSelect = vm::setTuning)
        Spacer(Modifier.height(24.dp))

        PitchMeter(state.snapshot)
        Spacer(Modifier.height(16.dp))

        Button(onClick = { if (state.isRunning) vm.stop() else onStart() }) {
            Text(if (state.isRunning) "Stop" else "Start")
        }
        state.error?.let {
            Spacer(Modifier.height(8.dp))
            Text(it, color = MaterialTheme.colorScheme.error)
        }

        Spacer(Modifier.height(24.dp))
        HorizontalDivider()
        Spacer(Modifier.height(16.dp))

        Text("Strum analysis", style = MaterialTheme.typography.titleMedium)
        Spacer(Modifier.height(8.dp))
        OutlinedButton(onClick = vm::analyseStrum, enabled = state.isRunning) {
            Text("Analyse strum")
        }
        Spacer(Modifier.height(8.dp))
        StrumGrid(state.strum)

        Spacer(Modifier.height(24.dp))
        HorizontalDivider()
        Spacer(Modifier.height(16.dp))

        Text("Chord identification", style = MaterialTheme.typography.titleMedium)
        Spacer(Modifier.height(8.dp))
        OutlinedButton(onClick = vm::recogniseChord, enabled = state.isRunning) {
            Text("Identify chord")
        }
        Spacer(Modifier.height(8.dp))
        ChordView(state.chord)
    }
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

private fun directionColor(cents: Float?): Color =
    when {
        cents == null -> Color.Gray
        abs(cents) <= 5f -> Color(0xFF2E7D32) // green
        else -> Color(0xFFE65100) // orange
    }

@Composable
fun PitchMeter(snapshot: Snapshot?) {
    val hasPitch = snapshot != null && snapshot.hasPitch
    val name = if (hasPitch) snapshot!!.nearestStringName else "—"
    val cents = if (hasPitch) snapshot!!.centsOff else null
    val hz = if (hasPitch) snapshot!!.pitchHz else null
    val confidence = snapshot?.confidence ?: 0f
    val color = directionColor(cents)

    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Text(
            name,
            fontSize = 64.sp,
            fontWeight = FontWeight.Bold,
            color = color,
            textAlign = TextAlign.Center,
        )
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
        CentsBar(cents)
        Spacer(Modifier.height(8.dp))
        Text("confidence %.0f%%".format(confidence * 100))
    }
}

/** Horizontal ±50-cent meter with a moving needle. */
@Composable
fun CentsBar(cents: Float?) {
    val color = directionColor(cents)
    Canvas(
        modifier =
            Modifier
                .fillMaxWidth()
                .height(60.dp),
    ) {
        val w = size.width
        val h = size.height
        val midY = h / 2f
        val usable = w * 0.9f
        val left = (w - usable) / 2f
        val centerX = w / 2f

        // Baseline.
        drawLine(
            color = Color.LightGray,
            start = Offset(left, midY),
            end = Offset(left + usable, midY),
            strokeWidth = 4f,
        )
        // Tick marks at -50, -25, 0, +25, +50 cents.
        listOf(-50f, -25f, 0f, 25f, 50f).forEach { c ->
            val x = centerX + (c / 50f) * (usable / 2f)
            val tickH = if (c == 0f) h * 0.4f else h * 0.22f
            drawLine(
                color = if (c == 0f) Color(0xFF2E7D32) else Color.Gray,
                start = Offset(x, midY - tickH),
                end = Offset(x, midY + tickH),
                strokeWidth = if (c == 0f) 5f else 3f,
            )
        }
        // Needle.
        if (cents != null) {
            val clamped = cents.coerceIn(-50f, 50f)
            val x = centerX + (clamped / 50f) * (usable / 2f)
            drawLine(
                color = color,
                start = Offset(x, midY - h * 0.45f),
                end = Offset(x, midY + h * 0.45f),
                strokeWidth = 8f,
            )
        }
    }
}

@Composable
fun StrumGrid(report: StrumReport?) {
    if (report == null) {
        Text("Strum all strings, then tap Analyse strum.")
        return
    }
    Column(modifier = Modifier.fillMaxWidth()) {
        report.strings.forEach { s ->
            val status =
                when (s.direction) {
                    null -> "—"
                    "in_tune" -> "in tune"
                    "flat" -> "♭ flat"
                    else -> "♯ sharp"
                }
            Row(
                modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Text(s.name, modifier = Modifier.width(48.dp), fontWeight = FontWeight.Bold)
                Text(s.detectedHz?.let { "%.1f Hz".format(it) } ?: "—")
                Text(s.centsOff?.let { "%+.1f¢".format(it) } ?: "—")
                Text(status, color = directionColor(s.centsOff))
            }
        }
    }
}

@Composable
fun ChordView(result: ChordResult?) {
    if (result == null) {
        Text("Play a chord, then tap Identify chord.")
        return
    }
    Column(horizontalAlignment = Alignment.CenterHorizontally, modifier = Modifier.fillMaxWidth()) {
        Text(
            result.best?.name ?: "—",
            fontSize = 40.sp,
            fontWeight = FontWeight.Bold,
        )
        result.best?.let { Text("confidence %.0f%%".format(it.score * 100)) }
        Spacer(Modifier.height(4.dp))
        result.candidates.take(3).forEach { c ->
            Text("${c.name}  (%.0f%%)".format(c.score * 100))
        }
    }
}

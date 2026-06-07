package com.opentuner

import android.Manifest
import android.content.pm.PackageManager
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
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
        val granted = checkSelfPermission(Manifest.permission.RECORD_AUDIO) ==
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
fun TunerScreen(vm: TunerViewModel, onStart: () -> Unit) {
    val state by vm.state.collectAsState()

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Text("OpenTuner", style = MaterialTheme.typography.headlineMedium)
        Spacer(Modifier.height(16.dp))

        TuningPicker(
            currentId = state.tuningId,
            onSelect = vm::setTuning,
        )
        Spacer(Modifier.height(24.dp))

        PitchDisplay(state.snapshot)
        Spacer(Modifier.height(24.dp))

        Button(onClick = { if (state.isRunning) vm.stop() else onStart() }) {
            Text(if (state.isRunning) "Stop" else "Start")
        }
    }
}

@Composable
fun TuningPicker(currentId: String, onSelect: (String) -> Unit) {
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

@Composable
fun PitchDisplay(snapshot: Snapshot?) {
    val name = snapshot?.takeIf { it.hasPitch }?.nearestStringName ?: "—"
    val cents = snapshot?.takeIf { it.hasPitch }?.centsOff
    val hz = snapshot?.takeIf { it.hasPitch }?.pitchHz
    val confidence = snapshot?.confidence ?: 0f

    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        modifier = Modifier.fillMaxSize(fraction = 0f),
    ) {
        Text(name, style = MaterialTheme.typography.displayLarge, textAlign = TextAlign.Center)
        Spacer(Modifier.height(8.dp))
        Text(hz?.let { "%.1f Hz".format(it) } ?: "")
        Text(
            cents?.let {
                val dir = when {
                    abs(it) <= 5f -> "in tune"
                    it < 0 -> "♭ flat"
                    else -> "♯ sharp"
                }
                "%+.1f cents · %s".format(it, dir)
            } ?: "",
        )
        Text("confidence %.0f%%".format(confidence * 100))
    }
}

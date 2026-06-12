package com.opentuner

import android.Manifest
import android.content.pm.PackageManager
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.viewModels
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.unit.dp

class MainActivity : ComponentActivity() {
    private val vm: TunerViewModel by viewModels()

    private val requestPermission =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            if (granted) vm.start() else vm.onPermissionDenied()
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme {
                Surface(modifier = Modifier.fillMaxSize()) {
                    OpenTunerApp(vm)
                }
            }
        }
    }

    override fun onResume() {
        super.onResume()
        ensureListening()
    }

    override fun onStop() {
        super.onStop()
        vm.stop() // no background recording
    }

    private fun ensureListening() {
        val granted =
            checkSelfPermission(Manifest.permission.RECORD_AUDIO) ==
                PackageManager.PERMISSION_GRANTED
        if (granted) {
            vm.start()
        } else {
            requestPermission.launch(Manifest.permission.RECORD_AUDIO)
        }
    }
}

@Composable
fun OpenTunerApp(vm: TunerViewModel) {
    val state by vm.state.collectAsState()

    // Keep the screen awake while listening so the mic stays available.
    val view = LocalView.current
    LaunchedEffect(state.isRunning) { view.keepScreenOn = state.isRunning }

    val tabs =
        listOf(
            Triple(Mode.TUNE, "Tune", "♪"),
            Triple(Mode.STRUM, "Strum", "≣"),
            Triple(Mode.CHORDS, "Chords", "𝄞"),
        )

    Scaffold(
        bottomBar = {
            NavigationBar {
                tabs.forEach { (mode, label, glyph) ->
                    NavigationBarItem(
                        selected = state.mode == mode,
                        onClick = { vm.setMode(mode) },
                        icon = { Text(glyph) },
                        label = { Text(label) },
                    )
                }
            }
        },
    ) { padding ->
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .padding(horizontal = 24.dp, vertical = 12.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text("OpenTuner", style = MaterialTheme.typography.headlineSmall)
            Spacer(Modifier.height(8.dp))
            TuningPicker(currentId = state.tuningId, onSelect = vm::setTuning)
            if (!state.isRunning) {
                Spacer(Modifier.height(4.dp))
                Text(
                    state.error ?: "Paused — return to the app to resume listening.",
                    color = MaterialTheme.colorScheme.error,
                )
            }
            Spacer(Modifier.height(16.dp))

            when (state.mode) {
                Mode.TUNE -> TuneScreen(state.snapshot)
                Mode.STRUM -> StrumScreen(state.strum)
                Mode.CHORDS -> ChordScreen(state.chord)
            }
        }
    }
}

package com.opentuner

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.opentuner.audio.AudioEngine
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

/** The four tunings offered in the UI, in display order. */
val SUPPORTED_TUNINGS =
    listOf(
        "guitar.standard" to "Guitar — Standard (E A D G B E)",
        "bass.standard" to "Bass — Standard (E A D G)",
        "guitarra.lisboa" to "Guitarra Portuguesa — Lisboa",
        "guitarra.coimbra" to "Guitarra Portuguesa — Coimbra",
    )

data class TunerUiState(
    val isRunning: Boolean = false,
    val tuningId: String = "guitar.standard",
    val snapshot: Snapshot? = null,
    val strum: StrumReport? = null,
    val chord: ChordResult? = null,
    val error: String? = null,
)

class TunerViewModel : ViewModel() {
    private val tuner = NativeTuner()
    private val engine = AudioEngine(tuner)

    private val _state = MutableStateFlow(TunerUiState())
    val state: StateFlow<TunerUiState> = _state.asStateFlow()

    private var pollJob: Job? = null

    fun start() {
        if (_state.value.isRunning) return
        val started = engine.start(viewModelScope)
        if (!started) {
            _state.update { it.copy(error = "Could not start the microphone.") }
            return
        }
        _state.update { it.copy(isRunning = true, error = null) }
        pollJob =
            viewModelScope.launch {
                while (isActive) {
                    _state.update { it.copy(snapshot = tuner.snapshot()) }
                    delay(50)
                }
            }
    }

    fun stop() {
        pollJob?.cancel()
        pollJob = null
        engine.stop()
        _state.update { it.copy(isRunning = false) }
    }

    fun setTuning(tuningId: String) {
        if (tuner.setTuning(tuningId)) {
            _state.update { it.copy(tuningId = tuningId, strum = null, chord = null) }
        }
    }

    fun analyseStrum() {
        _state.update { it.copy(strum = tuner.analyseStrum()) }
    }

    fun recogniseChord() {
        _state.update { it.copy(chord = tuner.recogniseChord()) }
    }

    override fun onCleared() {
        stop()
        tuner.close()
        super.onCleared()
    }
}

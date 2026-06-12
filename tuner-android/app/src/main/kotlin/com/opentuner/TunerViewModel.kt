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

/** The three live screens. */
enum class Mode { TUNE, STRUM, CHORDS }

data class TunerUiState(
    val isRunning: Boolean = false,
    val tuningId: String = "guitar.standard",
    val mode: Mode = Mode.TUNE,
    val snapshot: Snapshot? = null,
    val strum: List<SmoothedString> = emptyList(),
    val chord: ChordResult? = null,
    val error: String? = null,
)

class TunerViewModel : ViewModel() {
    private val tuner = NativeTuner()
    private val engine = AudioEngine(tuner)
    private val strumSmoother = StrumSmoother()
    private val chordSmoother = ChordSmoother()

    private val _state = MutableStateFlow(TunerUiState())
    val state: StateFlow<TunerUiState> = _state.asStateFlow()

    private var pollJob: Job? = null

    /** Begin (or resume) continuous listening. Idempotent. */
    fun start() {
        if (_state.value.isRunning) return
        if (!engine.start(viewModelScope)) {
            _state.update { it.copy(error = "Could not start the microphone.") }
            return
        }
        _state.update { it.copy(isRunning = true, error = null) }
        pollJob =
            viewModelScope.launch {
                while (isActive) {
                    tick()
                    delay(POLL_MS)
                }
            }
    }

    private fun tick() {
        val now = System.currentTimeMillis()
        val snap = tuner.snapshot()
        when (_state.value.mode) {
            Mode.TUNE -> _state.update { it.copy(snapshot = snap) }
            Mode.STRUM -> {
                val report = tuner.analyseStrum() ?: StrumReport(emptyList())
                val smoothed = strumSmoother.update(report, now)
                _state.update { it.copy(snapshot = snap, strum = smoothed) }
            }
            Mode.CHORDS -> {
                val result = tuner.recogniseChord() ?: ChordResult(emptyList(), null, emptyList())
                val displayed = chordSmoother.update(result, now)
                _state.update { it.copy(snapshot = snap, chord = displayed) }
            }
        }
    }

    fun stop() {
        pollJob?.cancel()
        pollJob = null
        engine.stop()
        _state.update { it.copy(isRunning = false) }
    }

    fun onPermissionDenied() {
        _state.update { it.copy(error = "Microphone permission is required to listen.") }
    }

    fun setMode(mode: Mode) {
        strumSmoother.reset()
        chordSmoother.reset()
        _state.update { it.copy(mode = mode, strum = emptyList(), chord = null) }
    }

    fun setTuning(tuningId: String) {
        if (tuner.setTuning(tuningId)) {
            strumSmoother.reset()
            chordSmoother.reset()
            _state.update { it.copy(tuningId = tuningId, strum = emptyList(), chord = null) }
        }
    }

    override fun onCleared() {
        stop()
        tuner.close()
        super.onCleared()
    }

    private companion object {
        const val POLL_MS = 100L
    }
}

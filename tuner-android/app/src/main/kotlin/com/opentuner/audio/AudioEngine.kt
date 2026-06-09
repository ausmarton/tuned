package com.opentuner.audio

import android.annotation.SuppressLint
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import android.util.Log
import com.opentuner.NativeTuner
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking

/**
 * Captures mono float PCM and forwards it to [NativeTuner].
 *
 * Prefers the UNPROCESSED source (no AGC / noise suppression, best for pitch
 * detection) but falls back to MIC then DEFAULT on devices that don't support
 * it. [start] returns `false` if no source could be initialised, so the UI only
 * shows a running state when capture really began.
 */
class AudioEngine(
    private val tuner: NativeTuner,
    private val sampleRate: Int = 48_000,
) {
    private var job: Job? = null
    private var record: AudioRecord? = null

    private val sources =
        listOf(
            MediaRecorder.AudioSource.UNPROCESSED,
            MediaRecorder.AudioSource.MIC,
            MediaRecorder.AudioSource.DEFAULT,
        )

    @SuppressLint("MissingPermission")
    fun start(scope: CoroutineScope): Boolean {
        if (job?.isActive == true) return true

        val minBuf =
            AudioRecord.getMinBufferSize(
                sampleRate,
                AudioFormat.CHANNEL_IN_MONO,
                AudioFormat.ENCODING_PCM_FLOAT,
            )
        if (minBuf <= 0) {
            Log.e(TAG, "getMinBufferSize returned $minBuf")
            return false
        }
        val framesPerRead = 2048
        val bufferBytes = maxOf(minBuf, framesPerRead * 4 * 2)

        val rec = openFirstWorkingSource(bufferBytes) ?: return false
        record = rec

        rec.startRecording()
        if (rec.recordingState != AudioRecord.RECORDSTATE_RECORDING) {
            Log.e(TAG, "AudioRecord failed to enter RECORDING state")
            rec.release()
            record = null
            return false
        }

        job =
            scope.launch(Dispatchers.IO) {
                val buf = FloatArray(framesPerRead)
                try {
                    while (isActive) {
                        val n = rec.read(buf, 0, buf.size, AudioRecord.READ_BLOCKING)
                        if (n > 0) {
                            tuner.pushSamples(if (n == buf.size) buf else buf.copyOf(n))
                        }
                        ensureActive()
                    }
                } finally {
                    runCatching { rec.stop() }
                    rec.release()
                }
            }
        return true
    }

    @SuppressLint("MissingPermission")
    private fun openFirstWorkingSource(bufferBytes: Int): AudioRecord? {
        for (source in sources) {
            val rec =
                runCatching {
                    AudioRecord(
                        source,
                        sampleRate,
                        AudioFormat.CHANNEL_IN_MONO,
                        AudioFormat.ENCODING_PCM_FLOAT,
                        bufferBytes,
                    )
                }.getOrNull()
            if (rec != null && rec.state == AudioRecord.STATE_INITIALIZED) {
                return rec
            }
            rec?.release()
            Log.w(TAG, "audio source $source unavailable, trying next")
        }
        return null
    }

    fun stop() {
        val j = job ?: return
        job = null
        runBlocking { j.cancelAndJoin() }
        record = null
    }

    private companion object {
        const val TAG = "AudioEngine"
    }
}

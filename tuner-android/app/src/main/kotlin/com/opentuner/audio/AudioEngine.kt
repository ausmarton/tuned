package com.opentuner.audio

import android.annotation.SuppressLint
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
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
 * Captures mono float PCM at 48 kHz from the unprocessed audio source and
 * forwards it to [NativeTuner]. UNPROCESSED avoids OS effects (AGC, noise
 * suppression) that would corrupt pitch detection.
 *
 * The MVP uses [AudioRecord]; Oboe can be swapped in later behind the same
 * NativeTuner surface without UI changes.
 */
class AudioEngine(
    private val tuner: NativeTuner,
    private val sampleRate: Int = 48_000,
) {
    private var job: Job? = null

    @SuppressLint("MissingPermission")
    fun start(scope: CoroutineScope) {
        if (job?.isActive == true) return
        val minBuf =
            AudioRecord.getMinBufferSize(
                sampleRate,
                AudioFormat.CHANNEL_IN_MONO,
                AudioFormat.ENCODING_PCM_FLOAT,
            )
        val framesPerRead = 2048
        val bufferBytes = maxOf(minBuf, framesPerRead * 4 * 2)

        job =
            scope.launch(Dispatchers.IO) {
                val record =
                    AudioRecord(
                        MediaRecorder.AudioSource.UNPROCESSED,
                        sampleRate,
                        AudioFormat.CHANNEL_IN_MONO,
                        AudioFormat.ENCODING_PCM_FLOAT,
                        bufferBytes,
                    )
                try {
                    record.startRecording()
                    val buf = FloatArray(framesPerRead)
                    while (isActive) {
                        val n = record.read(buf, 0, buf.size, AudioRecord.READ_BLOCKING)
                        if (n > 0) {
                            tuner.pushSamples(if (n == buf.size) buf else buf.copyOf(n))
                        }
                        ensureActive()
                    }
                } finally {
                    runCatching { record.stop() }
                    record.release()
                }
            }
    }

    fun stop() {
        val j = job ?: return
        job = null
        runBlocking { j.cancelAndJoin() }
    }
}

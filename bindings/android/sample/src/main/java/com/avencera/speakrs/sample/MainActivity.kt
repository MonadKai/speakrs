package com.avencera.speakrs.sample

import android.app.Activity
import android.os.Bundle
import android.util.Log
import android.widget.TextView
import com.avencera.speakrs.ExecutionMode
import com.avencera.speakrs.SpeakrsPipeline
import com.avencera.speakrs.prepareModels
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File

class MainActivity : Activity() {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)
    private lateinit var status: TextView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        status = TextView(this)
        setContentView(status)

        val audioPath = intent.getStringExtra(EXTRA_AUDIO_PATH) ?: bundledAudioPath()
        val modelDir = intent.getStringExtra(EXTRA_MODEL_DIR)

        scope.launch {
            status.text = "Preparing models"
            val result = runCatching {
                withContext(Dispatchers.IO) {
                    diarizeOneFile(audioPath, modelDir)
                }
            }
            result.onSuccess { output ->
                Log.i(TAG, output)
            }.onFailure { error ->
                Log.e(TAG, error.message ?: error.toString(), error)
            }
            status.text = result.getOrElse { error -> error.message ?: error.toString() }
        }
    }

    private fun bundledAudioPath(): String {
        val output = File(cacheDir, "test_short.wav")
        if (!output.isFile) {
            assets.open("test_short.wav").use { input ->
                output.outputStream().use { input.copyTo(it) }
            }
        }
        return output.absolutePath
    }

    override fun onDestroy() {
        scope.cancel()
        super.onDestroy()
    }

    private fun diarizeOneFile(
        audioPath: String,
        modelDir: String?,
    ): String {
        val mode = ExecutionMode.CPU
        prepareModels(mode, cacheDir = null, modelDir = modelDir).use { prepared ->
            SpeakrsPipeline.fromPrepared(
                prepared = prepared,
                mode = mode,
                pipelineConfig = null,
                runtimeConfig = null,
            ).use { pipeline ->
                val result = pipeline.diarizeFile(
                    path = audioPath,
                    fileId = "android-sample",
                    pipelineConfig = null,
                    cancelToken = null,
                )
                return result.rttm.ifBlank { "${result.segments.size} segments" }
            }
        }
    }

    companion object {
        private const val TAG = "SpeakrsSample"
        const val EXTRA_AUDIO_PATH = "com.avencera.speakrs.sample.AUDIO_PATH"
        const val EXTRA_MODEL_DIR = "com.avencera.speakrs.sample.MODEL_DIR"
    }
}

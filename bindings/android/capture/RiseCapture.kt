// Compile-verified against android-35 (2026-10-10, kotlinc 2.0.21 + JDK 21); runtime proof pending an app process.
// Thin shell only: capture one frame or video clip, hand raw bytes to core/.
// No signing logic here. See README.md honesty header.
//
// FFI contract: core exposes `rise_hash_frame(bytes, len, out65)`.

package rise.android.capture

import android.content.Context
import android.graphics.Bitmap
import android.graphics.ImageFormat
import android.hardware.camera2.CameraCaptureSession
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CameraDevice
import android.hardware.camera2.CameraManager
import android.hardware.camera2.CaptureRequest
import android.media.ImageReader
import android.media.MediaMetadataRetriever
import android.media.MediaRecorder
import android.os.Build
import android.os.Handler
import android.os.HandlerThread
import android.util.Log
import android.util.Size
import android.view.Surface
import java.io.ByteArrayOutputStream
import java.io.File
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Single-frame and video Camera2 capture shell.
 *
 * - Back camera preferred, first available otherwise.
 * - Photo: ImageReader format is YUV_420_888 (raw sensor planes, no JPEG encode).
 * - Video: Camera2 TEMPLATE_RECORD + MediaRecorder (H.264 MP4), followed by
 *   up to [MAX_VIDEO_FRAMES] evenly spaced frames via [MediaMetadataRetriever].
 */
class RiseCapture(private val context: Context) {

    companion object {
        const val MAX_VIDEO_FRAMES = 256
        const val DEFAULT_VIDEO_DURATION_MS = 3000L
        private const val SAMPLE_INTERVAL_MS = 200L
        private const val JPEG_QUALITY = 90

        /**
         * Extracts up to [maxFrames] evenly spaced frames from [mp4File] using
         * [MediaMetadataRetriever.getFrameAtTime] and compresses each frame to JPEG.
         *
         * Call only AFTER [MediaRecorder] has been stopped and released.
         * Fail-closed: throws [CaptureError.SessionFailed] if no frames can be extracted.
         */
        fun extractVideoFramesJpeg(
            mp4File: File,
            maxFrames: Int = MAX_VIDEO_FRAMES,
        ): List<ByteArray> {
            if (!mp4File.exists() || mp4File.length() == 0L) {
                throw CaptureError.SessionFailed("recorded MP4 missing or empty")
            }
            val retriever = MediaMetadataRetriever()
            try {
                retriever.setDataSource(mp4File.absolutePath)
                val durationMs = retriever
                    .extractMetadata(MediaMetadataRetriever.METADATA_KEY_DURATION)
                    ?.toLongOrNull()
                    ?: 0L
                val durationUs = maxOf(durationMs, 0L) * 1000L
                val sampleCount = if (durationMs > 0L) {
                    ((durationMs / SAMPLE_INTERVAL_MS) + 1L).coerceIn(1L, maxOf(1, maxFrames).toLong()).toInt()
                } else {
                    1
                }
                Log.i(
                    "RISE",
                    "RISE_VIDEO_EXTRACT_PLAN: durationMs=$durationMs sampleCount=$sampleCount maxFrames=$maxFrames",
                )
                val frames = ArrayList<ByteArray>(sampleCount)
                for (i in 0 until sampleCount) {
                    val timeUs = if (sampleCount == 1) {
                        0L
                    } else {
                        (i.toLong() * durationUs) / sampleCount.toLong()
                    }
                    // getFrameAtTime may return null for the last timestamp(s) near EOF;
                    // try OPTION_CLOSEST first, then default sync fallback, and skip if still null.
                    val bitmap: Bitmap? = try {
                        retriever.getFrameAtTime(timeUs, MediaMetadataRetriever.OPTION_CLOSEST)
                            ?: retriever.getFrameAtTime(timeUs)
                    } catch (e: Exception) {
                        Log.w("RISE", "RISE_VIDEO_FRAME_WARN: idx=$i timeUs=$timeUs err=${e.message}")
                        null
                    }
                    if (bitmap == null) {
                        Log.w("RISE", "RISE_VIDEO_FRAME_NULL: idx=$i timeUs=$timeUs (skipped)")
                        continue
                    }
                    try {
                        val bos = ByteArrayOutputStream()
                        val ok = bitmap.compress(Bitmap.CompressFormat.JPEG, JPEG_QUALITY, bos)
                        val jpg = bos.toByteArray()
                        if (ok && jpg.size >= 2 && jpg[0] == 0xFF.toByte() && jpg[1] == 0xD8.toByte()) {
                            frames.add(jpg)
                        } else {
                            Log.w("RISE", "RISE_VIDEO_FRAME_COMPRESS_SKIP: idx=$i size=${jpg.size}")
                        }
                    } finally {
                        bitmap.recycle()
                    }
                }
                if (frames.isEmpty()) {
                    throw CaptureError.SessionFailed("no valid frames extracted from video")
                }
                return frames
            } finally {
                try {
                    retriever.release()
                } catch (_: Exception) {
                }
            }
        }
    }

    private var backgroundThread: HandlerThread? = null
    private var backgroundHandler: Handler? = null
    private var cameraDevice: CameraDevice? = null
    private var captureSession: CameraCaptureSession? = null
    private var imageReader: ImageReader? = null
    private var mediaRecorder: MediaRecorder? = null
    private var recorderSurface: Surface? = null
    private var activeVideoStop: (() -> Unit)? = null

    /** Raw frame result. Timestamp is capture time in ms since epoch. */
    data class RawFrame(
        val yuvBytes: ByteArray,
        val width: Int,
        val height: Int,
        val timestampMs: Long,
    ) {
        override fun equals(other: Any?): Boolean {
            if (this === other) return true
            if (other !is RawFrame) return false
            return width == other.width &&
                height == other.height &&
                timestampMs == other.timestampMs &&
                yuvBytes.contentEquals(other.yuvBytes)
        }

        override fun hashCode(): Int {
            var result = yuvBytes.contentHashCode()
            result = 31 * result + width
            result = 31 * result + height
            result = 31 * result + timestampMs.hashCode()
            return result
        }
    }

    /** Recorded video result returned after MediaRecorder is stopped and released. */
    data class VideoRecording(
        val mp4File: File,
        val width: Int,
        val height: Int,
        val timestampMs: Long,
    )

    /** Failure reasons, kept to one line each for the caller UI. */
    sealed class CaptureError(message: String) : Exception(message) {
        class NoCamera : CaptureError("no back camera available")
        class PermissionMissing : CaptureError("camera permission not granted")
        class SessionFailed(cause: String) : CaptureError("capture session failed: $cause")
    }

    /** True while a video recording session is currently active. */
    fun isRecordingVideo(): Boolean = activeVideoStop != null

    /** Stops an active video recording early if one is running. */
    fun stopVideoRecording() {
        activeVideoStop?.invoke()
    }

    /**
     * Captures a video clip to [outputFile] using Camera2 + [MediaRecorder],
     * automatically stopping after [durationMs] (or when [stopVideoRecording]
     * is called). Releases [MediaRecorder] and the camera BEFORE invoking
     * [onComplete] so [MediaMetadataRetriever] can safely read [outputFile].
     */
    fun recordVideo(
        outputFile: File,
        durationMs: Long = DEFAULT_VIDEO_DURATION_MS,
        onComplete: (Result<VideoRecording>) -> Unit,
    ) {
        val manager = context.getSystemService(Context.CAMERA_SERVICE) as CameraManager
        val cameraId = pickBackCamera(manager)
        if (cameraId == null) {
            onComplete(Result.failure(CaptureError.NoCamera()))
            return
        }
        val characteristics = try {
            manager.getCameraCharacteristics(cameraId)
        } catch (e: Exception) {
            onComplete(Result.failure(CaptureError.SessionFailed(e.message ?: "characteristics failed")))
            return
        }
        val map = characteristics.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP)
        val videoSize: Size = map
            ?.getOutputSizes(MediaRecorder::class.java)
            ?.filter { it.width <= 1920 && it.height <= 1080 }
            ?.maxByOrNull { it.width * it.height }
            ?: Size(1280, 720)

        startBackgroundThread()
        val handler = backgroundHandler
        if (handler == null) {
            onComplete(Result.failure(CaptureError.SessionFailed("no background handler")))
            return
        }

        if (outputFile.exists()) {
            outputFile.delete()
        }

        val recorder = try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                MediaRecorder(context)
            } else {
                @Suppress("DEPRECATION")
                MediaRecorder()
            }.apply {
                setVideoSource(MediaRecorder.VideoSource.SURFACE)
                setOutputFormat(MediaRecorder.OutputFormat.MPEG_4)
                setOutputFile(outputFile.absolutePath)
                setVideoEncodingBitRate(4_000_000)
                setVideoFrameRate(30)
                setVideoSize(videoSize.width, videoSize.height)
                setVideoEncoder(MediaRecorder.VideoEncoder.H264)
                prepare()
            }
        } catch (e: Exception) {
            close()
            onComplete(Result.failure(CaptureError.SessionFailed("recorder prepare: ${e.message}")))
            return
        }
        mediaRecorder = recorder
        val surface = recorder.surface
        recorderSurface = surface
        val startTimestampMs = System.currentTimeMillis()
        val finished = AtomicBoolean(false)

        val finishRecording: (Throwable?) -> Unit = { error ->
            if (finished.compareAndSet(false, true)) {
                activeVideoStop = null
                var stopErr: Throwable? = error
                try {
                    captureSession?.stopRepeating()
                } catch (_: Exception) {
                }
                try {
                    captureSession?.close()
                } catch (_: Exception) {
                }
                captureSession = null
                try {
                    if (stopErr == null) {
                        recorder.stop()
                    }
                } catch (e: Exception) {
                    stopErr = e
                }
                // Release MediaRecorder BEFORE MediaMetadataRetriever reads the file.
                releaseMediaRecorder()
                close()
                if (stopErr != null || !outputFile.exists() || outputFile.length() == 0L) {
                    if (outputFile.exists()) outputFile.delete()
                    val msg = stopErr?.message ?: "empty video output"
                    onComplete(Result.failure(CaptureError.SessionFailed(msg)))
                } else {
                    onComplete(
                        Result.success(
                            VideoRecording(
                                mp4File = outputFile,
                                width = videoSize.width,
                                height = videoSize.height,
                                timestampMs = startTimestampMs,
                            ),
                        ),
                    )
                }
            }
        }

        activeVideoStop = {
            handler.post { finishRecording(null) }
        }

        try {
            @Suppress("MissingPermission")
            manager.openCamera(cameraId, object : CameraDevice.StateCallback() {
                override fun onOpened(camera: CameraDevice) {
                    cameraDevice = camera
                    startVideoSession(camera, surface, handler, durationMs, finishRecording)
                }

                override fun onDisconnected(camera: CameraDevice) {
                    camera.close()
                    cameraDevice = null
                    finishRecording(CaptureError.SessionFailed("camera disconnected"))
                }

                override fun onError(camera: CameraDevice, error: Int) {
                    camera.close()
                    cameraDevice = null
                    finishRecording(CaptureError.SessionFailed("device error $error"))
                }
            }, handler)
        } catch (e: SecurityException) {
            finishRecording(CaptureError.PermissionMissing())
        } catch (e: Exception) {
            finishRecording(CaptureError.SessionFailed(e.message ?: "open failed"))
        }
    }

    private fun startVideoSession(
        camera: CameraDevice,
        surface: Surface,
        handler: Handler,
        durationMs: Long,
        finishRecording: (Throwable?) -> Unit,
    ) {
        try {
            val requestBuilder = camera.createCaptureRequest(CameraDevice.TEMPLATE_RECORD)
            requestBuilder.addTarget(surface)
            requestBuilder.set(CaptureRequest.CONTROL_MODE, CaptureRequest.CONTROL_MODE_AUTO)
            @Suppress("DEPRECATION")
            camera.createCaptureSession(
                listOf(surface),
                object : CameraCaptureSession.StateCallback() {
                    override fun onConfigured(session: CameraCaptureSession) {
                        captureSession = session
                        try {
                            session.setRepeatingRequest(requestBuilder.build(), null, handler)
                            mediaRecorder?.start()
                            Log.i("RISE", "RISE_VIDEO_RECORDING_STARTED: durationMs=$durationMs")
                            handler.postDelayed({ finishRecording(null) }, durationMs)
                        } catch (e: Exception) {
                            finishRecording(CaptureError.SessionFailed(e.message ?: "video start failed"))
                        }
                    }

                    override fun onConfigureFailed(session: CameraCaptureSession) {
                        finishRecording(CaptureError.SessionFailed("video session configure failed"))
                    }
                },
                handler,
            )
        } catch (e: Exception) {
            finishRecording(CaptureError.SessionFailed(e.message ?: "video request failed"))
        }
    }

    /**
     * Captures exactly one frame, then closes the camera.
     * Must be called after [RiseCameraPermission.isGranted] returns true.
     */
    fun captureSingleFrame(onFrame: (Result<RawFrame>) -> Unit) {
        // Every outcome funnels through here exactly once: resources are always
        // released and the caller always gets a result, never a stuck callback.
        val completed = AtomicBoolean(false)
        fun complete(result: Result<RawFrame>) {
            if (completed.compareAndSet(false, true)) {
                try {
                    close()
                } catch (_: Exception) {
                }
                onFrame(result)
            }
        }
        val manager = context.getSystemService(Context.CAMERA_SERVICE) as CameraManager
        val cameraId = pickBackCamera(manager)
        if (cameraId == null) {
            complete(Result.failure(CaptureError.NoCamera()))
            return
        }
        val characteristics = manager.getCameraCharacteristics(cameraId)
        val map = characteristics.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP)
        val size: Size = map
            ?.getOutputSizes(ImageFormat.YUV_420_888)
            ?.maxByOrNull { it.width * it.height }
            ?: Size(1280, 720)

        startBackgroundThread()
        val handler = backgroundHandler
        if (handler == null) {
            complete(Result.failure(CaptureError.SessionFailed("no background handler")))
            return
        }

        val reader = ImageReader.newInstance(
            size.width,
            size.height,
            ImageFormat.YUV_420_888,
            2,
        )
        imageReader = reader
        reader.setOnImageAvailableListener({ available ->
            val image = available.acquireLatestImage()
            if (image == null) {
                complete(Result.failure(CaptureError.SessionFailed("empty image")))
                return@setOnImageAvailableListener
            }
            try {
                val bytes = readYuvPlanes(image.planes.map { plane ->
                    val buffer = plane.buffer
                    // Rewind direct ByteBuffer before reading remaining bytes.
                    buffer.rewind()
                    val rowStride = plane.rowStride
                    val pixelStride = plane.pixelStride
                    // Direct ByteBuffers (common on MIUI/Snapdragon) don't support
                    // .array() — copy via get() which works for both.
                    val copy = ByteArray(buffer.remaining())
                    buffer.get(copy)
                    PlaneData(copy, rowStride, pixelStride)
                })
                val frame = RawFrame(
                    yuvBytes = bytes,
                    width = image.width,
                    height = image.height,
                    timestampMs = System.currentTimeMillis(),
                )
                image.close()
                complete(Result.success(frame))
            } catch (e: Exception) {
                try {
                    image.close()
                } catch (_: Exception) {
                }
                complete(Result.failure(CaptureError.SessionFailed(e.message ?: "frame read failed")))
            }
        }, handler)

        try {
            @Suppress("MissingPermission")
            manager.openCamera(cameraId, object : CameraDevice.StateCallback() {
                override fun onOpened(camera: CameraDevice) {
                    cameraDevice = camera
                    createSingleCapture(camera, reader, handler, ::complete)
                }

                override fun onDisconnected(camera: CameraDevice) {
                    camera.close()
                    cameraDevice = null
                    complete(Result.failure(CaptureError.SessionFailed("camera disconnected")))
                }

                override fun onError(camera: CameraDevice, error: Int) {
                    camera.close()
                    cameraDevice = null
                    complete(Result.failure(CaptureError.SessionFailed("device error $error")))
                }
            }, handler)
        } catch (e: SecurityException) {
            complete(Result.failure(CaptureError.PermissionMissing()))
        } catch (e: Exception) {
            complete(Result.failure(CaptureError.SessionFailed(e.message ?: "open failed")))
        }
    }

    private fun createSingleCapture(
        camera: CameraDevice,
        reader: ImageReader,
        handler: Handler,
        onFrame: (Result<RawFrame>) -> Unit,
    ) {
        try {
            val requestBuilder = camera.createCaptureRequest(CameraDevice.TEMPLATE_STILL_CAPTURE)
            requestBuilder.addTarget(reader.surface)
            // Lock auto-exposure/white-balance at shutter defaults; no filters.
            requestBuilder.set(CaptureRequest.CONTROL_MODE, CaptureRequest.CONTROL_MODE_AUTO)
            @Suppress("DEPRECATION")
            camera.createCaptureSession(
                listOf(reader.surface),
                object : CameraCaptureSession.StateCallback() {
                    override fun onConfigured(session: CameraCaptureSession) {
                        try {
                            session.capture(requestBuilder.build(), null, handler)
                        } catch (e: Exception) {
                            onFrame(Result.failure(CaptureError.SessionFailed(e.message ?: "capture failed")))
                        }
                    }

                    override fun onConfigureFailed(session: CameraCaptureSession) {
                        onFrame(Result.failure(CaptureError.SessionFailed("configure failed")))
                    }
                },
                handler,
            )
        } catch (e: Exception) {
            onFrame(Result.failure(CaptureError.SessionFailed(e.message ?: "request failed")))
        }
    }

    private fun releaseMediaRecorder() {
        try {
            mediaRecorder?.reset()
        } catch (_: Exception) {
        }
        try {
            mediaRecorder?.release()
        } catch (_: Exception) {
        }
        mediaRecorder = null
        try {
            recorderSurface?.release()
        } catch (_: Exception) {
        }
        recorderSurface = null
    }

    /** Releases camera + recorder + reader + background thread. Safe to call twice. */
    fun close() {
        activeVideoStop = null
        try {
            captureSession?.close()
        } catch (_: Exception) {
        }
        captureSession = null
        releaseMediaRecorder()
        try {
            cameraDevice?.close()
        } catch (_: Exception) {
        }
        cameraDevice = null
        try {
            imageReader?.close()
        } catch (_: Exception) {
        }
        imageReader = null
        stopBackgroundThread()
    }

    private fun pickBackCamera(manager: CameraManager): String? {
        val ids = try {
            manager.cameraIdList
        } catch (_: Exception) {
            return null
        }
        for (id in ids) {
            val chars = try {
                manager.getCameraCharacteristics(id)
            } catch (_: Exception) {
                continue
            }
            if (chars.get(CameraCharacteristics.LENS_FACING) == CameraCharacteristics.LENS_FACING_BACK) {
                return id
            }
        }
        return ids.firstOrNull()
    }

    private data class PlaneData(val bytes: ByteArray, val rowStride: Int, val pixelStride: Int)

    /**
     * Concatenates Y/U/V planes into one byte array.
     * Raw copy only: no color conversion, no JPEG, no filtering.
     */
    private fun readYuvPlanes(planes: List<PlaneData>): ByteArray {
        var total = 0
        for (plane in planes) total += plane.bytes.size
        val out = ByteArray(total)
        var offset = 0
        for (plane in planes) {
            plane.bytes.copyInto(out, offset)
            offset += plane.bytes.size
        }
        return out
    }

    private fun startBackgroundThread() {
        stopBackgroundThread()
        val thread = HandlerThread("RiseCapture")
        thread.start()
        backgroundThread = thread
        backgroundHandler = Handler(thread.looper)
    }

    private fun stopBackgroundThread() {
        try {
            backgroundThread?.quitSafely()
            if (Thread.currentThread() !== backgroundThread) {
                backgroundThread?.join(1000)
            }
        } catch (_: Exception) {
        }
        backgroundThread = null
        backgroundHandler = null
    }
}

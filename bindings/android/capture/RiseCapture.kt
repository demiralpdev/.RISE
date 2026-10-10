// STUB — NOT compiled/tested (no Android SDK on this machine).
// Thin shell only: capture one frame, hand raw bytes to core/.
// No signing logic here. See README.md honesty header.
//
// FFI contract: core exposes `rise_hash_frame(bytes, len, out65)`.
// Future JNI bridge calls it (TODO, not implemented here).

package rise.android.capture

import android.content.Context
import android.graphics.ImageFormat
import android.hardware.camera2.CameraCaptureSession
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CameraDevice
import android.hardware.camera2.CameraManager
import android.hardware.camera2.CaptureRequest
import android.media.ImageReader
import android.os.Handler
import android.os.HandlerThread
import android.util.Size

/**
 * Single-frame Camera2 capture with a raw YUV path.
 *
 * - Back camera preferred, first available otherwise.
 * - ImageReader format is YUV_420_888 (raw sensor planes, no JPEG encode).
 * - [onFrame] receives concatenated plane bytes (Y + U + V rows, no padding
 *   claim: row stride is respected per plane).
 * - Caller hashes the bytes with core (`rise_hash_frame` via future JNI).
 */
class RiseCapture(private val context: Context) {

    private var backgroundThread: HandlerThread? = null
    private var backgroundHandler: Handler? = null
    private var cameraDevice: CameraDevice? = null
    private var imageReader: ImageReader? = null

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

    /** Failure reasons, kept to one line each for the caller UI. */
    sealed class CaptureError(message: String) : Exception(message) {
        class NoCamera : CaptureError("no back camera available")
        class PermissionMissing : CaptureError("camera permission not granted")
        class SessionFailed(cause: String) : CaptureError("capture session failed: $cause")
    }

    /**
     * Captures exactly one frame, then closes the camera.
     * Must be called after [RiseCameraPermission.isGranted] returns true.
     */
    fun captureSingleFrame(onFrame: (Result<RawFrame>) -> Unit) {
        val manager = context.getSystemService(Context.CAMERA_SERVICE) as CameraManager
        val cameraId = pickBackCamera(manager)
        if (cameraId == null) {
            onFrame(Result.failure(CaptureError.NoCamera()))
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
            onFrame(Result.failure(CaptureError.SessionFailed("no background handler")))
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
                onFrame(Result.failure(CaptureError.SessionFailed("empty image")))
                return@setOnImageAvailableListener
            }
            try {
                val bytes = readYuvPlanes(image.planes.map { plane ->
                    val buffer = plane.buffer
                    val rowStride = plane.rowStride
                    val pixelStride = plane.pixelStride
                    PlaneData(buffer.array().copyOfRange(buffer.position(), buffer.remaining()), rowStride, pixelStride)
                })
                onFrame(
                    Result.success(
                        RawFrame(
                            yuvBytes = bytes,
                            width = image.width,
                            height = image.height,
                            timestampMs = System.currentTimeMillis(),
                        ),
                    ),
                )
            } finally {
                image.close()
                close()
            }
        }, handler)

        try {
            @Suppress("MissingPermission")
            manager.openCamera(cameraId, object : CameraDevice.StateCallback() {
                override fun onOpened(camera: CameraDevice) {
                    cameraDevice = camera
                    createSingleCapture(camera, reader, handler, onFrame)
                }

                override fun onDisconnected(camera: CameraDevice) {
                    camera.close()
                    cameraDevice = null
                }

                override fun onError(camera: CameraDevice, error: Int) {
                    camera.close()
                    cameraDevice = null
                    onFrame(Result.failure(CaptureError.SessionFailed("device error $error")))
                }
            }, handler)
        } catch (e: SecurityException) {
            onFrame(Result.failure(CaptureError.PermissionMissing()))
        } catch (e: Exception) {
            onFrame(Result.failure(CaptureError.SessionFailed(e.message ?: "open failed")))
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

    /** Releases camera + reader + background thread. Safe to call twice. */
    fun close() {
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
     * TODO: pass this array to core via JNI `rise_hash_frame(bytes, len, out65)`.
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
            backgroundThread?.join(1000)
        } catch (_: Exception) {
        }
        backgroundThread = null
        backgroundHandler = null
    }
}

package rise.android.app

import android.Manifest
import android.app.Activity
import android.content.pm.PackageManager
import android.os.Bundle
import android.util.Log
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView
import com.rise.core.RiseCore
import rise.android.capture.RiseCapture
import rise.android.capture.RiseCameraPermission

/** Wave 1: tap -> camera permission -> one raw frame -> core hash via JNI ->
 *  YUV bytes saved for cross-platform comparison. */
class MainActivity : Activity() {
    private lateinit var status: TextView
    private lateinit var capture: RiseCapture

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        capture = RiseCapture(this)
        val layout = LinearLayout(this)
        layout.orientation = LinearLayout.VERTICAL
        layout.setPadding(48, 48, 48, 48)
        status = TextView(this)
        status.text = "RISE ready - tap to seal a frame"
        val btn = Button(this)
        btn.text = "CAPTURE + SEAL"
        btn.setOnClickListener {
            if (RiseCameraPermission.isGranted(this)) doCapture() else RiseCameraPermission.request(this)
        }
        layout.addView(btn)
        layout.addView(status)
        setContentView(layout)
    }

    override fun onRequestPermissionsResult(
        code: Int,
        permissions: Array<out String>,
        results: IntArray,
    ) {
        super.onRequestPermissionsResult(code, permissions, results)
        if (code == RiseCameraPermission.REQUEST_CODE &&
            results.isNotEmpty() &&
            results[0] == PackageManager.PERMISSION_GRANTED
        ) {
            doCapture()
        }
    }

    private fun doCapture() {
        status.text = "capturing..."
        capture.captureSingleFrame { result ->
            runOnUiThread {
                result.onSuccess { raw ->
                    val hex = RiseCore.hashFrame(raw.yuvBytes)
                    Log.i("RISE", "HASH=$hex")
                    Log.i("RISE", "BYTES=${raw.yuvBytes.size} W=${raw.width} H=${raw.height} T=${raw.timestampMs}")
                    val f = java.io.File(getExternalFilesDir(null), "frame-001.yuv")
                    f.writeBytes(raw.yuvBytes)
                    Log.i("RISE", "SAVED=${f.absolutePath}")
                    status.text = "SEALED: ${raw.yuvBytes.size} bytes\nHASH=$hex"
                }.onFailure { e ->
                    status.text = "FAIL: ${e.message}"
                    Log.i("RISE", "FAIL=${e.message}")
                }
            }
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        capture.close()
    }
}

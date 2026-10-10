// STUB — NOT compiled/tested (no Android SDK on this machine).
// Thin shell only: camera permission flow with rationale + settings deep-link.

package rise.android.capture

import android.Manifest
import android.app.Activity
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.provider.Settings
import androidx.activity.result.ActivityResultLauncher
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat

/**
 * Camera permission flow (A-101).
 *
 * Order: check -> rationale (if user denied once) -> system request ->
 * settings deep-link (if permanently denied).
 *
 * The caller owns the ActivityResultLauncher for
 * ActivityResultContracts.RequestPermission and forwards its boolean
 * result to [onPermissionResult].
 */
object RiseCameraPermission {

    const val CAMERA = Manifest.permission.CAMERA

    /** True when CAMERA is already granted. */
    fun isGranted(context: Context): Boolean =
        ContextCompat.checkSelfPermission(context, CAMERA) == PackageManager.PERMISSION_GRANTED

    /**
     * True when the system recommends showing a rationale first
     * (user denied at least once but did not select "don't ask again").
     */
    fun shouldShowRationale(activity: Activity): Boolean =
        ActivityCompat.shouldShowRequestPermissionRationale(activity, CAMERA)

    /**
     * Requests CAMERA via the caller's launcher.
     * Call only when [isGranted] is false.
     */
    fun request(launcher: ActivityResultLauncher<String>) {
        launcher.launch(CAMERA)
    }

    /**
     * Handles the launcher result.
     * - granted=true: proceed to [RiseCapture.captureSingleFrame].
     * - granted=false + rationale available: show rationale UI, ask again.
     * - granted=false + no rationale: user picked "don't ask again",
     *   send them to app settings via [openAppSettings].
     */
    fun onPermissionResult(
        activity: Activity,
        granted: Boolean,
        onGranted: () -> Unit,
        onShowRationale: () -> Unit,
        onPermanentlyDenied: () -> Unit,
    ) {
        if (granted) {
            onGranted()
            return
        }
        if (shouldShowRationale(activity)) {
            onShowRationale()
        } else {
            onPermanentlyDenied()
        }
    }

    /** Deep-link to this app's Settings page so the user can re-enable CAMERA. */
    fun openAppSettings(context: Context) {
        val intent = Intent(
            Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
            Uri.fromParts("package", context.packageName, null),
        )
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        context.startActivity(intent)
    }

    /** One-line rationale text for the app UI (English, fixed wording). */
    fun rationaleText(): String =
        "Camera access is needed to capture the frame that core seals. " +
        "No photo is stored or shared without your action."
}

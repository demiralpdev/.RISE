// Thin shell only: camera permission flow with rationale + settings deep-link.
// Compile-verified against android-35 (2026-10-10, platform APIs only —
// no AndroidX); runtime proof pending an app process.

package rise.android.capture

import android.Manifest
import android.app.Activity
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.provider.Settings

/**
 * Camera permission flow (A-101).
 *
 * Order: check -> rationale (if user denied once) -> system request ->
 * settings deep-link (if permanently denied).
 *
 * Classic request flow (no AndroidX): [request] launches the system dialog
 * with [REQUEST_CODE]; the Activity forwards the grant result to
 * [onPermissionResult] from onRequestPermissionsResult.
 */
object RiseCameraPermission {

    const val CAMERA = Manifest.permission.CAMERA

    /** Request code for the classic requestPermissions flow. */
    const val REQUEST_CODE = 4211

    /** True when CAMERA is already granted. */
    fun isGranted(context: Context): Boolean =
        context.checkSelfPermission(CAMERA) == PackageManager.PERMISSION_GRANTED

    /**
     * True when the system recommends showing a rationale first
     * (user denied at least once but did not select "don't ask again").
     */
    fun shouldShowRationale(activity: Activity): Boolean =
        activity.shouldShowRequestPermissionRationale(CAMERA)

    /**
     * Requests CAMERA via the classic system dialog.
     * Call only when [isGranted] is false. The result arrives in the
     * Activity's onRequestPermissionsResult; forward it to [onPermissionResult].
     */
    fun request(activity: Activity) {
        activity.requestPermissions(arrayOf(CAMERA), REQUEST_CODE)
    }

    /**
     * Handles the requestPermissions result.
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

// Compile-verified against android-35 (2026-10-10, kotlinc 2.0.21 + JDK 21); runtime proof pending an app process.
// Thin shell only: Play Integrity verdict parsing + STRONG gating.
// No signing logic here. Verdicts are read, never invented.

package rise.android.attest

/**
 * Play Integrity gating (A-102).
 *
 * Verdict labels come from the Play Integrity API server response:
 * - MEETS_STRONG_INTEGRITY: hardware-backed, Android 13+ with security patch.
 * - MEETS_DEVICE_INTEGRITY: booted from a recognized image.
 * - MEETS_BASIC_INTEGRITY: passed basic checks only (emulator risk).
 * - Empty verdict: no integrity signal at all.
 *
 * Rule (gold gate): no STRONG verdict, no gold-L4. Never upgrade on doubt.
 */
object RiseIntegrity {

    /** Parsed device-integrity labels, in order of strength. */
    enum class DeviceVerdict {
        STRONG,
        DEVICE,
        BASIC,
        NONE,
    }

    /** Minimal parsed response. Real parsing lives in verify-web; the shell only reads labels. */
    data class IntegritySignal(
        val deviceVerdict: DeviceVerdict,
        val appLicensed: Boolean,
        val accountLicensed: Boolean,
    )

    /**
     * Maps raw deviceRecognitionVerdict strings to [DeviceVerdict].
     * Picks the strongest label present.
     */
    fun parseDeviceVerdict(labels: List<String>): DeviceVerdict {
        if (labels.contains("MEETS_STRONG_INTEGRITY")) return DeviceVerdict.STRONG
        if (labels.contains("MEETS_DEVICE_INTEGRITY")) return DeviceVerdict.DEVICE
        if (labels.contains("MEETS_BASIC_INTEGRITY")) return DeviceVerdict.BASIC
        return DeviceVerdict.NONE
    }

    /** True only for a STRONG verdict with a licensed app + account. */
    fun isStrong(signal: IntegritySignal): Boolean =
        signal.deviceVerdict == DeviceVerdict.STRONG &&
            signal.appLicensed &&
            signal.accountLicensed

    /**
     * True when the chain is at least basically clean:
     * BASIC or better plus a licensed app. Used for the L2-max row.
     */
    fun isBasicClean(signal: IntegritySignal): Boolean =
        (signal.deviceVerdict == DeviceVerdict.BASIC ||
            signal.deviceVerdict == DeviceVerdict.DEVICE ||
            signal.deviceVerdict == DeviceVerdict.STRONG) &&
            signal.appLicensed
}

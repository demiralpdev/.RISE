// STUB — NOT compiled/tested (no Android SDK on this machine).
// Thin shell only: create a P-256 key in AndroidKeyStore, StrongBox-first.
// No signing logic here; signing happens in core/. The key never leaves
// hardware and is never logged.

package rise.android.attest

import android.content.pm.PackageManager
import android.os.Build
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.security.keystore.StrongBoxUnavailableException
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.spec.ECGenParameterSpec

/**
 * Hardware key handling (A-102).
 *
 * - Algorithm: P-256 (secp256r1), ECDSA, SHA-256 digest.
 * - Store: AndroidKeyStore, StrongBox-first with TEE fallback.
 * - Attestation: KeyMint attestation challenge recorded at keygen.
 * - The private key material is never read, exported, or logged.
 */
object RiseKeystore {

    const val ANDROID_KEYSTORE = "AndroidKeyStore"
    const val DEFAULT_ALIAS = "rise-frame-key"
    const val CURVE_P256 = "secp256r1"

    /** Where the key actually landed. Reported to core for the tier decision. */
    enum class KeyHome {
        STRONG_BOX,
        TEE,
        SOFTWARE_FALLBACK,
    }

    data class KeyInfo(
        val alias: String,
        val home: KeyHome,
        val attestationChallenge: ByteArray,
    ) {
        override fun equals(other: Any?): Boolean {
            if (this === other) return true
            if (other !is KeyInfo) return false
            // Challenge bytes are secret-adjacent: compare but never print.
            return alias == other.alias &&
                home == other.home &&
                attestationChallenge.contentEquals(other.attestationChallenge)
        }

        override fun hashCode(): Int {
            var result = alias.hashCode()
            result = 31 * result + home.hashCode()
            result = 31 * result + attestationChallenge.contentHashCode()
            return result
        }

        // No toString override on purpose: never print challenge bytes.
    }

    /** True when this device advertises a StrongBox security chip. */
    fun hasStrongBox(packageManager: PackageManager): Boolean =
        Build.VERSION.SDK_INT >= Build.VERSION_CODES.P &&
            packageManager.hasSystemFeature(PackageManager.FEATURE_STRONGBOX_KEYSTORE)

    /**
     * Creates (or reuses) the P-256 frame key.
     *
     * StrongBox is tried first; on [StrongBoxUnavailableException] the same
     * spec is retried without StrongBox (TEE). Returns where the key lives
     * so the downgrade table can gate gold correctly.
     *
     * @param challenge fresh server nonce for KeyMint attestation.
     */
    fun ensureP256Key(alias: String, challenge: ByteArray): KeyInfo {
        if (hasKey(alias)) {
            // Existing key: home is re-read, nothing secret is returned.
            return KeyInfo(alias, readHome(alias), challenge)
        }
        // StrongBox-first attempt.
        try {
            generate(alias, challenge, strongBox = true)
            return KeyInfo(alias, KeyHome.STRONG_BOX, challenge)
        } catch (e: StrongBoxUnavailableException) {
            generate(alias, challenge, strongBox = false)
            return KeyInfo(alias, readHome(alias), challenge)
        }
    }

    /** True when the alias already exists in AndroidKeyStore. */
    fun hasKey(alias: String): Boolean {
        val store = KeyStore.getInstance(ANDROID_KEYSTORE)
        store.load(null)
        return store.containsAlias(alias)
    }

    private fun generate(alias: String, challenge: ByteArray, strongBox: Boolean) {
        val specBuilder = KeyGenParameterSpec.Builder(
            alias,
            KeyProperties.PURPOSE_SIGN or KeyProperties.PURPOSE_VERIFY,
        )
            .setAlgorithmParameterSpec(ECGenParameterSpec(CURVE_P256))
            .setDigests(KeyProperties.DIGEST_SHA256)
            .setAttestationChallenge(challenge)
            // User authentication is NOT required: shutter signing must not
            // block on biometrics; theft mitigation is key rotation (90 days).
            .setUserAuthenticationRequired(false)
        if (strongBox && Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            specBuilder.setIsStrongBoxBacked(true)
        }
        val generator = KeyPairGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_EC,
            ANDROID_KEYSTORE,
        )
        generator.initialize(specBuilder.build())
        // Return value discarded on purpose: private material never touched.
        generator.generateKeyPair()
    }

    private fun readHome(alias: String): KeyHome {
        val store = KeyStore.getInstance(ANDROID_KEYSTORE)
        store.load(null)
        val cert = store.getCertificate(alias)
        // Without cert details we report conservatively; Play Integrity
        // gating in RiseIntegrity decides the final tier anyway.
        return if (cert != null) KeyHome.TEE else KeyHome.SOFTWARE_FALLBACK
    }
}

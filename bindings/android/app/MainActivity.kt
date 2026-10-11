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
import rise.android.attest.RiseKeystore

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
                    try {
                        val path = signAndSeal(raw, hex)
                        status.text = "SEALED + SIGNED: ${raw.yuvBytes.size} bytes\nHASH=$hex"
                        Log.i("RISE", "MANIFEST_SAVED=$path")
                    } catch (e: Exception) {
                        status.text = "SIGNED FAIL: ${e.message}"
                        Log.e("RISE", "SIGN_ERROR: " + e.message, e)
                    }
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

    private fun signAndSeal(raw: RiseCapture.RawFrame, frameHex: String): String {
        try {
            return signAndSealInner(raw, frameHex)
        } catch (e: Exception) {
            Log.e("RISE", "SIGN_ERROR: " + e.message, e)
            throw e
        }
    }

    private fun signAndSealInner(raw: RiseCapture.RawFrame, frameHex: String): String {
        val alias = RiseKeystore.DEFAULT_ALIAS
        RiseKeystore.ensureP256Key(alias, ByteArray(32).also { java.security.SecureRandom().nextBytes(it) })
        val tiles = splitTiles(raw.yuvBytes).map { sha256Hex(it) }
        val root = merkleRoot(tiles)
        val deviceId = android.provider.Settings.Secure.getString(contentResolver, android.provider.Settings.Secure.ANDROID_ID) ?: "android-unknown"
        val canon = canonicalJson(frameHex, tiles, root, raw.timestampMs, deviceId, alias)
        val canonBytes = canon.toByteArray(Charsets.UTF_8)
        val sigDer = try {
            RiseKeystore.sign(alias, canonBytes)
        } catch (e: java.security.InvalidKeyException) {
            Log.w("RISE", "signing key invalidated, regenerating: ${e.message}")
            RiseKeystore.delete(alias)
            RiseKeystore.ensureP256Key(alias, ByteArray(32).also { java.security.SecureRandom().nextBytes(it) })
            RiseKeystore.sign(alias, canonBytes)
        } catch (e: java.security.UnrecoverableEntryException) {
            Log.w("RISE", "signing key unrecoverable, regenerating: ${e.message}")
            RiseKeystore.delete(alias)
            RiseKeystore.ensureP256Key(alias, ByteArray(32).also { java.security.SecureRandom().nextBytes(it) })
            RiseKeystore.sign(alias, canonBytes)
        }
        val sigHex = derToRawHex(sigDer)
        Log.i("RISE", "SIGNATURE=$sigHex")
        val manifest = canonicalJson(frameHex, tiles, root, raw.timestampMs, deviceId, alias, sigHex)
        val f = java.io.File(getExternalFilesDir(null), "frame-001.manifest.json")
        f.writeText(manifest, Charsets.UTF_8)
        return f.absolutePath
    }

    private fun canonicalJson(frameHex: String, tiles: List<String>, root: String, ts: Long, deviceId: String, alias: String, sigHex: String? = null): String {
        val frames = "[\"$frameHex\"]"
        val tileArr = tiles.joinToString(",", "[", "]") { "\"$it\"" }
        val sigPart = if (sigHex != null) ",\"signature\":\"$sigHex\"" else ""
        return "{\"rise_version\":1,\"frame_hashes\":$frames,\"tile_hashes\":$tileArr,\"merkle_root\":\"$root\",\"sig_alg\":\"ES256\",\"timestamp_ms\":$ts,\"device_id\":\"${jsonEscape(deviceId)}\",\"assurance\":\"silver\",\"timestamp_token\":null$sigPart,\"signing_key_id\":\"$alias\"}"
    }

    private fun splitTiles(data: ByteArray): List<ByteArray> {
        if (data.isEmpty()) return emptyList()
        val n = minOf(data.size, 4)
        val size = (data.size + n - 1) / n
        val out = ArrayList<ByteArray>()
        var off = 0
        while (off < data.size) {
            out.add(data.copyOfRange(off, minOf(off + size, data.size)))
            off += size
        }
        return out
    }

    private fun sha256Hex(data: ByteArray): String {
        val d = java.security.MessageDigest.getInstance("SHA-256").digest(data)
        val sb = StringBuilder(d.size * 2)
        for (b in d) sb.append(String.format("%02x", b))
        return sb.toString()
    }

    private fun merkleRoot(leaves: List<String>): String {
        if (leaves.isEmpty()) return sha256Hex(ByteArray(0))
        var level = leaves
        while (level.size > 1) {
            val next = ArrayList<String>(level.size / 2 + 1)
            var i = 0
            while (i < level.size) {
                val right = if (i + 1 < level.size) level[i + 1] else level[i]
                next.add(sha256Hex((level[i] + right).toByteArray(Charsets.UTF_8)))
                i += 2
            }
            level = next
        }
        return level[0]
    }

    private fun derToRawHex(der: ByteArray): String {
        require(der.size >= 8) { "ECDSA DER too short: ${der.size} bytes" }
        require(der[0] == 0x30.toByte()) { "ECDSA DER: expected SEQUENCE tag" }
        var p = 1 + lenSize(der, 1)
        require(p < der.size && der[p] == 0x02.toByte()) { "ECDSA DER: expected INTEGER r" }
        p += 1
        val rLen = readLen(der, p)
        p += lenSize(der, p)
        require(rLen > 0 && p + rLen <= der.size) { "ECDSA DER: r length $rLen overruns ${der.size} bytes" }
        val r = stripLeadingZero(der.copyOfRange(p, p + rLen))
        p += rLen
        require(p < der.size && der[p] == 0x02.toByte()) { "ECDSA DER: expected INTEGER s" }
        p += 1
        val sLen = readLen(der, p)
        p += lenSize(der, p)
        require(sLen > 0 && p + sLen <= der.size) { "ECDSA DER: s length $sLen overruns ${der.size} bytes" }
        val s = stripLeadingZero(der.copyOfRange(p, p + sLen))
        require(r.size <= 32 && s.size <= 32) { "ECDSA DER: coordinate oversize r=${r.size} s=${s.size}" }
        return r.joinToString("") { String.format("%02x", it) }.padStart(64, '0') +
            s.joinToString("") { String.format("%02x", it) }.padStart(64, '0')
    }

    private fun readLen(der: ByteArray, p: Int): Int {
        require(p < der.size) { "ECDSA DER: length offset $p out of bounds" }
        val b = der[p].toInt() and 0xFF
        if (b < 0x80) return b
        val n = b and 0x7F
        require(n in 1..4 && p + 1 + n <= der.size) { "ECDSA DER: bad long-form length at $p" }
        var v = 0
        for (i in 1..n) v = (v shl 8) or (der[p + i].toInt() and 0xFF)
        return v
    }

    private fun lenSize(der: ByteArray, p: Int): Int {
        require(p < der.size) { "ECDSA DER: length offset $p out of bounds" }
        val b = der[p].toInt() and 0xFF
        return if (b < 0x80) 1 else 1 + (b and 0x7F)
    }

    private fun stripLeadingZero(a: ByteArray): ByteArray {
        var i = 0
        while (i < a.size - 1 && a[i] == 0.toByte()) i++
        return a.copyOfRange(i, a.size)
    }

    private fun jsonEscape(s: String): String {
        val sb = StringBuilder(s.length)
        for (c in s) {
            when (c) {
                '"' -> sb.append("\\\"")
                '\\' -> sb.append("\\\\")
                '\n' -> sb.append("\\n")
                '\r' -> sb.append("\\r")
                '\t' -> sb.append("\\t")
                else -> if (c < ' ') sb.append(String.format("\\u%04x", c.code)) else sb.append(c)
            }
        }
        return sb.toString()
    }
}

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
                        try {
                            val risePath = packRise(raw)
                            status.text = "SEALED + SIGNED + PACKED: ${raw.yuvBytes.size} bytes\nHASH=$hex"
                            Log.i("RISE", "RISE_FILE_SAVED=$risePath")
                        } catch (e: Exception) {
                            Log.e("RISE", "PACK_ERROR: " + e.message, e)
                        }
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

    /** Packs the sealed manifest into a .rise file (JPEG + APP11 JUMBF). */
    private fun packRise(raw: RiseCapture.RawFrame): String {
        val mf = java.io.File(getExternalFilesDir(null), "frame-001.manifest.json")
        val manifestBytes = mf.readBytes()
        val jpg = frameToJpeg(raw)
        val sealed = jpegEmbed(jpg, manifestBytes)
        val out = java.io.File(getExternalFilesDir(null), "frame-001.rise")
        out.writeBytes(sealed)
        return out.absolutePath
    }

    /** Best-effort YUV -> JPEG via YuvImage; falls back to a minimal SOI+EOI JPEG. */
    private fun frameToJpeg(raw: RiseCapture.RawFrame): ByteArray {
        try {
            val yuv = android.graphics.YuvImage(
                raw.yuvBytes,
                android.graphics.ImageFormat.NV21,
                raw.width, raw.height, null,
            )
            val bos = java.io.ByteArrayOutputStream()
            if (yuv.compressToJpeg(android.graphics.Rect(0, 0, raw.width, raw.height), 90, bos)) {
                val b = bos.toByteArray()
                if (b.size >= 2 && b[0] == 0xFF.toByte() && b[1] == 0xD8.toByte()) return b
            }
        } catch (_: Exception) {
        }
        return byteArrayOf(0xFF.toByte(), 0xD8.toByte(), 0xFF.toByte(), 0xD9.toByte())
    }

    /**
     * Embeds the manifest JSON as an APP11 JUMBF segment right after the JPEG
     * SOI marker (mirrors core/src/jumbf.rs build_superbox + jpeg_embed):
     * APP11 = FF EB + len(2BE) + "JUMBF\0" + superbox, where
     * superbox = len(4BE) + "jumb" + descBox + jsonBox,
     * descBox = len(4BE) + "jumd" + C2PA_UUID(16) + toggles(0x00),
     * jsonBox = len(4BE) + "json" + manifest bytes.
     */
    private fun jpegEmbed(jpg: ByteArray, manifestJson: ByteArray): ByteArray {
        require(jpg.size >= 2 && jpg[0] == 0xFF.toByte() && jpg[1] == 0xD8.toByte()) {
            "not a JPEG: missing SOI"
        }
        val c2paUuid = byteArrayOf(
            0x63.toByte(), 0x32.toByte(), 0x70.toByte(), 0x61.toByte(),
            0x00, 0x11.toByte(), 0x00, 0x10.toByte(),
            0x80.toByte(), 0x00, 0x00, 0xAA.toByte(),
            0x00, 0x38.toByte(), 0x9B.toByte(), 0x71.toByte(),
        )
        // description box: len + "jumd" + UUID + toggles
        val descLen = 4 + 4 + 16 + 1
        val desc = java.io.ByteArrayOutputStream(descLen)
        desc.write(be32(descLen))
        desc.write("jumd".toByteArray(Charsets.US_ASCII))
        desc.write(c2paUuid)
        desc.write(0x00)
        val descBytes = desc.toByteArray()
        // json content box: len + "json" + payload
        val jsonLen = 4 + 4 + manifestJson.size
        val jsonBox = java.io.ByteArrayOutputStream(jsonLen)
        jsonBox.write(be32(jsonLen))
        jsonBox.write("json".toByteArray(Charsets.US_ASCII))
        jsonBox.write(manifestJson)
        val jsonBytes = jsonBox.toByteArray()
        // superbox: len + "jumb" + desc + json
        val superLen = 4 + 4 + descBytes.size + jsonBytes.size
        val sup = java.io.ByteArrayOutputStream(superLen)
        sup.write(be32(superLen))
        sup.write("jumb".toByteArray(Charsets.US_ASCII))
        sup.write(descBytes)
        sup.write(jsonBytes)
        val superbox = sup.toByteArray()
        // APP11 segment: FF EB + segLen(2BE, includes its own 2 bytes) + "JUMBF\0" + superbox
        val segLen = 2 + 6 + superbox.size
        require(segLen <= 0xFFFF) { "manifest too large for single APP11 segment: $segLen" }
        val out = java.io.ByteArrayOutputStream(2 + 2 + segLen - 2 + (jpg.size - 2))
        out.write(byteArrayOf(0xFF.toByte(), 0xD8.toByte()))
        out.write(byteArrayOf(0xFF.toByte(), 0xEB.toByte()))
        out.write(byteArrayOf(((segLen shr 8) and 0xFF).toByte(), (segLen and 0xFF).toByte()))
        out.write("JUMBF\u0000".toByteArray(Charsets.US_ASCII))
        out.write(superbox)
        out.write(jpg, 2, jpg.size - 2)
        return out.toByteArray()
    }

    private fun be32(n: Int): ByteArray {
        return byteArrayOf(
            ((n shr 24) and 0xFF).toByte(),
            ((n shr 16) and 0xFF).toByte(),
            ((n shr 8) and 0xFF).toByte(),
            (n and 0xFF).toByte(),
        )
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

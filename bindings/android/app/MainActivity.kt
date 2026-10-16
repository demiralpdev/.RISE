package rise.android.app

import android.app.Activity
import android.content.pm.PackageManager
import android.os.Bundle
import android.util.Log
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView
import com.rise.core.RiseCore
import rise.android.attest.RiseKeystore
import rise.android.capture.RiseCameraPermission
import rise.android.capture.RiseCapture
import java.io.ByteArrayOutputStream
import java.io.File
import java.security.SecureRandom

/**
 * RISE Android thin shell:
 * - Photo: Camera2 YUV -> JNI SHA-256 -> AndroidKeyStore EC P-256 sign -> APP11 JUMBF -> .rise
 * - Video: Camera2 + MediaRecorder MP4 -> MediaMetadataRetriever JPEG frames (<=256)
 *   -> JNI SHA-256 -> AndroidKeyStore EC P-256 sign -> ISO-BMFF uuid box -> .mp4.rvid
 */
class MainActivity : Activity() {
    private lateinit var status: TextView
    private lateinit var photoBtn: Button
    private lateinit var videoBtn: Button
    private lateinit var capture: RiseCapture
    private var pendingVideoRequest: Boolean = false

    companion object {
        private val RISE_MP4_UUID = byteArrayOf(
            0x72.toByte(), 0x69.toByte(), 0x73.toByte(), 0x65.toByte(),
            0x00, 0x00, 0x40.toByte(), 0x80.toByte(),
            0x00, 0x00, 0x00, 0x00,
            0x52.toByte(), 0x49.toByte(), 0x53.toByte(), 0x45.toByte(),
        )
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        capture = RiseCapture(this)
        val layout = LinearLayout(this)
        layout.orientation = LinearLayout.VERTICAL
        layout.setPadding(48, 48, 48, 48)

        val buttonRow = LinearLayout(this)
        buttonRow.orientation = LinearLayout.HORIZONTAL

        photoBtn = Button(this)
        photoBtn.text = "CAPTURE + SEAL"
        photoBtn.setOnClickListener {
            pendingVideoRequest = false
            if (RiseCameraPermission.isGranted(this)) {
                doCapture()
            } else {
                RiseCameraPermission.request(this)
            }
        }

        videoBtn = Button(this)
        videoBtn.text = "RECORD + SEAL VIDEO"
        videoBtn.setOnClickListener {
            if (capture.isRecordingVideo()) {
                capture.stopVideoRecording()
                return@setOnClickListener
            }
            pendingVideoRequest = true
            if (RiseCameraPermission.isGranted(this)) {
                doRecordAndSealVideo()
            } else {
                RiseCameraPermission.request(this)
            }
        }

        buttonRow.addView(photoBtn)
        buttonRow.addView(videoBtn)

        status = TextView(this)
        status.text = "RISE ready - tap to seal a frame or video"

        layout.addView(buttonRow)
        layout.addView(status)
        setContentView(layout)

        if (intent?.getBooleanExtra("record_video", false) == true) {
            pendingVideoRequest = true
            if (RiseCameraPermission.isGranted(this)) {
                doRecordAndSealVideo()
            } else {
                RiseCameraPermission.request(this)
            }
        }
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
            if (pendingVideoRequest) {
                doRecordAndSealVideo()
            } else {
                doCapture()
            }
        }
    }

    private fun doCapture() {
        status.text = "capturing..."
        capture.captureSingleFrame { result ->
            runOnUiThread {
                result.onSuccess { raw ->
                    val hex = RiseCore.hashFrame(raw.yuvBytes)
                    require(hex.length == 64) { "JNI frame hash length mismatch" }
                    Log.i("RISE", "HASH=$hex")
                    Log.i("RISE", "BYTES=${raw.yuvBytes.size} W=${raw.width} H=${raw.height} T=${raw.timestampMs}")
                    val f = File(getExternalFilesDir(null), "frame-001.yuv")
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

    private fun doRecordAndSealVideo() {
        val dir = getExternalFilesDir(null)
        if (dir == null) {
            status.text = "VIDEO FAIL: external files dir unavailable"
            Log.e("RISE", "RISE_VIDEO_ERROR: external files dir unavailable")
            return
        }
        val mp4File = File(dir, "video-001.mp4")
        val rvidFile = File(dir, "video-001.mp4.rvid")
        // Fail-closed: outputs of a previous run must never pair with this run.
        val discardOutputs = {
            if (rvidFile.exists()) rvidFile.delete()
            File(dir, "video-001.pub").delete()
            File(dir, "video-001.frames").deleteRecursively()
            File(dir, "video-001.manifest.json").delete()
            File(dir, "${rvidFile.name}.tmp").delete()
        }
        discardOutputs()
        status.text = "recording video (~3s)..."
        videoBtn.text = "STOP + SEAL VIDEO"
        Log.i("RISE", "RISE_VIDEO_START: out=${mp4File.absolutePath}")

        capture.recordVideo(mp4File, RiseCapture.DEFAULT_VIDEO_DURATION_MS) { result ->
            runOnUiThread {
                videoBtn.text = "RECORD + SEAL VIDEO"
            }
            result.onSuccess { rec ->
                Log.i(
                    "RISE",
                    "RISE_VIDEO_RECORDED: path=${rec.mp4File.absolutePath} bytes=${rec.mp4File.length()} W=${rec.width} H=${rec.height} T=${rec.timestampMs}",
                )
                runOnUiThread {
                    status.text = "extracting frames + sealing .rvid..."
                }
                Thread {
                    try {
                        val sealedOutcome = sealRecordedVideo(rec, rvidFile)
                        runOnUiThread {
                            status.text =
                                "VIDEO SEALED + SIGNED + PACKED\nFRAMES=${sealedOutcome.frameCount}\nROOT=${sealedOutcome.merkleRoot}\nRVID=${sealedOutcome.rvidPath}"
                        }
                    } catch (e: Throwable) {
                        // Fail-closed: never leave a partial .rvid (or its .pub/frames) on disk.
                        discardOutputs()
                        Log.e("RISE", "RISE_VIDEO_ERROR: ${e.message}", e)
                        runOnUiThread {
                            status.text = "VIDEO FAIL: ${e.message}"
                        }
                    }
                }.start()
            }.onFailure { e ->
                discardOutputs()
                Log.e("RISE", "RISE_VIDEO_ERROR: ${e.message}", e)
                runOnUiThread {
                    status.text = "VIDEO FAIL: ${e.message}"
                }
            }
        }
    }

    private data class VideoSealOutcome(
        val frameCount: Int,
        val merkleRoot: String,
        val manifestPath: String,
        val rvidPath: String,
    )

    private fun sealRecordedVideo(
        rec: RiseCapture.VideoRecording,
        rvidFile: File,
    ): VideoSealOutcome {
        val framesJpeg = RiseCapture.extractVideoFramesJpeg(
            rec.mp4File,
            RiseCapture.MAX_VIDEO_FRAMES,
        )
        Log.i("RISE", "RISE_VIDEO_FRAMES_EXTRACTED=${framesJpeg.size}")

        val frameHashes = RiseCore.hashFramesBatch(framesJpeg)
        require(frameHashes.size == framesJpeg.size && frameHashes.all { it.length == 64 }) {
            "JNI frame hash count/length mismatch"
        }
        Log.i("RISE", "RISE_VIDEO_FRAME_HASHES_COUNT=${frameHashes.size} FIRST=${frameHashes.first()}")

        // Persist the exact JPEG bytes that were hashed, in manifest order, so
        // frame_hashes can be re-checked off-device (frame-001.jpg == index 0).
        val framesDir = File(rec.mp4File.parentFile, "video-001.frames")
        framesDir.mkdirs()
        require(framesDir.isDirectory) { "cannot create frames dir ${framesDir.absolutePath}" }
        framesJpeg.forEachIndexed { i, bytes ->
            val f = File(framesDir, String.format("frame-%03d.jpg", i + 1))
            f.writeBytes(bytes)
            require(f.length() == bytes.size.toLong()) { "short write: ${f.name}" }
        }
        Log.i("RISE", "RISE_VIDEO_FRAMES_SAVED=${framesDir.absolutePath} COUNT=${framesJpeg.size}")

        // Tile hashes span ALL frames combined in extraction order (<=4 tiles per frame).
        val allTiles = ArrayList<ByteArray>(framesJpeg.size * 4)
        for (frameBytes in framesJpeg) {
            allTiles.addAll(splitTiles(frameBytes))
        }
        val tileHashes = RiseCore.hashFramesBatch(allTiles)
        require(tileHashes.size == allTiles.size && tileHashes.all { it.length == 64 }) {
            "JNI tile hash count/length mismatch"
        }
        val root = merkleRoot(tileHashes)
        Log.i("RISE", "RISE_VIDEO_TILES=${tileHashes.size} MERKLE_ROOT=$root")

        val alias = RiseKeystore.DEFAULT_ALIAS
        RiseKeystore.ensureP256Key(alias, ByteArray(32).also { SecureRandom().nextBytes(it) })
        val deviceId = android.provider.Settings.Secure.getString(
            contentResolver,
            android.provider.Settings.Secure.ANDROID_ID,
        ) ?: "android-unknown"

        // Seal-moment device wall clock, millisecond precision, no rounding.
        // The SAME value goes into the signed canonical and the final manifest.
        val sealTimestampMs = System.currentTimeMillis()
        Log.i("RISE", "RISE_VIDEO_SEAL_TS_MS=$sealTimestampMs")

        val canon = canonicalMultiFrameJson(
            frameHashes = frameHashes,
            tileHashes = tileHashes,
            root = root,
            ts = sealTimestampMs,
            deviceId = deviceId,
            alias = alias,
            sigHex = null,
        )
        val canonBytes = canon.toByteArray(Charsets.UTF_8)
        val sigDer = signWithKeystoreFallback(alias, canonBytes)
        val sigHex = derToRawHex(sigDer)
        Log.i("RISE", "RISE_VIDEO_SIGNATURE=$sigHex")

        // Public key is read AFTER signing: the fallback may have rotated the key.
        // Fail-closed: the exported key must verify the signature just produced.
        require(RiseKeystore.verifyWithPublicKey(alias, canonBytes, sigDer)) {
            "signature does not verify against the exported public key"
        }
        val pubPem = RiseKeystore.publicKeyPem(alias)
        Log.i("RISE", "RISE_VIDEO_PUBKEY_SPKI_PEM=" + pubPem.replace("\n", "\\n"))

        val manifestJson = canonicalMultiFrameJson(
            frameHashes = frameHashes,
            tileHashes = tileHashes,
            root = root,
            ts = sealTimestampMs,
            deviceId = deviceId,
            alias = alias,
            sigHex = sigHex,
        )
        val manifestBytes = manifestJson.toByteArray(Charsets.UTF_8)
        val manifestFile = File(rec.mp4File.parentFile, "video-001.manifest.json")
        manifestFile.writeBytes(manifestBytes)
        Log.i("RISE", "RISE_VIDEO_MANIFEST_SAVED=${manifestFile.absolutePath}")

        val pubFile = File(rec.mp4File.parentFile, "video-001.pub")
        pubFile.writeText(pubPem, Charsets.US_ASCII)
        require(pubFile.readText(Charsets.US_ASCII) == pubPem) { "failed to write ${pubFile.name}" }
        Log.i("RISE", "RISE_VIDEO_PUBKEY_SAVED=${pubFile.absolutePath}")

        val mp4Bytes = rec.mp4File.readBytes()
        val sealedBytes = mp4AppendBox(mp4Bytes, manifestBytes)

        // Write to a staging file first so a partial .rvid is never exposed.
        val tmpFile = File(rvidFile.parentFile, "${rvidFile.name}.tmp")
        if (tmpFile.exists()) {
            tmpFile.delete()
        }
        tmpFile.writeBytes(sealedBytes)
        if (!tmpFile.renameTo(rvidFile)) {
            rvidFile.writeBytes(sealedBytes)
            tmpFile.delete()
        }
        require(rvidFile.exists() && rvidFile.length() == sealedBytes.size.toLong()) {
            "failed to write complete .rvid file"
        }
        Log.i(
            "RISE",
            "RISE_VIDEO_RVID_SAVED=${rvidFile.absolutePath} BYTES=${rvidFile.length()} FRAMES=${frameHashes.size}",
        )
        return VideoSealOutcome(
            frameCount = frameHashes.size,
            merkleRoot = root,
            manifestPath = manifestFile.absolutePath,
            rvidPath = rvidFile.absolutePath,
        )
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
        RiseKeystore.ensureP256Key(alias, ByteArray(32).also { SecureRandom().nextBytes(it) })
        val tiles = splitTiles(raw.yuvBytes).map { sha256Hex(it) }
        val root = merkleRoot(tiles)
        val deviceId = android.provider.Settings.Secure.getString(
            contentResolver,
            android.provider.Settings.Secure.ANDROID_ID,
        ) ?: "android-unknown"
        val canon = canonicalJson(frameHex, tiles, root, raw.timestampMs, deviceId, alias)
        val canonBytes = canon.toByteArray(Charsets.UTF_8)
        val sigDer = signWithKeystoreFallback(alias, canonBytes)
        val sigHex = derToRawHex(sigDer)
        Log.i("RISE", "SIGNATURE=$sigHex")
        val manifest = canonicalJson(frameHex, tiles, root, raw.timestampMs, deviceId, alias, sigHex)
        val f = File(getExternalFilesDir(null), "frame-001.manifest.json")
        f.writeText(manifest, Charsets.UTF_8)
        return f.absolutePath
    }

    /**
     * Broad-exception signing retry for Xiaomi MIUI / KeyMint quirks where
     * invalidated or corrupted hardware keys throw ProviderException or
     * KeyStoreException variants instead of only InvalidKeyException.
     */
    private fun signWithKeystoreFallback(alias: String, canonBytes: ByteArray): ByteArray {
        return try {
            RiseKeystore.sign(alias, canonBytes)
        } catch (e: Exception) {
            Log.w("RISE", "signing key failed (${e.javaClass.simpleName}), regenerating: ${e.message}")
            RiseKeystore.delete(alias)
            RiseKeystore.ensureP256Key(alias, ByteArray(32).also { SecureRandom().nextBytes(it) })
            RiseKeystore.sign(alias, canonBytes)
        }
    }

    /** Packs the sealed manifest into a .rise file (JPEG + APP11 JUMBF). */
    private fun packRise(raw: RiseCapture.RawFrame): String {
        val mf = File(getExternalFilesDir(null), "frame-001.manifest.json")
        val manifestBytes = mf.readBytes()
        val jpg = frameToJpeg(raw)
        val sealed = jpegEmbed(jpg, manifestBytes)
        val out = File(getExternalFilesDir(null), "frame-001.rise")
        out.writeBytes(sealed)
        return out.absolutePath
    }

    /** YUV -> JPEG via YuvImage; fail-closed: throws if compression fails. */
    private fun frameToJpeg(raw: RiseCapture.RawFrame): ByteArray {
        try {
            val yuv = android.graphics.YuvImage(
                raw.yuvBytes,
                android.graphics.ImageFormat.NV21,
                raw.width, raw.height, null,
            )
            val bos = ByteArrayOutputStream()
            if (yuv.compressToJpeg(android.graphics.Rect(0, 0, raw.width, raw.height), 90, bos)) {
                val b = bos.toByteArray()
                if (b.size >= 2 && b[0] == 0xFF.toByte() && b[1] == 0xD8.toByte()) return b
            }
        } catch (_: Exception) {
        }
        throw IllegalStateException("YUV to JPEG compression failed")
    }

    /**
     * Appends a top-level ISO-BMFF `uuid` box carrying the RISE UUID and raw
     * UTF-8 manifest JSON to an MP4 byte stream (mirrors container.rs::mp4_append_box).
     */
    private fun mp4AppendBox(mp4: ByteArray, manifestJson: ByteArray): ByteArray {
        require(
            mp4.size >= 8 &&
                mp4[4] == 'f'.code.toByte() &&
                mp4[5] == 't'.code.toByte() &&
                mp4[6] == 'y'.code.toByte() &&
                mp4[7] == 'p'.code.toByte(),
        ) {
            "not an MP4 (no ftyp box)"
        }
        val payloadLen = manifestJson.size.toLong() + RISE_MP4_UUID.size.toLong()
        require(payloadLen + 8L <= 0xFFFF_FFFFL) {
            "manifest too large for a single box"
        }
        val boxLen = (payloadLen + 8L).toInt()
        val out = ByteArrayOutputStream(mp4.size + boxLen)
        out.write(mp4)
        out.write(be32(boxLen))
        out.write("uuid".toByteArray(Charsets.US_ASCII))
        out.write(RISE_MP4_UUID)
        out.write(manifestJson)
        return out.toByteArray()
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
        val desc = ByteArrayOutputStream(descLen)
        desc.write(be32(descLen))
        desc.write("jumd".toByteArray(Charsets.US_ASCII))
        desc.write(c2paUuid)
        desc.write(0x00)
        val descBytes = desc.toByteArray()
        // json content box: len + "json" + payload
        val jsonLen = 4 + 4 + manifestJson.size
        val jsonBox = ByteArrayOutputStream(jsonLen)
        jsonBox.write(be32(jsonLen))
        jsonBox.write("json".toByteArray(Charsets.US_ASCII))
        jsonBox.write(manifestJson)
        val jsonBytes = jsonBox.toByteArray()
        // superbox: len + "jumb" + desc + json
        val superLen = 4 + 4 + descBytes.size + jsonBytes.size
        val sup = ByteArrayOutputStream(superLen)
        sup.write(be32(superLen))
        sup.write("jumb".toByteArray(Charsets.US_ASCII))
        sup.write(descBytes)
        sup.write(jsonBytes)
        val superbox = sup.toByteArray()
        // APP11 segment: FF EB + segLen(2BE, includes its own 2 bytes) + "JUMBF\0" + superbox
        val segLen = 2 + 6 + superbox.size
        require(segLen <= 0xFFFF) { "manifest too large for single APP11 segment: $segLen" }
        val out = ByteArrayOutputStream(2 + 2 + segLen - 2 + (jpg.size - 2))
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

    private fun canonicalJson(
        frameHex: String,
        tiles: List<String>,
        root: String,
        ts: Long,
        deviceId: String,
        alias: String,
        sigHex: String? = null,
    ): String = canonicalMultiFrameJson(
        frameHashes = listOf(frameHex),
        tileHashes = tiles,
        root = root,
        ts = ts,
        deviceId = deviceId,
        alias = alias,
        sigHex = sigHex,
    )

    /**
     * Emits ManifestV1 JSON in the exact field order of Rust serde (core/src/lib.rs):
     * rise_version, frame_hashes, tile_hashes, merkle_root, sig_alg, timestamp_ms,
     * device_id, assurance, timestamp_token, then optional signature and signing_key_id.
     */
    private fun canonicalMultiFrameJson(
        frameHashes: List<String>,
        tileHashes: List<String>,
        root: String,
        ts: Long,
        deviceId: String,
        alias: String?,
        sigHex: String? = null,
    ): String {
        val framesArr = frameHashes.joinToString(",", "[", "]") { "\"$it\"" }
        val tileArr = tileHashes.joinToString(",", "[", "]") { "\"$it\"" }
        val sigPart = if (sigHex != null) ",\"signature\":\"$sigHex\"" else ""
        val keyPart = if (alias != null) ",\"signing_key_id\":\"${jsonEscape(alias)}\"" else ""
        return "{\"rise_version\":1,\"frame_hashes\":$framesArr,\"tile_hashes\":$tileArr,\"merkle_root\":\"$root\",\"sig_alg\":\"ES256\",\"timestamp_ms\":$ts,\"device_id\":\"${jsonEscape(deviceId)}\",\"assurance\":\"silver\",\"timestamp_token\":null$sigPart$keyPart}"
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
        val seqLen = readLen(der, 1)
        var p = 1 + lenSize(der, 1)
        require(p + seqLen == der.size) { "ECDSA DER: trailing bytes after SEQUENCE" }
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
        require(p + sLen == der.size) { "ECDSA DER: trailing bytes after s" }
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

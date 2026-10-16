package com.rise.core

/** JNI bridge holder — one source of truth with rise_jni.c. */
object RiseCore {
    init {
        System.loadLibrary("rise_jni")
    }

    @JvmStatic
    external fun hashFrame(frame: ByteArray): String

    @JvmStatic
    external fun hashFrames(frames: Array<ByteArray>): Array<String>?

    /**
     * Batch-hashes multiple frames or tiles through librise_jni.so.
     * Uses the batch JNI entry when exported, falling back to per-frame
     * [hashFrame] calls if running against an older prebuilt librise_jni.so.
     */
    @JvmStatic
    fun hashFramesBatch(frames: List<ByteArray>): List<String> {
        if (frames.isEmpty()) return emptyList()
        return try {
            val res = hashFrames(frames.toTypedArray())
            if (res != null && res.size == frames.size) {
                res.forEachIndexed { i, it ->
                    require(it.length == 64) { "JNI batch hash length mismatch at index $i" }
                }
                res.toList()
            } else {
                frames.mapIndexed { i, frame ->
                    hashFrame(frame).also {
                        require(it.length == 64) { "JNI batch hash length mismatch at index $i" }
                    }
                }
            }
        } catch (_: UnsatisfiedLinkError) {
            frames.map { hashFrame(it) }
        }
    }
}

package com.rise.core

/** JNI bridge holder — one source of truth with rise_jni.c. */
object RiseCore {
    init {
        System.loadLibrary("rise_jni")
    }

    @JvmStatic
    external fun hashFrame(frame: ByteArray): String
}

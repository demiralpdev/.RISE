/* RISE Android JNI bridge — thin shell over core rise_hash_frame.
 *
 * STUB HONESTY HEADER — NOT BUILT, NOT RUN.
 * - Written against bindings/macos/capture/core.h
 *   (void rise_hash_frame(const unsigned char *, size_t, char *out_hex65))
 *   and core/src/lib.rs rise_hash_frame. Signature verified by reading
 *   both files on 2026-10-10; host syntax-only check passed (see below),
 *   no NDK / Android-ABI compile has run.
 * - Host syntax check 2026-10-10: `cc -fsyntax-only` with JDK jni.h
 *   (macOS darwin headers) exits 0, confirming the relative
 *   `../../macos/capture/core.h` include resolves and the JNI types check.
 *   This is NOT an NDK build and NOT an Android-ABI compile.
 * - Thin-shell rule: this file only moves bytes across JNI and calls core.
 *   No signing logic, no key handling, no key material logging.
 */

#include <jni.h>
#include <stddef.h>
#include <string.h>

/* Shared FFI contract. Relative include keeps one source of truth. */
#include "../../macos/capture/core.h"

/*
 * Class:     com_rise_core_RiseCore
 * Method:    hashFrame
 * Signature: ([B)Ljava/lang/String;
 *
 * Java side (Kotlin):
 *   package com.rise.core
 *   object RiseCore {
 *       external fun hashFrame(frame: ByteArray): String
 *   }
 *
 * Returns the 64-char lowercase hex SHA256 of the frame bytes,
 * or null if input is null / JNI fails. Never logs frame bytes.
 */
JNIEXPORT jstring JNICALL
Java_com_rise_core_RiseCore_hashFrame(JNIEnv *env, jclass clazz, jbyteArray data) {
    (void)clazz;

    if (env == NULL || data == NULL) {
        return NULL;
    }

    jsize len = (*env)->GetArrayLength(env, data);
    if (len < 0) {
        return NULL;
    }

    /* Empty input is valid: core hashes b"" to the known empty SHA256. */
    jbyte *bytes = NULL;
    if (len > 0) {
        bytes = (*env)->GetByteArrayElements(env, data, NULL);
        if (bytes == NULL) {
            /* OutOfMemoryError already pending. */
            return NULL;
        }
    }

    char out_hex65[65];
    memset(out_hex65, 0, sizeof(out_hex65));

    rise_hash_frame(
        (const unsigned char *)bytes,
        (size_t)len,
        out_hex65);

    if (len > 0) {
        /* JNI_ABORT: do not copy back, input array is read-only here. */
        (*env)->ReleaseByteArrayElements(env, data, bytes, JNI_ABORT);
        bytes = NULL;
    }

    /* Guarantee NUL termination even if core misbehaved. */
    out_hex65[64] = '\0';

    return (*env)->NewStringUTF(env, out_hex65);
}

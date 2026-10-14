/* RISE Android JNI bridge — thin shell over core rise_hash_frame.
 *
 * Status: REAL - NDK r27 aarch64-v8a build + on-device proof (2026-10-10):
 * - cmake (Unix Makefiles, android.toolchain.cmake) built librise_jni.so,
 *   statically linked against librise_core.a (cargo-ndk release build).
 * - llvm-nm -D: Java_com_rise_core_RiseCore_hashFrame exported (T).
 * - On-device (emugy5nbrsmrmzmz, /data/local/tmp): dlopen + dlsym resolved
 *   the symbol (dlopen_smoke.c -> JNI_SYMBOL_OK).
 * - Calling the function still needs a JNIEnv (app process); an APK-context
 *   System.loadLibrary test remains future work.
 * - Written against bindings/macos/capture/core.h
 *   (void rise_hash_frame(const unsigned char *, size_t, char *out_hex65))
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

    /* Empty input is valid: core hashes b"" to the known empty SHA256.
     * Never pass NULL with len 0: core's null-pointer guard would yield ""
     * instead of e3b0c442..., so hand it a real (unread) byte. */
    static const unsigned char empty = 0;
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
        len > 0 ? (const unsigned char *)bytes : &empty,
        (size_t)len,
        out_hex65);

    if (len > 0) {
        /* JNI_ABORT: do not copy back, input array is read-only here. */
        (*env)->ReleaseByteArrayElements(env, data, bytes, JNI_ABORT);
        bytes = NULL;
    }

    /* Fail-closed: an empty digest means core rejected the input; surface it
     * as NULL so the Kotlin require() guards catch it at runtime. */
    if (out_hex65[0] == '\0') {
        return NULL;
    }

    /* Guarantee NUL termination even if core misbehaved. */
    out_hex65[64] = '\0';

    return (*env)->NewStringUTF(env, out_hex65);
}

/*
 * Class:     com_rise_core_RiseCore
 * Method:    hashFrames
 * Signature: ([[B)[Ljava/lang/String;
 *
 * Batch-friendly JNI entry for video frame / tile hashing: hashes an array of
 * byte arrays in a single JNI call via rise_hash_frame. Fail-closed: returns
 * NULL if any element is NULL or JNI allocation fails.
 */
JNIEXPORT jobjectArray JNICALL
Java_com_rise_core_RiseCore_hashFrames(JNIEnv *env, jclass clazz, jobjectArray frames) {
    (void)clazz;

    if (env == NULL || frames == NULL) {
        return NULL;
    }

    jsize count = (*env)->GetArrayLength(env, frames);
    if (count < 0) {
        return NULL;
    }

    jclass stringClass = (*env)->FindClass(env, "java/lang/String");
    if (stringClass == NULL) {
        return NULL;
    }

    jobjectArray out = (*env)->NewObjectArray(env, count, stringClass, NULL);
    if (out == NULL) {
        return NULL;
    }
    (*env)->DeleteLocalRef(env, stringClass);

    for (jsize i = 0; i < count; i++) {
        jbyteArray item = (jbyteArray)(*env)->GetObjectArrayElement(env, frames, i);
        if (item == NULL) {
            (*env)->DeleteLocalRef(env, out);
            return NULL;
        }
        jstring hex = Java_com_rise_core_RiseCore_hashFrame(env, clazz, item);
        (*env)->DeleteLocalRef(env, item);
        if (hex == NULL) {
            (*env)->DeleteLocalRef(env, out);
            return NULL;
        }
        (*env)->SetObjectArrayElement(env, out, i, hex);
        (*env)->DeleteLocalRef(env, hex);
    }

    return out;
}

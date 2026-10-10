# RISE Android JNI bridge — CMake build for librise_jni.so
#
# STUB HONESTY HEADER — NOT CONFIGURED, NOT BUILT.
# No Android NDK on this machine (ANDROID_NDK_HOME unset), so
# `cmake --toolchain android.toolchain.cmake` has never run here.
# This file is review-only until an NDK host proves it.

cmake_minimum_required(VERSION 3.22)

project(rise_jni C)

# Shared JNI wrapper. Thin shell only: moves bytes, calls core.
add_library(rise_jni SHARED rise_jni.c)

# One source of truth for the FFI signature.
target_include_directories(rise_jni PRIVATE
    ../../macos/capture
)

# The Rust core must be prebuilt per ABI with cargo-ndk, e.g.:
#   cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -- build --release
# and the resulting librise_core.a placed per-ABI under
# bindings/android/jni/libs/<ABI>/ (see README.md).
# Each ABI build links its own static lib; CMake selects by ANDROID_ABI.
target_link_directories(rise_jni PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/libs/${ANDROID_ABI}
)

target_link_libraries(rise_jni PRIVATE rise_core log)

# Fail fast on a missing prebuilt instead of linking host garbage.
# CMake errors here if libs/<ABI>/librise_core.a was not staged.
if(NOT EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/libs/${ANDROID_ABI}/librise_core.a")
    message(WARNING
        "librise_core.a missing for ABI '${ANDROID_ABI}': "
        "expected libs/${ANDROID_ABI}/librise_core.a "
        "(build core with cargo-ndk first, see README.md)")
endif()

set_target_properties(rise_jni PROPERTIES
    C_STANDARD 11
    C_STANDARD_REQUIRED ON
)

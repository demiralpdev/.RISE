#!/bin/sh
# Headless minimal APK build proving System.loadLibrary("rise_jni") works in
# a real Android app process. kotlinc -> d8 -> aapt2 -> zip -> zipalign -> apksigner.
# No gradle, no Android Studio. Requires: NDK-built librise_jni.so (see ../jni/).
set -e
SDK=${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}
BT=$SDK/build-tools/35.0.0
PLATFORM=$SDK/platforms/android-35/android.jar
KOTLINC=${KOTLINC:-/tmp/kotlinc/bin/kotlinc}
JAVA_HOME=${JAVA_HOME:-"$(brew --prefix openjdk@21)/libexec/openjdk.jdk/Contents/Home"}
export JAVA_HOME
cd "$(dirname "$0")"
rm -rf build
mkdir -p build/classes build/dex build/stage/lib/arm64-v8a

echo B1-kotlinc
"$KOTLINC" -cp "$PLATFORM" MainActivity.kt com/rise/core/RiseCore.kt -d build/classes 1>&2
find build/classes -name '*.class' > build/classlist.txt

echo B2-d8
# Kotlin stdlib must ride inside the APK, or runtime crashes with
# NoClassDefFoundError (e.g. kotlin/text/Charsets).
KOTLIN_STDLIB=${KOTLIN_STDLIB:-/tmp/kotlinc/lib/kotlin-stdlib.jar}
"$BT/d8" --release --min-api 24 --lib "$PLATFORM" --output build/dex \
  @build/classlist.txt "$KOTLIN_STDLIB" 1>&2

echo B3-aapt2
"$BT/aapt2" link -I "$PLATFORM" --manifest AndroidManifest.xml \
  --min-sdk-version 24 --target-sdk-version 35 -o build/app-unsigned.apk 1>&2

echo B4-package
cp ../jni/build/arm64-v8a/librise_jni.so build/stage/lib/arm64-v8a/
# Kotlin stdlib pushes past 64K methods: d8 emits classes.dex + classes2.dex,
# native multidex (API 21+) loads every classes*.dex from the APK root.
(cd build/dex && zip -q -u ../app-unsigned.apk *.dex)
(cd build/stage && zip -q -u ../app-unsigned.apk lib/arm64-v8a/librise_jni.so)

echo B5-align-sign
"$BT/zipalign" -f 4 build/app-unsigned.apk build/app-aligned.apk
if [ ! -f build/debug.keystore ]; then
  "$JAVA_HOME/bin/keytool" -genkeypair -keystore build/debug.keystore -alias rise \
    -keyalg RSA -keysize 2048 -validity 10000 -storepass android -keypass android \
    -dname "CN=RISE Debug" 1>&2
fi
"$BT/apksigner" sign --ks build/debug.keystore --ks-pass pass:android \
  --key-pass pass:android --out build/app.apk build/app-aligned.apk

echo APP_BUILT: build/app.apk

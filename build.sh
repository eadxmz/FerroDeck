#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

SDK="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
NDK="${ANDROID_NDK_HOME:-$(ls -d "$SDK"/ndk/* 2>/dev/null | tail -n 1)}"
BT="$SDK/build-tools/34.0.0"
JAR="$SDK/platforms/android-34/android.jar"

mkdir -p build/gen build/classes build/dex build/apk/lib/arm64-v8a build/apk/lib/armeabi-v7a

# 1. Build Rust native libraries (arm64 + armv7)
cargo ndk -t arm64-v8a -t armeabi-v7a -o build/apk/lib build --release

# 2. Compile minimal resources & assets (no XML layouts, no AndroidX/Leanback bloat)
"$BT/aapt2" compile --dir res -o build/res.zip
"$BT/aapt2" link -o build/unsigned.apk -I "$JAR" --manifest AndroidManifest.xml \
    -A assets --java build/gen --min-sdk-version 24 --target-sdk-version 34 \
    --version-code 1 --version-name "0.1.0" build/res.zip

# 3. Compile thin NativeActivity + RemoteFilterService Java bridge
javac -encoding UTF-8 --release 11 -classpath "$JAR" \
    -d build/classes $(find java build/gen -name '*.java')
"$BT/d8" --release --min-api 24 --lib "$JAR" --output build/dex $(find build/classes -name '*.class')

cp build/dex/classes.dex build/apk/
unzip -q -o build/unsigned.apk -d build/apk
(cd build/apk && zip -q -r ../raw.apk .)

"$BT/zipalign" -f 4 build/raw.apk build/aligned.apk

KS=debug.keystore
if [ ! -f "$KS" ]; then
    keytool -genkeypair -keystore "$KS" -storepass android -keypass android -alias debug \
        -keyalg RSA -keysize 2048 -validity 10000 -dname "CN=FerroDeck debug" >/dev/null
fi
"$BT/apksigner" sign --ks "$KS" --ks-pass pass:android --key-pass pass:android \
    --out build/ferrodeck.apk build/aligned.apk

echo "Built build/ferrodeck.apk"

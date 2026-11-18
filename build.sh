#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

SDK="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
BT="$SDK/build-tools/34.0.0"
JAR="$SDK/platforms/android-34/android.jar"

mkdir -p assets build/gen build/classes build/dex build/apk/lib/arm64-v8a build/apk/lib/armeabi-v7a

# 1. Build Rust native libraries (arm64-v8a + armeabi-v7a) targeting Android API 24+
cargo ndk --platform 24 -t arm64-v8a -t armeabi-v7a -o build/apk/lib build --release

# 2. Compile minimal resources & assets (keeps resources.arsc uncompressed for targetSdk 34)
"$BT/aapt2" compile --dir res -o build/res.zip
"$BT/aapt2" link -o build/unsigned.apk -I "$JAR" --manifest AndroidManifest.xml \
    -A assets --java build/gen --min-sdk-version 24 --target-sdk-version 34 \
    --version-code 2 --version-name "0.2.0" build/res.zip

# 3. Compile thin NativeActivity + RemoteFilterService Java bridge
javac -encoding UTF-8 --release 11 -classpath "$JAR" \
    -d build/classes $(find java build/gen -name '*.java')
"$BT/d8" --release --min-api 24 --lib "$JAR" --output build/dex $(find build/classes -name '*.class')

# 4. Pack classes.dex and native .so libs directly into unsigned.apk without re-zipping resources.arsc
(cd build/dex && zip -q ../unsigned.apk classes.dex)
(cd build/apk && zip -q -r ../unsigned.apk lib)

# 5. Page-align native libs (-p) and 4-byte align resources.arsc
"$BT/zipalign" -p -f 4 build/unsigned.apk build/aligned.apk

KS=debug.keystore
if [ ! -f "$KS" ]; then
    keytool -genkeypair -keystore "$KS" -storepass android -keypass android -alias debug \
        -keyalg RSA -keysize 2048 -validity 10000 -dname "CN=FerroDeck debug" >/dev/null
fi
"$BT/apksigner" sign --ks "$KS" --ks-pass pass:android --key-pass pass:android \
    --out build/ferrodeck.apk build/aligned.apk

echo "Built build/ferrodeck.apk ($(du -h build/ferrodeck.apk | cut -f1))"

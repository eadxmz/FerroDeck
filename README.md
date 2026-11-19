# FerroDeck

![Rust](https://img.shields.io/badge/Rust-NativeActivity-orange?logo=rust)
![Platform](https://img.shields.io/badge/Platform-Android%20TV-3DDC84?logo=android)
![Renderer](https://img.shields.io/badge/Renderer-tiny--skia%200.12-blue)
![Architecture](https://img.shields.io/badge/ABI-arm64--v8a%20%7C%20armeabi--v7a-lightgrey)

An ultra-lightweight, high-performance Android TV launcher written natively in **Rust**. Built without Android XML layouts, Jetpack Compose, or Leanback UI overhead, FerroDeck pairs a **Liquid Glass** aesthetic with a 60fps delta-time rendering engine that drops straight to **0.0% idle CPU**.

---

## Highlights

* **Zero-Bloat Native Rust UI:** Powered by `android-activity` (`NativeActivity`), `tiny-skia`, and `fontdue` with a thin JNI bridge solely for Android system APIs.
* **Liquid Glass & Night Scenes:** Rotating scenic backdrops with foreground mountain cutouts overlapping a crisp 1:1 glass clock, frosted specular cards, and a floating squircle dock (plus a streamlined **Classic** mode).
* **Locked 60fps & 0% Idle CPU:** Pre-composited 1:1 scene and blurred glass caches, cached glyph bitmaps, and a hardware-scaled 720p software buffer (upscaled to 1080p/4K by Android's GPU Hardware Composer at 0% CPU cost).
* **Inbuilt Low-Footprint Voice Assistant:** On-demand `SpeechRecognizer` and natural neural `TextToSpeech` initialized strictly on button press and destroyed immediately afterward:
  * Launches apps by both real and **custom renamed names** (e.g., rename TizenTube to `YouTube` and say *"Open YouTube"*).
  * Context-aware **in-app search** over YouTube, TizenTube, SmartTube, Koji TV Browser, and Chrome (e.g., say *"MrBeast"* while inside YouTube).
  * Silent for navigation and launch actions; evaluates math locally and runs background web lookups to answer questions with a natural voice.
  * Liquid Glass top-bar HUD — stop or cancel anytime with **`Back`**, the **Voice button**, or a tap.
* **Control Center & Memory Switcher:** Right-aligned frosted stadium-pill grid (`Wi-Fi`, `Bluetooth`, `Memory`, `Do Not Disturb`, `Audio`, `Sleep Timer`, `Ambient Mode`, `Power Off`) + 4 quick-action squircles (`Inputs`, `Button Mapper`, `Home Settings`, `Search`), rendered with 22 custom anti-aliased vector glyphs.
* **Interactive Background Apps Manager:** Open **Control Center → Memory** to inspect live RAM usage and running/recent app cards — press **`Up`** to kill an individual app or **`Down`** to clear all background processes.
* **Smart Dock & Long-Press Menu:** Automatically excludes iconless/default-robot apps and OS plumbing while keeping dedicated-icon apps (including Gallery, Clock, etc.). Hold **`OK`** (`≥ 480 ms`) on any dock app for a compact Liquid Glass sheet: **Open**, **Rename**, **Move Left/Right**, **Hide App**, **App Details**, and **Disable** (built-in) or **Uninstall** (user apps).
* **Native In-Launcher Search:** Blurred search overlay with a one-row D-pad keyboard, hardware keyboard typing, live app filtering (respecting custom names), and one-click in-app search shortcuts.
* **Hardware Remote Keybind Mapper:** System-wide `AccessibilityService` (`RemoteFilterService`) captures OEM `scanCode`-only buttons (such as the yellow mic button) with a Liquid Glass key-capture modal for **Voice Assistant** and **Control Center**.
* **6-Step First-Time Setup Wizard:** Guided onboarding for **Native (India)** vs. **Global** languages (keeping English under Native India), custom profile name & avatar badge, live **Automatic** (IP/Open-Meteo) or **Manual City** weather, appearance, keybinds, and default launcher/ADB setup.

---

## Controls

| Action | Remote / Keyboard |
|---|---|
| Navigate Dock / Menus | `D-Pad Up / Down / Left / Right` |
| Open App / Select Option | `OK` / `Enter` |
| App Options Menu (Rename, Move, Hide, Disable/Uninstall) | **Hold `OK`** (`≥ 480 ms`) on a dock app |
| Open Control Center | `Menu` button (or top-right Sliders pill / custom keybind) |
| Open Home Settings | `Settings` button (or Control Center → Gear icon) |
| Open Android Device Settings | Dock → **Settings** gear tile |
| Trigger / Stop Voice Assistant | `Yellow Mic` / `Voice Assist` / custom keybind (`Back` to cancel) |
| Close App in Memory Switcher | `D-Pad Up` (or `D-Pad Down` to clean all) |

---

## Architecture

| Component | Path | Responsibility |
|---|---|---|
| Core Event Loop & JNI | `src/lib.rs` | `ALooper` event loop, 720p HWC buffer management, JNI bridge |
| Renderer & Overlays | `src/ui.rs` | Home screen, Dock, Control Center, Search, App Menu, Key Capture & Voice HUD |
| Animation Engine | `src/anim.rs` | Frame-rate-independent exponential easing & alpha helpers |
| Text & Glass Clock | `src/clock.rs` | 1:1 `CrispClock`, `GLYPH_CACHE`, and Android `NotoSans` script fallbacks |
| Vector Glyphs | `src/glyphs.rs` | 22 native `tiny-skia` anti-aliased UI icons |
| Home Settings | `src/settings_pill.rs` | Two-pane settings screen with inline section headers & gliding pills |
| Setup Wizard | `src/setup_wizard.rs` | 6-step onboarding flow with persistent step state |
| Key Mapper | `src/input.rs` | `keyCode` + `scanCode` bit-packed bindings & friendly key names |
| Android Host Bridge | `java/.../MainActivity.java` | App discovery, icon extraction, live weather, Liquid Glass input modals |
| Voice Assistant | `java/.../VoiceAssistant.java` | On-demand `SpeechRecognizer`, in-app search dispatch, math/Q&A & neural TTS |
| Remote Key Filter | `java/.../RemoteFilterService.java` | Global hardware key capture & foreground package tracking |

---

## Installation & Optional ADB Permissions

1. Download the latest signed **`ferrodeck.apk`** from **GitHub Actions → Artifacts** (or Releases).
2. Install via ADB or sideload on your Android TV device:
   ```bash
   adb install -r ferrodeck.apk
   ```
3. *(Optional)* Grant elevated permissions for one-tap ADB/accessibility management and hands-free voice:
   ```bash
   adb shell pm grant com.eadxmz.ferrodeck android.permission.WRITE_SECURE_SETTINGS
   adb shell pm grant com.eadxmz.ferrodeck android.permission.RECORD_AUDIO
   ```
4. Enable **FerroDeck – Full Remote & Voice Assistant** under **Android Settings → Accessibility** to allow global Voice Assistant and in-app search over YouTube, TizenTube, and browsers.

---

## Building from Source

FerroDeck builds automatically via **GitHub Actions** on every push to `main` using `cargo-ndk`, `aapt2`, `javac` (Java 11), `d8`, `zipalign`, and `apksigner`.

To build locally, ensure you have the Rust Android targets (`aarch64-linux-android`, `armv7-linux-androideabi`), `cargo-ndk`, Android SDK Build-Tools, and Android NDK installed, then run:

```bash
./build.sh
```

The signed APK is output to `build/ferrodeck.apk`.
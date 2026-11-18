# FerroDeck (`ferrohome`)

Ultra-lightweight **Liquid Glass Android TV Launcher** rewritten in **Rust** (`android-activity` + `tiny-skia` + `fontdue`) from the original Java tvOS Launcher.

### Key Upgrades over the Java Build
- **Zero UI Framework Bloat:** Stripped 40+ bulky Java UI classes, XML layouts, and multi-pass View drawables — replaced by a native Rust `ANativeWindow` renderer.
- **Crisp 1:1 Unblurred Clock:** Renders glyphs via `fontdue` directly at native physical framebuffer coordinates (`1:1` pixel geometry, no `sp`/`dp` bilinear scaling blur).
- **Cleaner Rounded-Pill Settings Panel:** In-surface `tiny-skia` frosted modal with true stadium-pill rows (`corner_radius = height / 2`) and smooth D-pad highlight interpolation.
- **Full Remote Button & Yellow Mic Support:** Captures raw `(scanCode, keyCode)` pairs in both `AInputQueue` and `RemoteFilterService` (including `KEYCODE_UNKNOWN` with hardware `scanCode > 0` and `VOICE_ASSIST` / `ASSIST` intents).
- **Koji Browser Home-Button Focus Fix:** Explicitly exits Android Touch Mode and resets native Rust selection state on `onNewIntent` / `onWindowFocusChanged`, so D-pad arrows respond immediately without pressing `Back` first.
- **0.0% Idle CPU:** Event-driven `ALooper` sleeps until a key press, focus event, or next minute clock tick.

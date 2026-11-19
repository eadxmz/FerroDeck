//! FerroDeck — Event-driven Rust NativeActivity TV Launcher (`android-activity` 0.6).

mod anim;
mod clock;
mod glyphs;
mod input;
mod settings_pill;
mod setup_wizard;
mod ui;

use crate::input::{
    BoundAction, CaptureTarget, RemoteMapper, AKEYCODE_BACK, AKEYCODE_DEL, AKEYCODE_DPAD_CENTER,
    AKEYCODE_DPAD_DOWN, AKEYCODE_DPAD_LEFT, AKEYCODE_DPAD_RIGHT, AKEYCODE_DPAD_UP, AKEYCODE_ENTER,
    AKEYCODE_HOME,
};
use crate::settings_pill::SettingsCategory;
use crate::ui::{
    AppTile, FocusZone, LauncherUi, SideEffect, SEARCH_KEYS, SETTINGS_TILE_PKG,
};
use android_activity::input::{InputEvent, KeyAction, MotionAction};
use android_activity::{AndroidApp, AndroidAppWaker, InputStatus, MainEvent, PollEvent};
use jni::objects::{JBooleanArray, JByteArray, JClass, JObject, JObjectArray, JString, JValue};
use jni::sys::{jboolean, jint};
use jni::JNIEnv;
use ndk::native_window::NativeWindow;
use once_cell::sync::Lazy;
use std::ffi::CString;
use std::io::Read;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tiny_skia::{IntSize, Pixmap};

#[allow(dead_code)]
enum PendingJavaCall {
    None,
    LaunchPkg(String),
    KillPkg(String),
    RenameApp(String, String),
    SetAppHidden(String, bool),
    SaveDockOrder(String),
    SaveVoiceKeybind(i32, i32),
    SetKeyCaptureFlag(bool),
    OpenAppDetails(String),
    UninstallApp(String),
    DisableApp(String),
    StartVoiceAssistant,
    SearchInPackage(String, String),
    PromptProfileName(String),
    PromptWeatherCity(String),
    FetchWeather(bool, String),
    SavePrefs,
    Effect(SideEffect),
}

struct SharedState {
    ui: LauncherUi,
    mapper: RemoteMapper,
    waker: Option<AndroidAppWaker>,
    tz_offset_ms: i64,
    screen_dims: (f32, f32),
    ok_pressed_at: Option<Instant>,
    ok_long_fired: bool,
    pending_calls: Vec<PendingJavaCall>,
    last_frame_at: Option<Instant>,
    scene_dirty: bool,
    needs_redraw: bool,
}

impl SharedState {
    fn request_redraw(&mut self) {
        self.needs_redraw = true;
        if let Some(ref waker) = self.waker {
            waker.wake();
        }
    }
}

static STATE: Lazy<Mutex<SharedState>> = Lazy::new(|| {
    Mutex::new(SharedState {
        ui: LauncherUi::new(None, None, None, None, None),
        mapper: RemoteMapper::new(),
        waker: None,
        tz_offset_ms: 0,
        screen_dims: (1920.0, 1080.0),
        ok_pressed_at: None,
        ok_long_fired: false,
        pending_calls: Vec::new(),
        last_frame_at: None,
        scene_dirty: true,
        needs_redraw: true,
    })
});

fn load_asset_or_system_font(app: &AndroidApp, asset_path: &str, sys_paths: &[&str]) -> Option<Vec<u8>> {
    if let Some(c_path) = CString::new(asset_path).ok() {
        if let Some(mut asset) = app.asset_manager().open(&c_path) {
            let mut buf = Vec::new();
            if asset.read_to_end(&mut buf).is_ok() && !buf.is_empty() {
                return Some(buf);
            }
        }
    }
    for sp in sys_paths {
        if let Ok(bytes) = std::fs::read(sp) {
            if !bytes.is_empty() {
                return Some(bytes);
            }
        }
    }
    None
}

fn load_asset_bytes(app: &AndroidApp, path: &str) -> Option<Vec<u8>> {
    let c_path = CString::new(path).ok()?;
    let mut asset = app.asset_manager().open(&c_path)?;
    let mut buf = Vec::new();
    asset.read_to_end(&mut buf).ok()?;
    Some(buf)
}

fn decode_png_pixmap(bytes: &[u8]) -> Option<Pixmap> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    let mut raw = img.into_raw();
    premultiply_rgba(&mut raw);
    IntSize::from_wh(w, h).and_then(|sz| Pixmap::from_vec(raw, sz))
}

fn premultiply_rgba(rgba: &mut [u8]) {
    for px in rgba.chunks_exact_mut(4) {
        let a = px[3] as u32;
        if a < 255 {
            px[0] = ((px[0] as u32 * a + 127) / 255) as u8;
            px[1] = ((px[1] as u32 * a + 127) / 255) as u8;
            px[2] = ((px[2] as u32 * a + 127) / 255) as u8;
        }
    }
}

fn load_scene_into_ui(app: &AndroidApp, st: &mut SharedState) {
    let spec = st.ui.current_scene();
    if let Some(bg_bytes) = load_asset_bytes(app, spec.bg_asset) {
        if let Ok(img) = image::load_from_memory(&bg_bytes) {
            let rgba = img.to_rgba8();
            let (bw, bh) = rgba.dimensions();
            let fg_data = spec.fg_asset.and_then(|fg_path| {
                let bytes = load_asset_bytes(app, fg_path)?;
                let fg_img = image::load_from_memory(&bytes).ok()?.to_rgba8();
                let (fw, fh) = fg_img.dimensions();
                let mut fraw = fg_img.into_raw();
                premultiply_rgba(&mut fraw);
                Some((fw, fh, fraw))
            });
            st.ui.set_scene_pixmaps(bw, bh, rgba.into_raw(), fg_data);
        }
    } else {
        st.ui.ensure_procedural_scene();
    }
    st.scene_dirty = false;
}

fn refresh_timezone_offset(app: &AndroidApp) -> Option<i64> {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis() as i64;
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    let tz_class = env.find_class("java/util/TimeZone").ok()?;
    let tz_obj = env
        .call_static_method(tz_class, "getDefault", "()Ljava/util/TimeZone;", &[])
        .ok()?
        .l()
        .ok()?;
    let offset = env
        .call_method(tz_obj, "getOffset", "(J)I", &[JValue::Long(now_ms)])
        .ok()?
        .i()
        .ok()?;
    Some(offset as i64)
}

fn format_clock_greeting_date(
    tz_offset_ms: i64,
    user_name: &str,
    clock_24h: bool,
    lang_code: &str,
) -> (String, String, String) {
    let utc_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let local_secs = (utc_ms + tz_offset_ms).div_euclid(1000);
    let total_mins = local_secs.div_euclid(60).rem_euclid(24 * 60);
    let h24 = (total_mins / 60) as u32;
    let m = (total_mins % 60) as u32;
    let time_str = if clock_24h {
        format!("{:02}:{:02}", h24, m)
    } else {
        let h12 = match h24 % 12 {
            0 => 12,
            v => v,
        };
        format!("{}:{:02}", h12, m)
    };

    let greeting = crate::ui::localized_greeting(lang_code, h24, user_name);

    let days = local_secs.div_euclid(86_400);
    let dow = (days + 4).rem_euclid(7) as usize;
    let weekdays = [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ];
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month_idx = (if mp < 10 { mp + 3 } else { mp - 9 }) as usize;
    let months = [
        "",
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let date_str = format!("{} {} {}", weekdays[dow], day, months[month_idx]);

    (time_str, greeting, date_str)
}

fn render_window(window: &NativeWindow, st: &mut SharedState) -> bool {
    let w = window.width() as u32;
    let h = window.height() as u32;
    if w == 0 || h == 0 {
        return false;
    }
    st.screen_dims = (w as f32, h as f32);

    unsafe {
        ndk_sys::ANativeWindow_setBuffersGeometry(
            window.ptr().as_ptr(),
            w as i32,
            h as i32,
            ndk_sys::ANativeWindow_LegacyFormat::WINDOW_FORMAT_RGBA_8888.0 as i32,
        );
    }

    let mut pixmap = match Pixmap::new(w, h) {
        Some(p) => p,
        None => return false,
    };

    let now = Instant::now();
    let dt = st
        .last_frame_at
        .map(|prev| prev.elapsed().as_secs_f32().clamp(0.004, 0.045))
        .unwrap_or(1.0 / 60.0);
    let animating = st.ui.tick_animations(dt);
    st.last_frame_at = if animating { Some(now) } else { None };
    let (time_str, greeting_str, date_str) =
        format_clock_greeting_date(st.tz_offset_ms, &st.ui.user_name, st.ui.clock_24h, st.ui.lang_code);
    let key_label = st.mapper.voice_key_label.clone();
    let bound_count = st.mapper.bound_count;
    st.ui.render(
        &mut pixmap,
        &time_str,
        &greeting_str,
        &date_str,
        &key_label,
        bound_count,
    );

    if let Ok(mut buf) = window.lock(None) {
        let stride_px = buf.stride().max(0) as usize;
        let bits = buf.bits();
        if !bits.is_null() && stride_px >= w as usize {
            let dst_bytes = unsafe {
                std::slice::from_raw_parts_mut(bits as *mut u8, stride_px * (h as usize) * 4)
            };
            let src_bytes = pixmap.data();
            let row_bytes = (w as usize) * 4;
            for y in 0..(h as usize) {
                let src_off = y * row_bytes;
                let dst_off = y * stride_px * 4;
                dst_bytes[dst_off..dst_off + row_bytes]
                    .copy_from_slice(&src_bytes[src_off..src_off + row_bytes]);
            }
        }
    }

    st.needs_redraw = animating;
    animating
}

fn dispatch_java_calls(app: &AndroidApp, st: &mut SharedState) {
    if st.pending_calls.is_empty() {
        return;
    }
    let calls: Vec<PendingJavaCall> = st.pending_calls.drain(..).collect();
    let secure_mode = st.ui.sys.adb_mode_secure;

    if let Ok(vm) = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) } {
        if let Ok(mut env) = vm.attach_current_thread() {
            let act_obj = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };
            for call in calls {
                match call {
                    PendingJavaCall::None => {}
                    PendingJavaCall::LaunchPkg(pkg) => {
                        if let Ok(jp) = env.new_string(&pkg) {
                            let _ = env.call_method(&act_obj, "launchPackageFromRust", "(Ljava/lang/String;)V", &[(&jp).into()]);
                        }
                    }
                    PendingJavaCall::KillPkg(pkg) => {
                        if let Ok(jp) = env.new_string(&pkg) {
                            let _ = env.call_method(&act_obj, "killBackgroundPackageFromRust", "(Ljava/lang/String;)V", &[(&jp).into()]);
                        }
                    }
                    PendingJavaCall::RenameApp(pkg, cur) => {
                        if let (Ok(jp), Ok(jc)) = (env.new_string(&pkg), env.new_string(&cur)) {
                            let _ = env.call_method(
                                &act_obj,
                                "promptRenameAppFromRust",
                                "(Ljava/lang/String;Ljava/lang/String;)V",
                                &[(&jp).into(), (&jc).into()],
                            );
                        }
                    }
                    PendingJavaCall::SetAppHidden(pkg, hidden) => {
                        if let Ok(jp) = env.new_string(&pkg) {
                            let _ = env.call_method(
                                &act_obj,
                                "setAppHiddenFromRust",
                                "(Ljava/lang/String;Z)V",
                                &[(&jp).into(), JValue::Bool(if hidden { 1 } else { 0 })],
                            );
                        }
                    }
                    PendingJavaCall::SaveDockOrder(csv) => {
                        if let Ok(jc) = env.new_string(&csv) {
                            let _ = env.call_method(&act_obj, "saveDockOrderFromRust", "(Ljava/lang/String;)V", &[(&jc).into()]);
                        }
                    }
                    PendingJavaCall::SaveVoiceKeybind(kc, sc) => {
                        let _ = env.call_method(
                            &act_obj,
                            "saveVoiceKeybindFromRust",
                            "(II)V",
                            &[JValue::Int(kc), JValue::Int(sc)],
                        );
                    }
                    PendingJavaCall::SetKeyCaptureFlag(active) => {
                        let _ = env.call_method(
                            &act_obj,
                            "setKeyCaptureFlagFromRust",
                            "(Z)V",
                            &[JValue::Bool(if active { 1 } else { 0 })],
                        );
                    }
                    PendingJavaCall::OpenAppDetails(pkg) => {
                        if let Ok(jp) = env.new_string(&pkg) {
                            let _ = env.call_method(&act_obj, "openAppDetailsFromRust", "(Ljava/lang/String;)V", &[(&jp).into()]);
                        }
                    }
                    PendingJavaCall::UninstallApp(pkg) => {
                        if let Ok(jp) = env.new_string(&pkg) {
                            let _ = env.call_method(&act_obj, "uninstallAppFromRust", "(Ljava/lang/String;)V", &[(&jp).into()]);
                        }
                    }
                    PendingJavaCall::DisableApp(pkg) => {
                        if let Ok(jp) = env.new_string(&pkg) {
                            let _ = env.call_method(&act_obj, "disableAppFromRust", "(Ljava/lang/String;)V", &[(&jp).into()]);
                        }
                    }
                    PendingJavaCall::StartVoiceAssistant => {
                        let _ = env.call_method(&act_obj, "startVoiceAssistantFromRust", "()V", &[]);
                    }
                    PendingJavaCall::SearchInPackage(pkg, q) => {
                        if let (Ok(jp), Ok(jq)) = (env.new_string(&pkg), env.new_string(&q)) {
                            let _ = env.call_method(
                                &act_obj,
                                "searchInPackageFromRust",
                                "(Ljava/lang/String;Ljava/lang/String;)V",
                                &[(&jp).into(), (&jq).into()],
                            );
                        }
                    }
                    PendingJavaCall::PromptProfileName(cur) => {
                        if let Ok(jc) = env.new_string(&cur) {
                            let _ = env.call_method(&act_obj, "promptProfileNameFromRust", "(Ljava/lang/String;)V", &[(&jc).into()]);
                        }
                    }
                    PendingJavaCall::PromptWeatherCity(cur) => {
                        if let Ok(jc) = env.new_string(&cur) {
                            let _ = env.call_method(&act_obj, "promptWeatherCityFromRust", "(Ljava/lang/String;)V", &[(&jc).into()]);
                        }
                    }
                    PendingJavaCall::FetchWeather(auto_det, city) => {
                        if let Ok(jc) = env.new_string(&city) {
                            let _ = env.call_method(
                                &act_obj,
                                "fetchWeatherFromRust",
                                "(ZLjava/lang/String;)V",
                                &[JValue::Bool(if auto_det { 1 } else { 0 }), (&jc).into()],
                            );
                        }
                    }
                    PendingJavaCall::SavePrefs => {
                        if let (Ok(jn), Ok(jl), Ok(jc)) = (
                            env.new_string(&st.ui.user_name),
                            env.new_string(st.ui.lang_code),
                            env.new_string(&st.ui.weather_city),
                        ) {
                            let _ = env.call_method(
                                &act_obj,
                                "savePrefsFromRust",
                                "(ZILjava/lang/String;Ljava/lang/String;ZZZLjava/lang/String;)V",
                                &[
                                    JValue::Bool(if st.ui.setup_done { 1 } else { 0 }),
                                    JValue::Int(st.ui.setup.step as i32),
                                    (&jn).into(),
                                    (&jl).into(),
                                    JValue::Bool(if st.ui.classic_look { 1 } else { 0 }),
                                    JValue::Bool(if st.ui.weather_enabled { 1 } else { 0 }),
                                    JValue::Bool(if st.ui.weather_auto { 1 } else { 0 }),
                                    (&jc).into(),
                                ],
                            );
                        }
                    }
                    PendingJavaCall::Effect(eff) => match eff {
                        SideEffect::None => {}
                        SideEffect::OpenWifiSettings => {
                            let _ = env.call_method(&act_obj, "openWifiSettingsFromRust", "()V", &[]);
                        }
                        SideEffect::OpenBluetoothSettings => {
                            let _ = env.call_method(&act_obj, "openBluetoothSettingsFromRust", "()V", &[]);
                        }
                        SideEffect::OpenSoundSettings => {
                            let _ = env.call_method(&act_obj, "openSoundSettingsFromRust", "()V", &[]);
                        }
                        SideEffect::OpenDisplaySettings => {
                            let _ = env.call_method(&act_obj, "openDisplaySettingsFromRust", "()V", &[]);
                        }
                        SideEffect::OpenInputsOrTvSettings => {
                            let _ = env.call_method(&act_obj, "openInputsFromRust", "()V", &[]);
                        }
                        SideEffect::OpenAccessibilitySettings => {
                            let _ = env.call_method(&act_obj, "openAccessibilitySettingsFromRust", "()V", &[]);
                        }
                        SideEffect::OpenAndroidSettings => {
                            let _ = env.call_method(&act_obj, "openAndroidSettingsFromRust", "()V", &[]);
                        }
                        SideEffect::OpenDefaultHomePicker => {
                            let _ = env.call_method(&act_obj, "openHomePickerFromRust", "()V", &[]);
                        }
                        SideEffect::TriggerVoiceAssistant => {
                            let _ = env.call_method(&act_obj, "startVoiceAssistantFromRust", "()V", &[]);
                        }
                        SideEffect::LockScreenNow => {
                            let _ = env.call_method(&act_obj, "lockScreenFromRust", "()V", &[]);
                        }
                        SideEffect::CleanMemoryNow => {
                            let _ = env.call_method(&act_obj, "cleanMemoryFromRust", "()V", &[]);
                        }
                        SideEffect::RunAdbPairAndGrant => {
                            let _ = env.call_method(
                                &act_obj,
                                "runLocalAdbPairFromRust",
                                "(Z)V",
                                &[JValue::Bool(if secure_mode { 1 } else { 0 })],
                            );
                        }
                        SideEffect::ToggleAdbPort => {
                            let _ = env.call_method(&act_obj, "toggleAdbPortFromRust", "()V", &[]);
                        }
                        SideEffect::RefreshSystemSnapshot => {
                            let _ = env.call_method(&act_obj, "refreshSystemSnapshot", "()V", &[]);
                        }
                        SideEffect::SetKeepScreenOn(on) => {
                            let _ = env.call_method(
                                &act_obj,
                                "setKeepScreenOnFromRust",
                                "(Z)V",
                                &[JValue::Bool(if on { 1 } else { 0 })],
                            );
                        }
                        SideEffect::PromptProfileName => {
                            if let Ok(jc) = env.new_string(&st.ui.user_name) {
                                let _ = env.call_method(&act_obj, "promptProfileNameFromRust", "(Ljava/lang/String;)V", &[(&jc).into()]);
                            }
                        }
                        SideEffect::PromptWeatherCity => {
                            if let Ok(jc) = env.new_string(&st.ui.weather_city) {
                                let _ = env.call_method(&act_obj, "promptWeatherCityFromRust", "(Ljava/lang/String;)V", &[(&jc).into()]);
                            }
                        }
                        SideEffect::FetchWeatherNow => {
                            if let Ok(jc) = env.new_string(&st.ui.weather_city) {
                                let _ = env.call_method(
                                    &act_obj,
                                    "fetchWeatherFromRust",
                                    "(ZLjava/lang/String;)V",
                                    &[JValue::Bool(if st.ui.weather_auto { 1 } else { 0 }), (&jc).into()],
                                );
                            }
                        }
                        SideEffect::SavePrefs => {
                            if let (Ok(jn), Ok(jl), Ok(jc)) = (
                                env.new_string(&st.ui.user_name),
                                env.new_string(st.ui.lang_code),
                                env.new_string(&st.ui.weather_city),
                            ) {
                                let _ = env.call_method(
                                    &act_obj,
                                    "savePrefsFromRust",
                                    "(ZILjava/lang/String;Ljava/lang/String;ZZZLjava/lang/String;)V",
                                    &[
                                        JValue::Bool(if st.ui.setup_done { 1 } else { 0 }),
                                        JValue::Int(st.ui.setup.step as i32),
                                        (&jn).into(),
                                        (&jl).into(),
                                        JValue::Bool(if st.ui.classic_look { 1 } else { 0 }),
                                        JValue::Bool(if st.ui.weather_enabled { 1 } else { 0 }),
                                        JValue::Bool(if st.ui.weather_auto { 1 } else { 0 }),
                                        (&jc).into(),
                                    ],
                                );
                            }
                        }
                    },
                }
            }
        }
    }
}

fn apply_bound_action(st: &mut SharedState, act: BoundAction) {
    match act {
        BoundAction::OpenControlCenter => {
            if st.ui.control_center_open {
                st.ui.control_center_open = false;
            } else {
                st.ui.open_control_center();
                st.pending_calls.push(PendingJavaCall::Effect(SideEffect::RefreshSystemSnapshot));
            }
        }
        BoundAction::OpenSettings => {
            if st.ui.settings.open {
                st.ui.settings.open = false;
            } else {
                st.ui.control_center_open = false;
                st.ui.settings.open_at(SettingsCategory::Appearance);
                st.pending_calls.push(PendingJavaCall::Effect(SideEffect::RefreshSystemSnapshot));
            }
        }
        BoundAction::OpenSearch => {
            st.ui.control_center_open = false;
            st.ui.settings.open = false;
            st.ui.search_ui.open = true;
            st.ui.search_ui.query.clear();
            st.ui.search_ui.focus_row = 0;
            st.ui.search_ui.key_idx = 0;
            st.ui.search_ui.result_idx = 0;
        }
        BoundAction::OpenRecentApps => {
            st.ui.control_center_open = false;
            st.ui.settings.open = false;
            st.ui.recent_apps_open = !st.ui.recent_apps_open;
            st.pending_calls.push(PendingJavaCall::Effect(SideEffect::RefreshSystemSnapshot));
        }
        BoundAction::ToggleClockStyle => {
            st.ui.clock_style = (st.ui.clock_style + 1) % 3;
        }
        BoundAction::CycleWallpaper => {
            st.ui.scene_idx = (st.ui.scene_idx + 1) % crate::ui::SCENES.len();
            st.ui.invalidate_scene_cache();
            st.scene_dirty = true;
        }
        BoundAction::TriggerVoiceAssist => {
            st.pending_calls.push(PendingJavaCall::StartVoiceAssistant);
        }
        BoundAction::LaunchApp(pkg) => {
            st.pending_calls.push(PendingJavaCall::LaunchPkg(pkg));
        }
    }
}

#[no_mangle]
pub extern "C" fn Java_com_eadxmz_ferrodeck_MainActivity_nativeApplyLoadedPrefs(
    mut env: JNIEnv,
    _class: JClass,
    setup_done: jboolean,
    setup_step: jint,
    profile_name: JString,
    lang_code: JString,
    classic_look: jboolean,
    weather_enabled: jboolean,
    weather_auto: jboolean,
    weather_city: JString,
) {
    let name: String = env.get_string(&profile_name).map(|s| s.into()).unwrap_or_default();
    let lang: String = env.get_string(&lang_code).map(|s| s.into()).unwrap_or_default();
    let city: String = env.get_string(&weather_city).map(|s| s.into()).unwrap_or_default();

    if let Ok(mut st) = STATE.lock() {
        st.ui.setup_done = setup_done != 0;
        if !name.trim().is_empty() {
            st.ui.user_name = name.trim().to_string();
        }
        if let Some(entry) = crate::ui::ALL_LANGUAGES.iter().find(|l| l.code == lang.as_str()) {
            st.ui.lang_code = entry.code;
            st.ui.lang_region_global = !entry.is_native_india;
            st.ui.setup.lang_region_global = !entry.is_native_india;
        }
        st.ui.classic_look = classic_look != 0;
        st.ui.clock_style = if st.ui.classic_look { 1 } else { 0 };
        st.ui.weather_enabled = weather_enabled != 0;
        st.ui.weather_auto = weather_auto != 0;
        st.ui.weather_city = city;
        if !st.ui.setup_done && !st.ui.setup.open {
            st.ui.setup.open_at_step(setup_step.max(0) as usize);
        }
        st.request_redraw();
    }
}

#[no_mangle]
pub extern "C" fn Java_com_eadxmz_ferrodeck_MainActivity_nativeUpdateWeather(
    mut env: JNIEnv,
    _class: JClass,
    city: JString,
    temp_text: JString,
    desc_text: JString,
) {
    let c: String = env.get_string(&city).map(|s| s.into()).unwrap_or_default();
    let t: String = env.get_string(&temp_text).map(|s| s.into()).unwrap_or_default();
    let d: String = env.get_string(&desc_text).map(|s| s.into()).unwrap_or_default();
    if let Ok(mut st) = STATE.lock() {
        if !c.is_empty() {
            st.ui.weather_city = c;
        }
        st.ui.weather_temp = t;
        st.ui.weather_desc = d;
        st.request_redraw();
    }
}

#[no_mangle]
pub extern "C" fn Java_com_eadxmz_ferrodeck_MainActivity_nativeOnHomePressed(
    _env: JNIEnv,
    _class: JClass,
) {
    if let Ok(mut st) = STATE.lock() {
        st.ui.on_home_pressed();
        st.request_redraw();
    }
}

#[no_mangle]
pub extern "C" fn Java_com_eadxmz_ferrodeck_MainActivity_nativeOnRemoteKey(
    _env: JNIEnv,
    _class: JClass,
    key_code: jint,
    scan_code: jint,
) {
    if let Ok(mut st) = STATE.lock() {
        if let Some(act) = st.mapper.on_raw_key(key_code, scan_code) {
            apply_bound_action(&mut st, act);
        }
        if let Some((vkc, vsc)) = st.mapper.captured_voice_pair.take() {
            st.pending_calls.push(PendingJavaCall::SaveVoiceKeybind(vkc, vsc));
            st.pending_calls.push(PendingJavaCall::SetKeyCaptureFlag(false));
        } else if st.mapper.capture_target == CaptureTarget::None {
            st.pending_calls.push(PendingJavaCall::SetKeyCaptureFlag(false));
        }
        st.request_redraw();
    }
}

#[no_mangle]
pub extern "C" fn Java_com_eadxmz_ferrodeck_MainActivity_nativeOpenSearchWithQuery(
    mut env: JNIEnv,
    _class: JClass,
    query: JString,
) {
    let q: String = env.get_string(&query).map(|s| s.into()).unwrap_or_default();
    if let Ok(mut st) = STATE.lock() {
        st.ui.control_center_open = false;
        st.ui.settings.open = false;
        st.ui.app_menu.open = false;
        st.ui.search_ui.open = true;
        st.ui.search_ui.query = q;
        st.ui.search_ui.focus_row = if st.ui.search_ui.query.is_empty() { 0 } else { 1 };
        st.ui.search_ui.result_idx = 0;
        st.request_redraw();
    }
}

#[no_mangle]
pub extern "C" fn Java_com_eadxmz_ferrodeck_MainActivity_nativeSetVoiceHud(
    mut env: JNIEnv,
    _class: JClass,
    active: jboolean,
    title: JString,
    subtitle: JString,
) {
    let t: String = env.get_string(&title).map(|s| s.into()).unwrap_or_default();
    let sub: String = env.get_string(&subtitle).map(|s| s.into()).unwrap_or_default();
    if let Ok(mut st) = STATE.lock() {
        st.ui.voice_hud = if active != 0 { Some((t, sub)) } else { None };
        st.request_redraw();
    }
}

#[no_mangle]
pub extern "C" fn Java_com_eadxmz_ferrodeck_MainActivity_nativeSetHiddenApps(
    mut env: JNIEnv,
    _class: JClass,
    packages: JObjectArray,
    labels: JObjectArray,
) {
    let count = env.get_array_length(&packages).unwrap_or(0).max(0) as usize;
    let mut hidden = Vec::with_capacity(count);
    for i in 0..count {
        let pkg: String = env
            .get_object_array_element(&packages, i as i32)
            .ok()
            .and_then(|o| env.get_string(&JString::from(o)).ok().map(|s| s.into()))
            .unwrap_or_default();
        let lbl: String = env
            .get_object_array_element(&labels, i as i32)
            .ok()
            .and_then(|o| env.get_string(&JString::from(o)).ok().map(|s| s.into()))
            .unwrap_or_default();
        if !pkg.is_empty() {
            hidden.push((pkg, lbl));
        }
    }
    if let Ok(mut st) = STATE.lock() {
        st.ui.hidden_apps = hidden;
        st.request_redraw();
    }
}

#[no_mangle]
pub extern "C" fn Java_com_eadxmz_ferrodeck_MainActivity_nativeUpdateSystemStatus(
    mut env: JNIEnv,
    _class: JClass,
    wifi_label: JString,
    bt_on: jboolean,
    dnd_on: jboolean,
    ram_used_mb: jint,
    ram_total_mb: jint,
    adb_port_open: jboolean,
    a11y_enabled: jboolean,
    default_home: jboolean,
    write_secure: jboolean,
    status_msg: JString,
) {
    let wifi: String = env
        .get_string(&wifi_label)
        .map(|s| s.into())
        .unwrap_or_else(|_| "Connected".into());
    let msg: String = env
        .get_string(&status_msg)
        .map(|s| s.into())
        .unwrap_or_default();

    if let Ok(mut st) = STATE.lock() {
        st.ui.sys.wifi_label = wifi;
        st.ui.sys.bt_on = bt_on != 0;
        st.ui.sys.dnd_on = dnd_on != 0;
        st.ui.sys.ram_used_mb = ram_used_mb.max(0) as u32;
        st.ui.sys.ram_total_mb = ram_total_mb.max(1) as u32;
        st.ui.sys.adb_port_open = adb_port_open != 0;
        st.ui.sys.a11y_enabled = a11y_enabled != 0;
        st.ui.sys.default_home = default_home != 0;
        st.ui.sys.write_secure = write_secure != 0;
        if !msg.is_empty() {
            st.ui.settings.status_banner = Some(msg);
        }
        st.request_redraw();
    }
}

#[no_mangle]
pub extern "C" fn Java_com_eadxmz_ferrodeck_MainActivity_nativeSetApps(
    mut env: JNIEnv,
    _class: JClass,
    packages: JObjectArray,
    labels: JObjectArray,
    icons: JObjectArray,
    is_builtin: JBooleanArray,
    icon_size: jint,
) {
    let mut builtin_flags = vec![0u8; env.get_array_length(&packages).unwrap_or(0).max(0) as usize];
    let _ = env.get_boolean_array_region(&is_builtin, 0, &mut builtin_flags);
    let count = env.get_array_length(&packages).unwrap_or(0).max(0) as usize;
    let sz = icon_size.max(0) as u32;
    let expected_len = (sz * sz * 4) as usize;
    let palette: [[u8; 3]; 6] = [
        [34, 72, 138],
        [142, 42, 68],
        [28, 108, 92],
        [96, 54, 148],
        [156, 86, 32],
        [44, 62, 82],
    ];
    let mut tiles = Vec::with_capacity(count);
    for i in 0..count {
        let pkg: String = env
            .get_object_array_element(&packages, i as i32)
            .ok()
            .and_then(|o| env.get_string(&JString::from(o)).ok().map(|s| s.into()))
            .unwrap_or_default();
        let lbl: String = env
            .get_object_array_element(&labels, i as i32)
            .ok()
            .and_then(|o| env.get_string(&JString::from(o)).ok().map(|s| s.into()))
            .unwrap_or_default();

        let icon_pm = if sz > 0 {
            env.get_object_array_element(&icons, i as i32)
                .ok()
                .and_then(|o| {
                    if o.is_null() {
                        return None;
                    }
                    let jarr = JByteArray::from(o);
                    let mut raw = env.convert_byte_array(&jarr).ok()?;
                    if raw.len() == expected_len {
                        premultiply_rgba(&mut raw);
                        IntSize::from_wh(sz, sz).and_then(|dim| Pixmap::from_vec(raw, dim))
                    } else {
                        None
                    }
                })
        } else {
            None
        };

        tiles.push(AppTile {
            package_name: pkg,
            label: lbl,
            icon: icon_pm,
            accent: palette[i % palette.len()],
            is_builtin: builtin_flags.get(i).copied().unwrap_or(0) != 0,
        });
    }
    if let Ok(mut st) = STATE.lock() {
        st.ui.set_apps(tiles);
        st.request_redraw();
    }
}

fn execute_app_menu_selection(st: &mut SharedState) {
    let idx = st.ui.app_menu.app_idx;
    let choice = st.ui.app_menu.selected;
    st.ui.app_menu.open = false;

    let Some(app) = st.ui.apps.get(idx).cloned() else {
        return;
    };
    if app.package_name == SETTINGS_TILE_PKG {
        return;
    }

    match choice {
        0 => {
            // Open
            st.pending_calls.push(PendingJavaCall::LaunchPkg(app.package_name));
        }
        1 => {
            // Rename
            st.pending_calls.push(PendingJavaCall::RenameApp(app.package_name, app.label));
        }
        2 => {
            // Move Left
            if idx > 0 {
                st.ui.apps.swap(idx, idx - 1);
                st.ui.selected_index = idx - 1;
                persist_dock_order(st);
            }
        }
        3 => {
            // Move Right
            let max_app_idx = st.ui.apps.len().saturating_sub(2);
            if idx < max_app_idx {
                st.ui.apps.swap(idx, idx + 1);
                st.ui.selected_index = idx + 1;
                persist_dock_order(st);
            }
        }
        4 => {
            // Hide App
            st.pending_calls.push(PendingJavaCall::SetAppHidden(app.package_name, true));
        }
        5 => {
            // App Details
            st.pending_calls.push(PendingJavaCall::OpenAppDetails(app.package_name));
        }
        6 => {
            // Disable (built-in) or Uninstall (user-installed)
            if app.is_builtin {
                st.pending_calls.push(PendingJavaCall::DisableApp(app.package_name));
            } else {
                st.pending_calls.push(PendingJavaCall::UninstallApp(app.package_name));
            }
        }
        _ => {}
    }
}

fn persist_dock_order(st: &mut SharedState) {
    let csv: Vec<String> = st
        .ui
        .apps
        .iter()
        .filter(|a| a.package_name != SETTINGS_TILE_PKG)
        .map(|a| a.package_name.clone())
        .collect();
    st.pending_calls.push(PendingJavaCall::SaveDockOrder(csv.join(",")));
}

fn execute_setup_action(st: &mut SharedState, action_id: &str) {
    match action_id {
        "setup_next" => {
            st.ui.settings.status_banner = None;
            st.ui.setup.next_step();
            if !st.ui.setup.open {
                st.ui.setup_done = true;
            }
            st.pending_calls.push(PendingJavaCall::SavePrefs);
        }
        "setup_finish" => {
            st.ui.setup.open = false;
            st.ui.setup_done = true;
            st.pending_calls.push(PendingJavaCall::SavePrefs);
            if st.ui.weather_enabled {
                st.pending_calls.push(PendingJavaCall::FetchWeather(
                    st.ui.weather_auto,
                    st.ui.weather_city.clone(),
                ));
            }
        }
        "setup_toggle_lang_region" => {
            st.ui.setup.lang_region_global = !st.ui.setup.lang_region_global;
            st.ui.lang_region_global = st.ui.setup.lang_region_global;
        }
        id if id.starts_with("set_lang:") => {
            let code = &id["set_lang:".len()..];
            if let Some(entry) = crate::ui::ALL_LANGUAGES.iter().find(|l| l.code == code) {
                st.ui.lang_code = entry.code;
                st.pending_calls.push(PendingJavaCall::SavePrefs);
            }
        }
        "setup_edit_name" => {
            st.pending_calls
                .push(PendingJavaCall::PromptProfileName(st.ui.user_name.clone()));
        }
        "setup_cycle_avatar" => {
            st.ui.setup.avatar_idx =
                (st.ui.setup.avatar_idx + 1) % crate::setup_wizard::AVATAR_COLORS.len();
        }
        "setup_weather_auto" => {
            st.ui.weather_enabled = true;
            st.ui.weather_auto = true;
            st.pending_calls.push(PendingJavaCall::SavePrefs);
            st.pending_calls
                .push(PendingJavaCall::FetchWeather(true, String::new()));
        }
        "setup_weather_manual" => {
            st.ui.weather_enabled = true;
            st.ui.weather_auto = false;
            st.pending_calls
                .push(PendingJavaCall::PromptWeatherCity(st.ui.weather_city.clone()));
        }
        "setup_weather_off" => {
            st.ui.weather_enabled = false;
            st.pending_calls.push(PendingJavaCall::SavePrefs);
        }
        "setup_look_scenes" => {
            st.ui.classic_look = false;
            st.ui.clock_style = 0;
            st.pending_calls.push(PendingJavaCall::SavePrefs);
        }
        "setup_look_classic" => {
            st.ui.classic_look = true;
            st.ui.clock_style = 1;
            st.pending_calls.push(PendingJavaCall::SavePrefs);
        }
        "setup_capture_voice" => {
            st.mapper.capture_target = CaptureTarget::VoiceAssistant;
            st.mapper.voice_key_label = "Press remote button now...".into();
            st.pending_calls.push(PendingJavaCall::SetKeyCaptureFlag(true));
        }
        "setup_capture_cc" => {
            st.mapper.capture_target = CaptureTarget::ControlCenter;
            st.mapper.last_captured_label = "Press remote button now...".into();
            st.pending_calls.push(PendingJavaCall::SetKeyCaptureFlag(true));
        }
        "setup_default_home" => {
            st.pending_calls.push(PendingJavaCall::SavePrefs);
            st.pending_calls
                .push(PendingJavaCall::Effect(SideEffect::OpenDefaultHomePicker));
        }
        "setup_a11y" => {
            st.pending_calls.push(PendingJavaCall::SavePrefs);
            st.pending_calls
                .push(PendingJavaCall::Effect(SideEffect::OpenAccessibilitySettings));
        }
        "setup_adb_easy" => {
            st.ui.settings.status_banner = Some("Checking Local ADB (127.0.0.1:5555)...".into());
            st.pending_calls.push(PendingJavaCall::SavePrefs);
            st.pending_calls
                .push(PendingJavaCall::Effect(SideEffect::RunAdbPairAndGrant));
        }
        _ => {}
    }
}

fn activate_ok_press(st: &mut SharedState, row_count: usize) {
    if st.ui.setup.open {
        let opts = st.ui.setup.build_options(
            st.ui.lang_code,
            &st.ui.user_name,
            st.ui.weather_auto,
            st.ui.weather_enabled,
            &st.ui.weather_city,
            st.ui.classic_look,
            &st.mapper.voice_key_label,
            st.ui.sys.default_home,
            st.ui.sys.a11y_enabled,
        );
        if let Some(opt) = opts.get(st.ui.setup.selected) {
            execute_setup_action(st, opt.action_id);
        }
        return;
    }

    if st.ui.app_menu.open {
        execute_app_menu_selection(st);
        return;
    }

    if st.ui.search_ui.open {
        if st.ui.search_ui.focus_row == 0 {
            let k = SEARCH_KEYS[st.ui.search_ui.key_idx % SEARCH_KEYS.len()];
            match k {
                "SPACE" => st.ui.search_ui.query.push(' '),
                "DEL" => {
                    st.ui.search_ui.query.pop();
                }
                "MIC" => {
                    st.pending_calls.push(PendingJavaCall::StartVoiceAssistant);
                }
                ch => st.ui.search_ui.query.push_str(&ch.to_lowercase()),
            }
        } else {
            let results = st.ui.search_matching_apps();
            if let Some((pkg, _lbl, is_action)) = results.get(st.ui.search_ui.result_idx).cloned() {
                let q = st.ui.search_ui.query.trim().to_string();
                st.ui.search_ui.open = false;
                if is_action && !q.is_empty() {
                    st.pending_calls.push(PendingJavaCall::SearchInPackage(pkg, q));
                } else {
                    st.pending_calls.push(PendingJavaCall::LaunchPkg(pkg));
                }
            }
        }
        return;
    }

    if st.ui.ambient_mode_open {
        st.ui.ambient_mode_open = false;
    } else if st.ui.settings.open {
        if !st.ui.settings.in_right_pane {
            st.ui.settings.move_right(row_count);
        } else {
            let rows = st
                .ui
                .build_settings_rows(&st.mapper.voice_key_label, st.mapper.bound_count);
            if let Some(item) = rows.get(st.ui.settings.row_idx) {
                if item.action_id == "capture_voice_key" {
                    st.mapper.capture_target = CaptureTarget::VoiceAssistant;
                    st.mapper.voice_key_label = "Press remote button now…".into();
                    st.pending_calls.push(PendingJavaCall::SetKeyCaptureFlag(true));
                } else if item.action_id == "capture_cc_key" {
                    st.mapper.capture_target = CaptureTarget::ControlCenter;
                    st.mapper.last_captured_label = "Press remote button now…".into();
                    st.pending_calls.push(PendingJavaCall::SetKeyCaptureFlag(true));
                } else if let Some(rest) = item.action_id.strip_prefix("unhide:") {
                    if let Ok(hidx) = rest.parse::<usize>() {
                        if let Some((pkg, _)) = st.ui.hidden_apps.get(hidx).cloned() {
                            st.pending_calls.push(PendingJavaCall::SetAppHidden(pkg, false));
                        }
                    }
                } else {
                    let (dirty, eff, _) = st.ui.execute_setting_action(item.action_id);
                    if dirty {
                        st.scene_dirty = true;
                    }
                    if eff != SideEffect::None {
                        st.pending_calls.push(PendingJavaCall::Effect(eff));
                    }
                }
            }
        }
    } else if st.ui.control_center_open {
        let (eff, _) = st.ui.activate_control_center_item();
        if eff != SideEffect::None {
            st.pending_calls.push(PendingJavaCall::Effect(eff));
        }
    } else if st.ui.recent_apps_open {
        let app_count = st.ui.apps.len().saturating_sub(1);
        if app_count > 0 {
            let idx = st.ui.recent_selected % app_count;
            if let Some(tile) = st.ui.apps.get(idx) {
                st.ui.recent_apps_open = false;
                st.pending_calls.push(PendingJavaCall::LaunchPkg(tile.package_name.clone()));
            }
        }
    } else if st.ui.weather_popup_open {
        st.ui.weather_popup_open = false;
    } else {
        match st.ui.focus_zone {
            FocusZone::TopWeather => {
                st.ui.weather_popup_open = true;
            }
            FocusZone::TopControlCenter => {
                st.ui.open_control_center();
                st.pending_calls.push(PendingJavaCall::Effect(SideEffect::RefreshSystemSnapshot));
            }
            FocusZone::Dock => {
                if let Some(app_tile) = st.ui.apps.get(st.ui.selected_index) {
                    if app_tile.package_name == SETTINGS_TILE_PKG {
                        st.pending_calls.push(PendingJavaCall::Effect(SideEffect::OpenAndroidSettings));
                    } else {
                        st.pending_calls.push(PendingJavaCall::LaunchPkg(app_tile.package_name.clone()));
                    }
                }
            }
        }
    }
}

fn handle_key_event(st: &mut SharedState, kc: i32, sc: i32, action: KeyAction, repeat_count: i32) -> bool {
    if action == KeyAction::Up {
        if kc == AKEYCODE_DPAD_CENTER || kc == AKEYCODE_ENTER {
            let was_long = st.ok_long_fired;
            st.ok_pressed_at = None;
            st.ok_long_fired = false;
            if !was_long {
                let row_count = if st.ui.settings.open {
                    st.ui
                        .build_settings_rows(&st.mapper.voice_key_label, st.mapper.bound_count)
                        .len()
                } else {
                    0
                };
                activate_ok_press(st, row_count);
                st.needs_redraw = true;
            }
            return true;
        }
        return false;
    }

    if action != KeyAction::Down {
        return false;
    }

    if let Some(bound) = st.mapper.on_raw_key(kc, sc) {
        apply_bound_action(st, bound);
        st.needs_redraw = true;
        return true;
    }
    if let Some((vkc, vsc)) = st.mapper.captured_voice_pair.take() {
        st.pending_calls.push(PendingJavaCall::SaveVoiceKeybind(vkc, vsc));
        st.pending_calls.push(PendingJavaCall::SetKeyCaptureFlag(false));
        st.needs_redraw = true;
        return true;
    }
    if st.mapper.capture_target == CaptureTarget::None && repeat_count == 0 {
        st.pending_calls.push(PendingJavaCall::SetKeyCaptureFlag(false));
    }

    let row_count = if st.ui.settings.open {
        st.ui
            .build_settings_rows(&st.mapper.voice_key_label, st.mapper.bound_count)
            .len()
    } else {
        0
    };

    if st.ui.setup.open {
        let setup_len = st
            .ui
            .setup
            .build_options(
                st.ui.lang_code,
                &st.ui.user_name,
                st.ui.weather_auto,
                st.ui.weather_enabled,
                &st.ui.weather_city,
                st.ui.classic_look,
                &st.mapper.voice_key_label,
                st.ui.sys.default_home,
                st.ui.sys.a11y_enabled,
            )
            .len();
        match kc {
            AKEYCODE_DPAD_UP => {
                st.ui.setup.move_up(setup_len);
                st.needs_redraw = true;
                return true;
            }
            AKEYCODE_DPAD_DOWN => {
                st.ui.setup.move_down(setup_len);
                st.needs_redraw = true;
                return true;
            }
            AKEYCODE_DPAD_LEFT => {
                if st.ui.setup.step == 1 {
                    let n = crate::setup_wizard::AVATAR_COLORS.len();
                    st.ui.setup.avatar_idx = (st.ui.setup.avatar_idx + n - 1) % n;
                    st.needs_redraw = true;
                    return true;
                }
            }
            AKEYCODE_DPAD_RIGHT => {
                if st.ui.setup.step == 1 {
                    st.ui.setup.avatar_idx =
                        (st.ui.setup.avatar_idx + 1) % crate::setup_wizard::AVATAR_COLORS.len();
                    st.needs_redraw = true;
                    return true;
                }
            }
            AKEYCODE_BACK => {
                if st.ui.setup.step > 0 {
                    st.ui.settings.status_banner = None;
                    st.ui.setup.prev_step();
                    st.pending_calls.push(PendingJavaCall::SavePrefs);
                } else {
                    st.ui.setup.open = false;
                    st.ui.setup_done = true;
                    st.pending_calls.push(PendingJavaCall::SavePrefs);
                }
                st.needs_redraw = true;
                return true;
            }
            _ => {}
        }
    }

    match kc {
        AKEYCODE_DPAD_LEFT => {
            st.ui.move_dpad(-1, 0, row_count);
            st.needs_redraw = true;
            true
        }
        AKEYCODE_DPAD_RIGHT => {
            st.ui.move_dpad(1, 0, row_count);
            st.needs_redraw = true;
            true
        }
        AKEYCODE_DPAD_UP => {
            if st.ui.recent_apps_open {
                let app_count = st.ui.apps.len().saturating_sub(1);
                if app_count > 0 {
                    let idx = st.ui.recent_selected % app_count;
                    if let Some(tile) = st.ui.apps.get(idx) {
                        st.pending_calls.push(PendingJavaCall::KillPkg(tile.package_name.clone()));
                    }
                }
            } else {
                st.ui.move_dpad(0, -1, row_count);
            }
            st.needs_redraw = true;
            true
        }
        AKEYCODE_DPAD_DOWN => {
            st.ui.move_dpad(0, 1, row_count);
            st.needs_redraw = true;
            true
        }
        AKEYCODE_DPAD_CENTER | AKEYCODE_ENTER => {
            if repeat_count == 0 {
                st.ok_pressed_at = Some(Instant::now());
                st.ok_long_fired = false;
            } else if !st.ok_long_fired
                && !st.ui.setup.open
                && !st.ui.app_menu.open
                && !st.ui.search_ui.open
                && !st.ui.settings.open
                && !st.ui.control_center_open
                && st.ui.focus_zone == FocusZone::Dock
            {
                st.ui.open_app_context_menu();
                st.ok_long_fired = true;
            }
            st.needs_redraw = true;
            true
        }
        AKEYCODE_DEL => {
            if st.ui.search_ui.open {
                st.ui.search_ui.query.pop();
                st.needs_redraw = true;
                return true;
            }
            false
        }
        29..=54 if st.ui.search_ui.open => {
            let ch = (b'a' + (kc - 29) as u8) as char;
            st.ui.search_ui.query.push(ch);
            st.ui.search_ui.result_idx = 0;
            st.needs_redraw = true;
            true
        }
        7..=16 if st.ui.search_ui.open => {
            let ch = (b'0' + (kc - 7) as u8) as char;
            st.ui.search_ui.query.push(ch);
            st.ui.search_ui.result_idx = 0;
            st.needs_redraw = true;
            true
        }
        62 if st.ui.search_ui.open => {
            st.ui.search_ui.query.push(' ');
            st.needs_redraw = true;
            true
        }
        AKEYCODE_BACK => {
            if st.ui.app_menu.open {
                st.ui.app_menu.open = false;
            } else if st.ui.search_ui.open {
                st.ui.search_ui.open = false;
            } else if st.ui.ambient_mode_open {
                st.ui.ambient_mode_open = false;
            } else if st.ui.settings.open {
                if st.ui.settings.in_right_pane {
                    st.ui.settings.in_right_pane = false;
                } else {
                    st.ui.settings.open = false;
                }
            } else if st.ui.control_center_open {
                st.ui.control_center_open = false;
            } else if st.ui.recent_apps_open {
                st.ui.recent_apps_open = false;
            } else if st.ui.weather_popup_open {
                st.ui.weather_popup_open = false;
            } else if st.ui.focus_zone != FocusZone::Dock {
                st.ui.focus_zone = FocusZone::Dock;
            }
            st.needs_redraw = true;
            true
        }
        AKEYCODE_HOME => {
            st.ui.on_home_pressed();
            st.needs_redraw = true;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub fn android_main(app: AndroidApp) {
    let big_font = load_asset_or_system_font(
        &app,
        "fonts/Antonio-Light.ttf",
        &[
            "/system/fonts/Roboto-Thin.ttf",
            "/system/fonts/Roboto-Light.ttf",
            "/system/fonts/Roboto-Regular.ttf",
            "/system/fonts/DroidSans.ttf",
        ],
    );
    let bold_font = load_asset_or_system_font(
        &app,
        "fonts/InterDisplay-Bold.ttf",
        &[
            "/system/fonts/Roboto-Bold.ttf",
            "/system/fonts/Roboto-Medium.ttf",
            "/system/fonts/DroidSans-Bold.ttf",
        ],
    );
    let med_font = load_asset_or_system_font(
        &app,
        "fonts/Inter-Medium.ttf",
        &[
            "/system/fonts/Roboto-Medium.ttf",
            "/system/fonts/Roboto-Regular.ttf",
            "/system/fonts/DroidSans.ttf",
        ],
    );
    let reg_font = load_asset_or_system_font(
        &app,
        "fonts/Inter-Regular.ttf",
        &[
            "/system/fonts/Roboto-Regular.ttf",
            "/system/fonts/DroidSans.ttf",
        ],
    );
    let settings_icon = load_asset_bytes(&app, "icons/System Settings.png")
        .as_deref()
        .and_then(decode_png_pixmap);
    let tz_offset = refresh_timezone_offset(&app).unwrap_or(0);

    if let Ok(mut st) = STATE.lock() {
        // Preserve any apps, weather, or setup prefs already pushed by MainActivity.onCreate()
        let prev_apps = std::mem::take(&mut st.ui.apps);
        let prev_hidden = std::mem::take(&mut st.ui.hidden_apps);
        let prev_setup_done = st.ui.setup_done;
        let prev_setup_open = st.ui.setup.open;
        let prev_setup_step = st.ui.setup.step;
        let prev_name = st.ui.user_name.clone();
        let prev_lang = st.ui.lang_code;
        let prev_lang_global = st.ui.lang_region_global;
        let prev_classic = st.ui.classic_look;
        let prev_clock_style = st.ui.clock_style;
        let prev_w_en = st.ui.weather_enabled;
        let prev_w_auto = st.ui.weather_auto;
        let prev_w_city = st.ui.weather_city.clone();
        let prev_w_temp = st.ui.weather_temp.clone();
        let prev_w_desc = st.ui.weather_desc.clone();
        let prev_sys = st.ui.sys.clone();

        st.waker = Some(app.create_waker());
        st.tz_offset_ms = tz_offset;
        st.ui = LauncherUi::new(
            big_font.as_deref(),
            bold_font.as_deref(),
            med_font.as_deref(),
            reg_font.as_deref(),
            settings_icon,
        );
        if !prev_apps.is_empty() {
            // Re-attach the settings gear icon if it was missing before asset load
            let mut apps_clean: Vec<AppTile> = prev_apps
                .into_iter()
                .filter(|a| a.package_name != SETTINGS_TILE_PKG)
                .collect();
            st.ui.set_apps(apps_clean.drain(..).collect());
        }
        st.ui.hidden_apps = prev_hidden;
        st.ui.setup_done = prev_setup_done;
        st.ui.setup.open = prev_setup_open || !prev_setup_done;
        st.ui.setup.step = prev_setup_step;
        st.ui.user_name = prev_name;
        st.ui.lang_code = prev_lang;
        st.ui.lang_region_global = prev_lang_global;
        st.ui.setup.lang_region_global = prev_lang_global;
        st.ui.classic_look = prev_classic;
        st.ui.clock_style = prev_clock_style;
        st.ui.weather_enabled = prev_w_en;
        st.ui.weather_auto = prev_w_auto;
        st.ui.weather_city = prev_w_city;
        st.ui.weather_temp = prev_w_temp;
        st.ui.weather_desc = prev_w_desc;
        st.ui.sys = prev_sys;

        load_scene_into_ui(&app, &mut st);
    }

    let mut running = true;
    while running {
        let timeout = {
            let st = STATE.lock().unwrap();
            if st.needs_redraw {
                Some(Duration::from_millis(16))
            } else if let Some(down_at) = st.ok_pressed_at {
                if !st.ok_long_fired {
                    let elapsed = down_at.elapsed();
                    let hold_target = Duration::from_millis(480);
                    Some(hold_target.saturating_sub(elapsed).max(Duration::from_millis(10)))
                } else {
                    Some(Duration::from_secs(30))
                }
            } else {
                let now_secs = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let secs_to_next_min = 60 - (now_secs % 60);
                Some(Duration::from_secs(secs_to_next_min.max(1)))
            }
        };

        let mut refresh_tz = false;
        app.poll_events(timeout, |event| match event {
            PollEvent::Main(main_ev) => match main_ev {
                MainEvent::Destroy => running = false,
                MainEvent::InitWindow { .. }
                | MainEvent::WindowResized { .. }
                | MainEvent::RedrawNeeded { .. }
                | MainEvent::ConfigChanged { .. } => {
                    if let Ok(mut st) = STATE.lock() {
                        st.needs_redraw = true;
                    }
                }
                MainEvent::GainedFocus | MainEvent::Resume { .. } => {
                    refresh_tz = true;
                    if let Ok(mut st) = STATE.lock() {
                        st.needs_redraw = true;
                    }
                }
                _ => {}
            },
            PollEvent::Timeout | PollEvent::Wake => {
                if let Ok(mut st) = STATE.lock() {
                    st.needs_redraw = true;
                }
            }
            _ => {}
        });

        // Check if OK button was held >= 480ms on a dock app -> open App Long-Press Menu!
        {
            let mut st = STATE.lock().unwrap();
            if let Some(down_at) = st.ok_pressed_at {
                if !st.ok_long_fired
                    && down_at.elapsed() >= Duration::from_millis(480)
                    && !st.ui.setup.open
                    && !st.ui.app_menu.open
                    && !st.ui.search_ui.open
                    && !st.ui.settings.open
                    && !st.ui.control_center_open
                    && st.ui.focus_zone == FocusZone::Dock
                {
                    st.ui.open_app_context_menu();
                    st.ok_long_fired = true;
                    st.needs_redraw = true;
                }
            }
        }

        if refresh_tz {
            if let Some(offset) = refresh_timezone_offset(&app) {
                if let Ok(mut st) = STATE.lock() {
                    st.tz_offset_ms = offset;
                }
            }
        }

        if let Ok(mut iter) = app.input_events_iter() {
            while iter.next(|event| {
                match event {
                    InputEvent::KeyEvent(ke) => {
                        let kc: i32 = u32::from(ke.key_code()) as i32;
                        let sc: i32 = ke.scan_code();
                        let act = ke.action();
                        let rep = ke.repeat_count();
                        let mut st = STATE.lock().unwrap();
                        if handle_key_event(&mut st, kc, sc, act, rep) {
                            return InputStatus::Handled;
                        }
                    }
                    InputEvent::MotionEvent(me) => {
                        if me.action() == MotionAction::Up {
                            if let Some(ptr) = me.pointers().next() {
                                let x = ptr.x();
                                let y = ptr.y();
                                let mut st = STATE.lock().unwrap();
                                if st.ui.setup.open {
                                    let (sw, sh) = st.screen_dims;
                                    let unit = (sw / 960.0).min(sh / 540.0);
                                    // Top-right Skip Setup pill tap
                                    if x >= sw - 160.0 * unit && y <= 66.0 * unit {
                                        execute_setup_action(&mut st, "setup_finish");
                                        st.needs_redraw = true;
                                        return InputStatus::Handled;
                                    }
                                    let opts = st.ui.setup.build_options(
                                        st.ui.lang_code,
                                        &st.ui.user_name,
                                        st.ui.weather_auto,
                                        st.ui.weather_enabled,
                                        &st.ui.weather_city,
                                        st.ui.classic_look,
                                        &st.mapper.voice_key_label,
                                        st.ui.sys.default_home,
                                        st.ui.sys.a11y_enabled,
                                    );
                                    let list_top = if st.ui.setup.step == 1 {
                                        (192.0 * unit).round()
                                    } else {
                                        (146.0 * unit).round()
                                    };
                                    let card_w = (460.0 * unit).round();
                                    let card_x = ((sw - card_w) * 0.5).round();
                                    let row_h = (42.0 * unit).round();
                                    let row_gap = (9.0 * unit).round();
                                    let max_vis = 6usize;
                                    let start_idx = if st.ui.setup.selected >= max_vis {
                                        st.ui.setup.selected + 1 - max_vis
                                    } else {
                                        0
                                    };
                                    if x >= card_x && x <= card_x + card_w {
                                        for vis_i in 0..max_vis.min(opts.len().saturating_sub(start_idx)) {
                                            let ry = list_top + (vis_i as f32) * (row_h + row_gap);
                                            if y >= ry && y <= ry + row_h {
                                                let abs_i = start_idx + vis_i;
                                                st.ui.setup.selected = abs_i;
                                                execute_setup_action(&mut st, opts[abs_i].action_id);
                                                break;
                                            }
                                        }
                                    }
                                    st.needs_redraw = true;
                                    return InputStatus::Handled;
                                }
                                if st.ui.app_menu.open {
                                    // Compact Liquid Glass app context menu hit-test
                                    let (sw, sh) = st.screen_dims;
                                    let unit = (sw / 960.0).min(sh / 540.0);
                                    let items = st.ui.app_menu_items();
                                    let (px, py, panel_w, panel_h, pad, header_h, row_h, row_gap) =
                                        st.ui.app_menu_geometry(sw, sh, unit);
                                    if x < px || x > px + panel_w || y < py || y > py + panel_h {
                                        st.ui.app_menu.open = false;
                                    } else {
                                        let list_y = py + pad + header_h;
                                        for i in 0..items.len() {
                                            let ry = list_y + (i as f32) * (row_h + row_gap);
                                            if y >= ry && y <= ry + row_h {
                                                st.ui.app_menu.selected = i;
                                                execute_app_menu_selection(&mut st);
                                                break;
                                            }
                                        }
                                    }
                                    st.needs_redraw = true;
                                    return InputStatus::Handled;
                                }
                                let (sw, sh) = st.screen_dims;
                                let (pkg, eff) = st.ui.handle_tap(x, y, sw, sh);
                                if let Some(p) = pkg {
                                    st.pending_calls.push(PendingJavaCall::LaunchPkg(p));
                                }
                                if eff != SideEffect::None {
                                    st.pending_calls.push(PendingJavaCall::Effect(eff));
                                }
                                st.needs_redraw = true;
                                return InputStatus::Handled;
                            }
                        }
                    }
                    _ => {}
                }
                InputStatus::Unhandled
            }) {}
        }

        let mut st = STATE.lock().unwrap();
        if st.scene_dirty {
            load_scene_into_ui(&app, &mut st);
        }
        if st.needs_redraw {
            if let Some(win) = app.native_window() {
                render_window(&win, &mut st);
            }
        }
        dispatch_java_calls(&app, &mut st);
    }
}

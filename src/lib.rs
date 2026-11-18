//! FerroDeck — Event-driven Rust NativeActivity TV Launcher (`android-activity` 0.6).

mod clock;
mod input;
mod settings_pill;
mod ui;

use crate::input::{
    BoundAction, RemoteMapper, AKEYCODE_BACK, AKEYCODE_DPAD_CENTER, AKEYCODE_DPAD_DOWN,
    AKEYCODE_DPAD_LEFT, AKEYCODE_DPAD_RIGHT, AKEYCODE_DPAD_UP, AKEYCODE_ENTER, AKEYCODE_HOME,
};
use crate::settings_pill::SettingsCategory;
use crate::ui::{AppTile, FocusZone, LauncherUi, SideEffect, SETTINGS_TILE_PKG};
use android_activity::input::{InputEvent, KeyAction, MotionAction};
use android_activity::{AndroidApp, AndroidAppWaker, InputStatus, MainEvent, PollEvent};
use jni::objects::{JByteArray, JClass, JObject, JObjectArray, JString, JValue};
use jni::sys::{jboolean, jint};
use jni::JNIEnv;
use ndk::native_window::NativeWindow;
use once_cell::sync::Lazy;
use std::ffi::CString;
use std::io::Read;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tiny_skia::{IntSize, Pixmap};

struct SharedState {
    ui: LauncherUi,
    mapper: RemoteMapper,
    waker: Option<AndroidAppWaker>,
    tz_offset_ms: i64,
    screen_dims: (f32, f32),
    pending_launch_pkg: Option<String>,
    pending_kill_pkg: Option<String>,
    pending_effect: SideEffect,
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
        pending_launch_pkg: None,
        pending_kill_pkg: None,
        pending_effect: SideEffect::None,
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

    let greet_prefix = if h24 < 12 {
        "Good morning"
    } else if h24 < 17 {
        "Good afternoon"
    } else {
        "Good evening"
    };
    let greeting = format!("{}, {}", greet_prefix, user_name);

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

    let animating = st.ui.tick_animations();
    let (time_str, greeting_str, date_str) =
        format_clock_greeting_date(st.tz_offset_ms, &st.ui.user_name, st.ui.clock_24h);
    let key_label = st.mapper.last_captured_label.clone();
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

fn dispatch_java_side_effects(app: &AndroidApp, st: &mut SharedState) {
    let launch_pkg = st.pending_launch_pkg.take();
    let kill_pkg = st.pending_kill_pkg.take();
    let effect = std::mem::replace(&mut st.pending_effect, SideEffect::None);
    let secure_mode = st.ui.sys.adb_mode_secure;

    if launch_pkg.is_none() && kill_pkg.is_none() && effect == SideEffect::None {
        return;
    }

    if let Ok(vm) = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) } {
        if let Ok(mut env) = vm.attach_current_thread() {
            let act_obj = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };
            if let Some(pkg) = launch_pkg {
                if let Ok(j_pkg) = env.new_string(&pkg) {
                    let _ = env.call_method(
                        &act_obj,
                        "launchPackageFromRust",
                        "(Ljava/lang/String;)V",
                        &[(&j_pkg).into()],
                    );
                }
            }
            if let Some(kpkg) = kill_pkg {
                if let Ok(j_pkg) = env.new_string(&kpkg) {
                    let _ = env.call_method(
                        &act_obj,
                        "killBackgroundPackageFromRust",
                        "(Ljava/lang/String;)V",
                        &[(&j_pkg).into()],
                    );
                }
            }
            match effect {
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
                SideEffect::TriggerVoiceSearch => {
                    let _ = env.call_method(&act_obj, "triggerVoiceAssistFromRust", "()V", &[]);
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
                st.pending_effect = SideEffect::RefreshSystemSnapshot;
            }
        }
        BoundAction::OpenSettings => {
            if st.ui.settings.open {
                st.ui.settings.open = false;
            } else {
                st.ui.control_center_open = false;
                st.ui.settings.open_at(SettingsCategory::Appearance);
                st.pending_effect = SideEffect::RefreshSystemSnapshot;
            }
        }
        BoundAction::OpenRecentApps => {
            st.ui.control_center_open = false;
            st.ui.settings.open = false;
            st.ui.recent_apps_open = !st.ui.recent_apps_open;
            st.pending_effect = SideEffect::RefreshSystemSnapshot;
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
            st.pending_effect = SideEffect::TriggerVoiceSearch;
        }
        BoundAction::LaunchApp(pkg) => {
            st.pending_launch_pkg = Some(pkg);
        }
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
    icon_size: jint,
) {
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
            .and_then(|o| {
                let jstr = JString::from(o);
                env.get_string(&jstr).ok().map(|s| s.into())
            })
            .unwrap_or_default();
        let lbl: String = env
            .get_object_array_element(&labels, i as i32)
            .ok()
            .and_then(|o| {
                let jstr = JString::from(o);
                env.get_string(&jstr).ok().map(|s| s.into())
            })
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
        });
    }
    if let Ok(mut st) = STATE.lock() {
        st.ui.set_apps(tiles);
        st.request_redraw();
    }
}

fn handle_key_down(st: &mut SharedState, kc: i32, sc: i32) -> bool {
    if let Some(bound) = st.mapper.on_raw_key(kc, sc) {
        apply_bound_action(st, bound);
        st.needs_redraw = true;
        return true;
    }

    let row_count = if st.ui.settings.open {
        st.ui
            .build_settings_rows(&st.mapper.last_captured_label, st.mapper.bound_count)
            .len()
    } else {
        0
    };

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
                        st.pending_kill_pkg = Some(tile.package_name.clone());
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
            if st.ui.ambient_mode_open {
                st.ui.ambient_mode_open = false;
            } else if st.ui.settings.open {
                if !st.ui.settings.in_right_pane {
                    st.ui.settings.move_right(row_count);
                } else {
                    let rows = st
                        .ui
                        .build_settings_rows(&st.mapper.last_captured_label, st.mapper.bound_count);
                    if let Some(item) = rows.get(st.ui.settings.row_idx) {
                        let (dirty, eff, start_capture) =
                            st.ui.execute_setting_action(item.action_id);
                        if dirty {
                            st.scene_dirty = true;
                        }
                        if start_capture {
                            st.mapper.capture_mode = true;
                            st.mapper.last_captured_label = "Press remote button now…".into();
                        }
                        if eff != SideEffect::None {
                            st.pending_effect = eff;
                        }
                    }
                }
            } else if st.ui.control_center_open {
                let (eff, _) = st.ui.activate_control_center_item();
                if eff != SideEffect::None {
                    st.pending_effect = eff;
                }
            } else if st.ui.recent_apps_open {
                let app_count = st.ui.apps.len().saturating_sub(1);
                if app_count > 0 {
                    let idx = st.ui.recent_selected % app_count;
                    if let Some(tile) = st.ui.apps.get(idx) {
                        st.ui.recent_apps_open = false;
                        st.pending_launch_pkg = Some(tile.package_name.clone());
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
                        st.pending_effect = SideEffect::RefreshSystemSnapshot;
                    }
                    FocusZone::Dock => {
                        if let Some(app_tile) = st.ui.apps.get(st.ui.selected_index) {
                            if app_tile.package_name == SETTINGS_TILE_PKG {
                                st.ui.settings.open_at(SettingsCategory::Appearance);
                                st.pending_effect = SideEffect::RefreshSystemSnapshot;
                            } else {
                                st.pending_launch_pkg = Some(app_tile.package_name.clone());
                            }
                        }
                    }
                }
            }
            st.needs_redraw = true;
            true
        }
        AKEYCODE_BACK => {
            if st.ui.ambient_mode_open {
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
        st.waker = Some(app.create_waker());
        st.tz_offset_ms = tz_offset;
        st.ui = LauncherUi::new(
            big_font.as_deref(),
            bold_font.as_deref(),
            med_font.as_deref(),
            reg_font.as_deref(),
            settings_icon,
        );
        load_scene_into_ui(&app, &mut st);
    }

    let mut running = true;
    while running {
        let timeout = {
            let st = STATE.lock().unwrap();
            if st.needs_redraw {
                Some(Duration::from_millis(16))
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
                        if ke.action() == KeyAction::Down {
                            let kc: i32 = u32::from(ke.key_code()) as i32;
                            let sc: i32 = ke.scan_code();
                            let mut st = STATE.lock().unwrap();
                            if handle_key_down(&mut st, kc, sc) {
                                return InputStatus::Handled;
                            }
                        }
                    }
                    InputEvent::MotionEvent(me) => {
                        if me.action() == MotionAction::Up {
                            if let Some(ptr) = me.pointers().next() {
                                let x = ptr.x();
                                let y = ptr.y();
                                let mut st = STATE.lock().unwrap();
                                let (sw, sh) = st.screen_dims;
                                let (pkg, eff) = st.ui.handle_tap(x, y, sw, sh);
                                if let Some(p) = pkg {
                                    st.pending_launch_pkg = Some(p);
                                }
                                if eff != SideEffect::None {
                                    st.pending_effect = eff;
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
        dispatch_java_side_effects(&app, &mut st);
    }
}

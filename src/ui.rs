//! FerroDeck Complete Native Renderer (`tiny-skia` + `fontdue`):
//! 1. Night Scenes Home Screen (scene-night.jpg + giant 1:1 glass clock behind mountain peaks +
//!    greeting/date + top-right Weather & Control Center pills + centered Liquid Glass squircle dock)
//! 2. Exact Control Center Overlay (`image_4f8f03.jpg`): 8 right-aligned stadium-pill toggles
//!    (Wi-Fi, Bluetooth, Memory with live usage bar, Do Not Disturb, Audio, Sleep Timer,
//!     Ambient Mode, Power Off) + 4 squircle action buttons (Inputs, Button Mapper, Settings, Search)
//! 3. Recent Apps Switcher (`image_4f8f85.jpg`): Top-right live RAM meter pill + horizontal app cards
//! 4. Full Two-Pane Home Settings (`HomeSettingsScreen`) including all Advanced ADB options.

use crate::clock::{draw_crisp_label, measure_label_width, CrispClock};
use crate::glyphs::{draw_glyph, Glyph};
use crate::settings_pill::{rounded_rect_path, HomeSettingsScreen, SettingRowItem, SettingsCategory};
use crate::setup_wizard::SetupWizard;
pub struct LangEntry {
    pub code: &'static str,
    pub is_native_india: bool,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub action_id: &'static str,
}

pub const ALL_LANGUAGES: &[LangEntry] = &[
    // NATIVE (INDIA) — English kept as Native default alongside Indian languages
    LangEntry {
        code: "en",
        is_native_india: true,
        title: "English (Native)",
        subtitle: "Use FerroDeck in English · India Native default",
        action_id: "set_lang:en",
    },
    LangEntry {
        code: "hi",
        is_native_india: true,
        title: "Hindi",
        subtitle: "Use FerroDeck in Hindi (India)",
        action_id: "set_lang:hi",
    },
    LangEntry {
        code: "ta",
        is_native_india: true,
        title: "Tamil",
        subtitle: "Use FerroDeck in Tamil (India)",
        action_id: "set_lang:ta",
    },
    LangEntry {
        code: "te",
        is_native_india: true,
        title: "Telugu",
        subtitle: "Use FerroDeck in Telugu (India)",
        action_id: "set_lang:te",
    },
    LangEntry {
        code: "bn",
        is_native_india: true,
        title: "Bengali",
        subtitle: "Use FerroDeck in Bengali (India)",
        action_id: "set_lang:bn",
    },
    LangEntry {
        code: "mr",
        is_native_india: true,
        title: "Marathi",
        subtitle: "Use FerroDeck in Marathi (India)",
        action_id: "set_lang:mr",
    },
    LangEntry {
        code: "kn",
        is_native_india: true,
        title: "Kannada",
        subtitle: "Use FerroDeck in Kannada (India)",
        action_id: "set_lang:kn",
    },
    LangEntry {
        code: "ml",
        is_native_india: true,
        title: "Malayalam",
        subtitle: "Use FerroDeck in Malayalam (India)",
        action_id: "set_lang:ml",
    },
    LangEntry {
        code: "gu",
        is_native_india: true,
        title: "Gujarati",
        subtitle: "Use FerroDeck in Gujarati (India)",
        action_id: "set_lang:gu",
    },
    LangEntry {
        code: "pa",
        is_native_india: true,
        title: "Punjabi",
        subtitle: "Use FerroDeck in Punjabi (India)",
        action_id: "set_lang:pa",
    },
    LangEntry {
        code: "ur",
        is_native_india: true,
        title: "Urdu",
        subtitle: "Use FerroDeck in Urdu (India)",
        action_id: "set_lang:ur",
    },
    // GLOBAL
    LangEntry {
        code: "tr",
        is_native_india: false,
        title: "Türkçe",
        subtitle: "FerroDeck'i Türkçe kullanın",
        action_id: "set_lang:tr",
    },
    LangEntry {
        code: "es",
        is_native_india: false,
        title: "Español",
        subtitle: "Usar FerroDeck en español",
        action_id: "set_lang:es",
    },
    LangEntry {
        code: "fr",
        is_native_india: false,
        title: "Français",
        subtitle: "Utiliser FerroDeck en français",
        action_id: "set_lang:fr",
    },
    LangEntry {
        code: "de",
        is_native_india: false,
        title: "Deutsch",
        subtitle: "FerroDeck auf Deutsch verwenden",
        action_id: "set_lang:de",
    },
    LangEntry {
        code: "it",
        is_native_india: false,
        title: "Italiano",
        subtitle: "Usa FerroDeck in italiano",
        action_id: "set_lang:it",
    },
    LangEntry {
        code: "pt",
        is_native_india: false,
        title: "Português",
        subtitle: "Usar o FerroDeck em português",
        action_id: "set_lang:pt",
    },
    LangEntry {
        code: "sv",
        is_native_india: false,
        title: "Svenska",
        subtitle: "Använd FerroDeck på svenska",
        action_id: "set_lang:sv",
    },
    LangEntry {
        code: "ru",
        is_native_india: false,
        title: "Russian",
        subtitle: "Use FerroDeck in Russian (Global)",
        action_id: "set_lang:ru",
    },
    LangEntry {
        code: "ar",
        is_native_india: false,
        title: "Arabic",
        subtitle: "Use FerroDeck in Arabic (Global)",
        action_id: "set_lang:ar",
    },
    LangEntry {
        code: "ja",
        is_native_india: false,
        title: "Japanese",
        subtitle: "Use FerroDeck in Japanese (Global)",
        action_id: "set_lang:ja",
    },
    LangEntry {
        code: "ko",
        is_native_india: false,
        title: "Korean",
        subtitle: "Use FerroDeck in Korean (Global)",
        action_id: "set_lang:ko",
    },
    LangEntry {
        code: "zh",
        is_native_india: false,
        title: "Chinese",
        subtitle: "Use FerroDeck in Chinese (Global)",
        action_id: "set_lang:zh",
    },
];

pub fn localized_greeting(lang_code: &str, h24: u32, user_name: &str) -> String {
    let prefix = match lang_code {
        "hi" | "mr" | "bn" | "pa" | "gu" | "ta" | "te" | "kn" | "ml" => "Namaste",
        "ur" => "Assalamu Alaikum",
        "tr" => {
            if h24 < 12 {
                "Günaydın"
            } else if h24 < 17 {
                "İyi günler"
            } else {
                "İyi akşamlar"
            }
        }
        "es" => {
            if h24 < 12 {
                "Buenos días"
            } else if h24 < 19 {
                "Buenas tardes"
            } else {
                "Buenas noches"
            }
        }
        "fr" => {
            if h24 < 18 {
                "Bonjour"
            } else {
                "Bonsoir"
            }
        }
        "de" => {
            if h24 < 12 {
                "Guten Morgen"
            } else if h24 < 18 {
                "Guten Tag"
            } else {
                "Guten Abend"
            }
        }
        "it" => {
            if h24 < 18 {
                "Buongiorno"
            } else {
                "Buonasera"
            }
        }
        "pt" => {
            if h24 < 12 {
                "Bom dia"
            } else if h24 < 18 {
                "Boa tarde"
            } else {
                "Boa noite"
            }
        }
        "sv" => {
            if h24 < 12 {
                "God morgon"
            } else if h24 < 18 {
                "God dag"
            } else {
                "God kväll"
            }
        }
        _ => {
            if h24 < 12 {
                "Good morning"
            } else if h24 < 17 {
                "Good afternoon"
            } else {
                "Good evening"
            }
        }
    };
    format!("{}, {}", prefix, user_name)
}

use fontdue::{Font, FontSettings};
use tiny_skia::{
    Color, FillRule, FilterQuality, GradientStop, LinearGradient, Paint, PathBuilder, Pattern,
    Pixmap, PixmapPaint, Point, SpreadMode, Stroke, Transform,
};

pub const DESIGN_W: f32 = 960.0;
pub const DESIGN_H: f32 = 540.0;
pub const MAX_DOCK_TILES: usize = 10;
pub const SETTINGS_TILE_PKG: &str = "__ferrodeck_settings__";

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FocusZone {
    Dock,
    TopWeather,
    TopControlCenter,
}

#[derive(Clone, Debug)]
pub struct SceneSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub bg_asset: &'static str,
    pub fg_asset: Option<&'static str>,
    pub clock_top_u: f32,
    pub clock_size_u: f32,
    pub date_top_u: f32,
    pub sky_top: [u8; 3],
    pub sky_mid: [u8; 3],
    pub sky_bot: [u8; 3],
    pub ridge_far: [u8; 3],
    pub ridge_near: [u8; 3],
}

pub const SCENES: &[SceneSpec] = &[
    SceneSpec {
        id: "night",
        label: "Night",
        bg_asset: "backgrounds/scene-night.jpg",
        fg_asset: Some("backgrounds/scene-night-fg.webp"),
        clock_top_u: 62.0,
        clock_size_u: 178.0,
        date_top_u: 42.0,
        sky_top: [14, 18, 42],
        sky_mid: [36, 42, 86],
        sky_bot: [74, 68, 122],
        ridge_far: [22, 26, 54],
        ridge_near: [11, 14, 28],
    },
    SceneSpec {
        id: "golden",
        label: "Golden Hour",
        bg_asset: "backgrounds/scene-golden.jpg",
        fg_asset: Some("backgrounds/scene-golden-fg.webp"),
        clock_top_u: 76.0,
        clock_size_u: 195.0,
        date_top_u: 53.0,
        sky_top: [44, 34, 68],
        sky_mid: [148, 76, 62],
        sky_bot: [228, 148, 72],
        ridge_far: [68, 38, 48],
        ridge_near: [28, 18, 28],
    },
    SceneSpec {
        id: "fog",
        label: "Fog",
        bg_asset: "backgrounds/scene-fog.jpg",
        fg_asset: Some("backgrounds/scene-fog-fg.webp"),
        clock_top_u: 86.0,
        clock_size_u: 210.0,
        date_top_u: 58.0,
        sky_top: [54, 68, 84],
        sky_mid: [98, 114, 130],
        sky_bot: [148, 162, 174],
        ridge_far: [64, 78, 92],
        ridge_near: [34, 44, 54],
    },
    SceneSpec {
        id: "dawn",
        label: "Rainforest Dawn",
        bg_asset: "backgrounds/scene-dawn.jpg",
        fg_asset: Some("backgrounds/scene-dawn-fg.webp"),
        clock_top_u: 115.0,
        clock_size_u: 190.0,
        date_top_u: 88.0,
        sky_top: [20, 42, 54],
        sky_mid: [58, 96, 98],
        sky_bot: [152, 168, 142],
        ridge_far: [26, 54, 52],
        ridge_near: [14, 30, 28],
    },
    SceneSpec {
        id: "water",
        label: "Waterline",
        bg_asset: "backgrounds/scene-water.jpg",
        fg_asset: None,
        clock_top_u: 110.0,
        clock_size_u: 195.0,
        date_top_u: 82.0,
        sky_top: [16, 36, 64],
        sky_mid: [38, 84, 126],
        sky_bot: [88, 142, 178],
        ridge_far: [22, 54, 86],
        ridge_near: [12, 28, 48],
    },
];

#[allow(dead_code)]
#[derive(Clone)]
pub struct AppTile {
    pub package_name: String,
    pub label: String,
    pub icon: Option<Pixmap>,
    pub accent: [u8; 3],
    pub is_builtin: bool,
}

#[derive(Clone, Debug)]
pub struct SystemSnapshot {
    pub wifi_label: String,
    pub bt_on: bool,
    pub dnd_on: bool,
    pub ram_used_mb: u32,
    pub ram_total_mb: u32,
    pub adb_mode_secure: bool,
    pub adb_port_open: bool,
    pub a11y_enabled: bool,
    pub default_home: bool,
    pub write_secure: bool,
}

impl Default for SystemSnapshot {
    fn default() -> Self {
        Self {
            wifi_label: "Connected".into(),
            bt_on: true,
            dnd_on: false,
            ram_used_mb: 1150,
            ram_total_mb: 2048,
            adb_mode_secure: true,
            adb_port_open: false,
            a11y_enabled: false,
            default_home: true,
            write_secure: false,
        }
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SideEffect {
    None,
    OpenWifiSettings,
    OpenBluetoothSettings,
    OpenSoundSettings,
    OpenDisplaySettings,
    OpenInputsOrTvSettings,
    OpenAccessibilitySettings,
    OpenAndroidSettings,
    OpenDefaultHomePicker,
    TriggerVoiceAssistant,
    LockScreenNow,
    CleanMemoryNow,
    RunAdbPairAndGrant,
    ToggleAdbPort,
    RefreshSystemSnapshot,
    SetKeepScreenOn(bool),
    PromptProfileName,
    PromptWeatherCity,
    FetchWeatherNow,
    SavePrefs,
}

#[derive(Clone, Debug)]
pub struct AppContextMenu {
    pub open: bool,
    pub app_idx: usize,
    pub selected: usize,
}

#[derive(Clone, Debug)]
pub struct SearchOverlayState {
    pub open: bool,
    pub query: String,
    pub focus_row: usize, // 0 = keyboard row, 1 = results row
    pub key_idx: usize,
    pub result_idx: usize,
}

pub const SEARCH_KEYS: &[&str] = &[
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M",
    "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z",
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "SPACE", "DEL", "MIC",
];

pub struct LauncherUi {
    pub apps: Vec<AppTile>,
    pub hidden_apps: Vec<(String, String)>,
    pub focus_zone: FocusZone,
    pub selected_index: usize,
    pub page_offset: usize,

    // Overlays
    pub control_center_open: bool,
    pub cc_selected: usize,
    pub recent_apps_open: bool,
    pub recent_selected: usize,
    pub ambient_mode_open: bool,
    pub weather_popup_open: bool,
    pub app_menu: AppContextMenu,
    pub search_ui: SearchOverlayState,
    pub voice_hud: Option<(String, String)>,
    pub setup: SetupWizard,
    pub settings: HomeSettingsScreen,

    // Launcher preferences
    pub setup_done: bool,
    pub lang_region_global: bool,
    pub classic_look: bool,
    pub clock_style: u8, // 0 = Giant Scene Glass Clock, 1 = Top-Right Crisp, 2 = Hidden
    pub clock_24h: bool,
    pub scene_idx: usize,
    pub user_name: String,
    pub weather_enabled: bool,
    pub weather_auto: bool,
    pub weather_city: String,
    pub weather_temp: String,
    pub weather_desc: String,
    pub sleep_timer_mins: u32,
    pub keep_alive_enabled: bool,
    pub caffeine_enabled: bool,
    pub auto_updates: bool,
    pub lang_code: &'static str,

    pub sys: SystemSnapshot,
    wallpaper_pm: Option<Pixmap>,
    foreground_pm: Option<Pixmap>,
    glass_backdrop_pm: Option<Pixmap>,
    settings_icon_pm: Option<Pixmap>,
    focus_scales: Vec<f32>,
    font_medium: Option<Font>,
    font_regular: Option<Font>,
    pub clock: CrispClock,
}

impl LauncherUi {
    pub fn new(
        big_font: Option<&[u8]>,
        bold_font: Option<&[u8]>,
        med_font: Option<&[u8]>,
        reg_font: Option<&[u8]>,
        settings_icon: Option<Pixmap>,
    ) -> Self {
        let font_medium = med_font.or(bold_font).or(reg_font).and_then(|b| {
            Font::from_bytes(
                b,
                FontSettings {
                    scale: 32.0,
                    ..Default::default()
                },
            )
            .ok()
        });
        let font_regular = reg_font.or(med_font).or(bold_font).and_then(|b| {
            Font::from_bytes(
                b,
                FontSettings {
                    scale: 32.0,
                    ..Default::default()
                },
            )
            .ok()
        });
        Self {
            apps: Vec::new(),
            hidden_apps: Vec::new(),
            focus_zone: FocusZone::Dock,
            selected_index: 0,
            page_offset: 0,
            control_center_open: false,
            cc_selected: 0,
            recent_apps_open: false,
            recent_selected: 0,
            ambient_mode_open: false,
            weather_popup_open: false,
            app_menu: AppContextMenu {
                open: false,
                app_idx: 0,
                selected: 0,
            },
            search_ui: SearchOverlayState {
                open: false,
                query: String::new(),
                focus_row: 0,
                key_idx: 0,
                result_idx: 0,
            },
            voice_hud: None,
            setup: SetupWizard::new(bold_font, med_font, reg_font),
            settings: HomeSettingsScreen::new(bold_font, med_font, reg_font),
            setup_done: false,
            lang_region_global: false,
            classic_look: false,
            clock_style: 0,
            clock_24h: false,
            scene_idx: 0,
            user_name: "Home".into(),
            weather_enabled: true,
            weather_auto: true,
            weather_city: String::new(),
            weather_temp: String::new(),
            weather_desc: String::new(),
            sleep_timer_mins: 0,
            keep_alive_enabled: true,
            caffeine_enabled: false,
            auto_updates: true,
            lang_code: "en",
            sys: SystemSnapshot::default(),
            wallpaper_pm: None,
            foreground_pm: None,
            glass_backdrop_pm: None,
            settings_icon_pm: settings_icon,
            focus_scales: Vec::new(),
            font_medium,
            font_regular,
            clock: CrispClock::new(big_font.or(bold_font), bold_font.or(med_font)),
        }
    }

    pub fn current_scene(&self) -> &'static SceneSpec {
        &SCENES[self.scene_idx % SCENES.len()]
    }

    pub fn set_apps(&mut self, mut apps: Vec<AppTile>) {
        apps.push(AppTile {
            package_name: SETTINGS_TILE_PKG.into(),
            label: "Home settings".into(),
            icon: self.settings_icon_pm.clone(),
            accent: [48, 54, 68],
            is_builtin: true,
        });
        self.focus_scales = vec![1.0; apps.len()];
        if !self.focus_scales.is_empty() {
            self.focus_scales[0] = 1.15;
        }
        self.apps = apps;
        if self.selected_index >= self.apps.len() {
            self.selected_index = 0;
        }
    }

    pub fn set_scene_pixmaps(
        &mut self,
        bg_w: u32,
        bg_h: u32,
        bg_rgba: Vec<u8>,
        fg: Option<(u32, u32, Vec<u8>)>,
    ) {
        if let Some(size) = tiny_skia::IntSize::from_wh(bg_w, bg_h) {
            if let Some(pm) = Pixmap::from_vec(bg_rgba, size) {
                self.glass_backdrop_pm = Some(bake_liquid_glass(&pm));
                self.wallpaper_pm = Some(pm);
            }
        }
        self.foreground_pm = fg.and_then(|(fw, fh, frgba)| {
            tiny_skia::IntSize::from_wh(fw, fh).and_then(|sz| Pixmap::from_vec(frgba, sz))
        });
    }

    pub fn ensure_procedural_scene(&mut self) {
        if self.wallpaper_pm.is_some() {
            return;
        }
        let spec = self.current_scene();
        let w = 960u32;
        let h = 540u32;
        if let Some(mut bg) = Pixmap::new(w, h) {
            if let Some(grad) = LinearGradient::new(
                Point::from_xy(0.0, 0.0),
                Point::from_xy(0.0, h as f32),
                vec![
                    GradientStop::new(0.0, Color::from_rgba8(spec.sky_top[0], spec.sky_top[1], spec.sky_top[2], 255)),
                    GradientStop::new(0.55, Color::from_rgba8(spec.sky_mid[0], spec.sky_mid[1], spec.sky_mid[2], 255)),
                    GradientStop::new(1.0, Color::from_rgba8(spec.sky_bot[0], spec.sky_bot[1], spec.sky_bot[2], 255)),
                ],
                SpreadMode::Pad,
                Transform::identity(),
            ) {
                let mut p = Paint::default();
                p.shader = grad;
                if let Some(r) = tiny_skia::Rect::from_xywh(0.0, 0.0, w as f32, h as f32) {
                    bg.fill_rect(r, &p, Transform::identity(), None);
                }
            }

            if spec.id == "night" {
                let mut sp = Paint::default();
                sp.anti_alias = true;
                for i in 0..75u32 {
                    let sx = ((i * 197 + 43) % 940 + 10) as f32;
                    let sy = ((i * 89 + 17) % 240 + 12) as f32;
                    let r = if i % 5 == 0 { 1.4 } else { 0.85 };
                    let alpha = if i % 3 == 0 { 210 } else { 125 };
                    sp.set_color(Color::from_rgba8(235, 242, 255, alpha));
                    if let Some(star) = rounded_rect_path(sx, sy, r * 2.0, r * 2.0, r) {
                        bg.fill_path(&star, &sp, FillRule::Winding, Transform::identity(), None);
                    }
                }
            }

            let mut far_pb = PathBuilder::new();
            far_pb.move_to(0.0, h as f32);
            far_pb.line_to(0.0, 320.0);
            far_pb.line_to(140.0, 268.0);
            far_pb.line_to(310.0, 305.0);
            far_pb.line_to(490.0, 242.0);
            far_pb.line_to(680.0, 292.0);
            far_pb.line_to(840.0, 256.0);
            far_pb.line_to(w as f32, 298.0);
            far_pb.line_to(w as f32, h as f32);
            far_pb.close();
            if let Some(path) = far_pb.finish() {
                let mut fp = Paint::default();
                fp.anti_alias = true;
                fp.set_color(Color::from_rgba8(spec.ridge_far[0], spec.ridge_far[1], spec.ridge_far[2], 255));
                bg.fill_path(&path, &fp, FillRule::Winding, Transform::identity(), None);
            }

            self.glass_backdrop_pm = Some(bake_liquid_glass(&bg));
            self.wallpaper_pm = Some(bg);
        }

        if let Some(mut fg) = Pixmap::new(w, h) {
            let mut near_pb = PathBuilder::new();
            near_pb.move_to(0.0, h as f32);
            near_pb.line_to(0.0, 365.0);
            near_pb.line_to(195.0, 302.0);
            near_pb.line_to(375.0, 344.0);
            near_pb.line_to(545.0, 286.0);
            near_pb.line_to(745.0, 336.0);
            near_pb.line_to(960.0, 296.0);
            near_pb.line_to(w as f32, h as f32);
            near_pb.close();
            if let Some(path) = near_pb.finish() {
                let mut np = Paint::default();
                np.anti_alias = true;
                np.set_color(Color::from_rgba8(spec.ridge_near[0], spec.ridge_near[1], spec.ridge_near[2], 255));
                fg.fill_path(&path, &np, FillRule::Winding, Transform::identity(), None);
            }
            self.foreground_pm = Some(fg);
        }
    }

    pub fn invalidate_scene_cache(&mut self) {
        self.wallpaper_pm = None;
        self.foreground_pm = None;
        self.glass_backdrop_pm = None;
    }

    pub fn on_home_pressed(&mut self) {
        self.settings.open = false;
        self.control_center_open = false;
        self.recent_apps_open = false;
        self.ambient_mode_open = false;
        self.weather_popup_open = false;
        self.app_menu.open = false;
        self.search_ui.open = false;
        self.focus_zone = FocusZone::Dock;
        if self.selected_index >= self.apps.len() {
            self.selected_index = 0;
        }
    }

    pub fn open_control_center(&mut self) {
        self.settings.open = false;
        self.recent_apps_open = false;
        self.weather_popup_open = false;
        self.ambient_mode_open = false;
        self.control_center_open = true;
        self.cc_selected = 0;
    }

    pub fn build_settings_rows(&self, last_key_label: &str, bound_count: usize) -> Vec<SettingRowItem> {
        match self.settings.current_category() {
            SettingsCategory::Appearance => vec![
                SettingRowItem {
                    section: "LOOK",
                    title: "Night scenes".into(),
                    subtitle: Some("A big glass clock, scenes taking turns".into()),
                    value: String::new(),
                    checked: Some(!self.classic_look),
                    action_id: "look_scenes",
                },
                SettingRowItem {
                    section: "LOOK",
                    title: "Classic".into(),
                    subtitle: Some("Top-right crisp clock and streamlined dock".into()),
                    value: String::new(),
                    checked: Some(self.classic_look),
                    action_id: "look_classic",
                },
                SettingRowItem {
                    section: "SCENES & CLOCK",
                    title: "Active background scene".into(),
                    subtitle: None,
                    value: self.current_scene().label.into(),
                    checked: None,
                    action_id: "cycle_scene",
                },
                SettingRowItem {
                    section: "SCENES & CLOCK",
                    title: "Clock display".into(),
                    subtitle: None,
                    value: match self.clock_style {
                        0 => "Big Glass (1:1)".into(),
                        1 => "Top-Right Crisp".into(),
                        _ => "Hidden".into(),
                    },
                    checked: None,
                    action_id: "cycle_clock_style",
                },
                SettingRowItem {
                    section: "SCENES & CLOCK",
                    title: "Clock format".into(),
                    subtitle: None,
                    value: if self.clock_24h { "24 hour".into() } else { "12 hour".into() },
                    checked: None,
                    action_id: "toggle_clock_24h",
                },
            ],
            SettingsCategory::Weather => vec![
                SettingRowItem {
                    section: "WEATHER & LOCATION",
                    title: "Show weather on Home".into(),
                    subtitle: Some("Displays live temperature next to the Home Screen date".into()),
                    value: if self.weather_enabled { "On".into() } else { "Off".into() },
                    checked: None,
                    action_id: "toggle_weather_enabled",
                },
                SettingRowItem {
                    section: "LOCATION MODE",
                    title: "Use My Location (Automatic)".into(),
                    subtitle: Some(if self.weather_auto && !self.weather_city.is_empty() {
                        format!("Detected: {} · {}", self.weather_city, self.weather_temp)
                    } else {
                        "Auto-detect city from network & fetch live weather".into()
                    }),
                    value: String::new(),
                    checked: Some(self.weather_enabled && self.weather_auto),
                    action_id: "weather_mode_auto",
                },
                SettingRowItem {
                    section: "LOCATION MODE",
                    title: "Choose a City Manually...".into(),
                    subtitle: Some(if !self.weather_auto && !self.weather_city.is_empty() {
                        format!("Manual city: {} · {}", self.weather_city, self.weather_temp)
                    } else {
                        "Enter any city name manually".into()
                    }),
                    value: "Edit ›".into(),
                    checked: Some(self.weather_enabled && !self.weather_auto),
                    action_id: "weather_mode_manual",
                },
                SettingRowItem {
                    section: "WEATHER CARD",
                    title: "Open Weather card".into(),
                    subtitle: None,
                    value: "View ›".into(),
                    checked: None,
                    action_id: "open_weather_card",
                },
            ],
            SettingsCategory::Apps => {
                let mut list = vec![
                    SettingRowItem {
                        section: "DOCK & APPS",
                        title: "Long-press OK on any dock app".into(),
                        subtitle: Some("Rename (recognized by Voice Assistant), Move, Hide, or App Details".into()),
                        value: format!("{} visible", self.apps.len().saturating_sub(1)),
                        checked: None,
                        action_id: "noop",
                    },
                    SettingRowItem {
                        section: "DOCK & APPS",
                        title: "Open Recent Apps Switcher".into(),
                        subtitle: Some("View running apps and free RAM".into()),
                        value: "Open ›".into(),
                        checked: None,
                        action_id: "open_recents",
                    },
                ];
                for (i, (pkg, lbl)) in self.hidden_apps.iter().enumerate() {
                    list.push(SettingRowItem {
                        section: "HIDDEN APPS (OK TO UNHIDE)",
                        title: format!("Unhide {}", lbl),
                        subtitle: Some(pkg.clone()),
                        value: "Restore ›".into(),
                        checked: None,
                        action_id: match i {
                            0 => "unhide:0",
                            1 => "unhide:1",
                            2 => "unhide:2",
                            3 => "unhide:3",
                            _ => "unhide:4",
                        },
                    });
                }
                list
            }
            SettingsCategory::People => vec![
                SettingRowItem {
                    section: "PROFILE",
                    title: "Edit profile name...".into(),
                    subtitle: Some("Shown in the greeting above the Home clock".into()),
                    value: self.user_name.clone(),
                    checked: None,
                    action_id: "edit_profile_name",
                },
            ],
            SettingsCategory::ControlPanel => vec![
                SettingRowItem {
                    section: "CONTROL CENTER",
                    title: "Open Control Center overlay".into(),
                    subtitle: Some("Wi-Fi, Bluetooth, Memory, DND, Audio, Sleep, Ambient, Power".into()),
                    value: "Open ›".into(),
                    checked: None,
                    action_id: "open_cc",
                },
                SettingRowItem {
                    section: "SHORTCUTS",
                    title: "Wi-Fi & Network settings".into(),
                    subtitle: None,
                    value: self.sys.wifi_label.clone(),
                    checked: None,
                    action_id: "open_wifi",
                },
                SettingRowItem {
                    section: "SHORTCUTS",
                    title: "Bluetooth settings".into(),
                    subtitle: None,
                    value: if self.sys.bt_on { "On".into() } else { "Off".into() },
                    checked: None,
                    action_id: "open_bt",
                },
                SettingRowItem {
                    section: "SHORTCUTS",
                    title: "Sleep timer".into(),
                    subtitle: None,
                    value: if self.sleep_timer_mins == 0 {
                        "Off".into()
                    } else {
                        format!("{} min", self.sleep_timer_mins)
                    },
                    checked: None,
                    action_id: "cycle_sleep",
                },
            ],
            SettingsCategory::ButtonMapper => vec![
                SettingRowItem {
                    section: "VOICE ASSISTANT KEYBIND",
                    title: "Set Voice Assistant button…".into(),
                    subtitle: Some("Press OK, then press the remote button you want for Voice Assistant".into()),
                    value: last_key_label.into(),
                    checked: None,
                    action_id: "capture_voice_key",
                },
                SettingRowItem {
                    section: "ACCESSIBILITY SERVICE",
                    title: "Button Mapper & Voice accessibility service".into(),
                    subtitle: Some("Enables Voice Assistant & in-app search over YouTube, Browser & all apps".into()),
                    value: if self.sys.a11y_enabled {
                        "Active".into()
                    } else {
                        "Enable in Accessibility ›".into()
                    },
                    checked: None,
                    action_id: "open_a11y",
                },
                SettingRowItem {
                    section: "CONTROL CENTER KEYBIND",
                    title: "Bind button to Control Center…".into(),
                    subtitle: Some("Press OK, then press any remote button to open Control Center".into()),
                    value: format!("{} mapped", bound_count),
                    checked: None,
                    action_id: "capture_cc_key",
                },
            ],
            SettingsCategory::QuickSettings => vec![
                SettingRowItem {
                    section: "HOME & SETUP",
                    title: "Run first-time setup again...".into(),
                    subtitle: Some("Configure language, profile name, weather location, appearance & keybinds".into()),
                    value: "Start ›".into(),
                    checked: None,
                    action_id: "run_setup_again",
                },
                SettingRowItem {
                    section: "HOME & SETUP",
                    title: "Default home screen".into(),
                    subtitle: None,
                    value: if self.sys.default_home {
                        "Active".into()
                    } else {
                        "Set as default ›".into()
                    },
                    checked: None,
                    action_id: "open_home_picker",
                },
                SettingRowItem {
                    section: "HOME & SETUP",
                    title: "Open Android TV Settings...".into(),
                    subtitle: None,
                    value: "›".into(),
                    checked: None,
                    action_id: "open_android_settings",
                },
            ],
            SettingsCategory::Language => {
                let mut rows = vec![
                    SettingRowItem {
                        section: "LANGUAGE REGION MENU",
                        title: "Native (India)".into(),
                        subtitle: Some("English (Native) + Indian regional languages".into()),
                        value: if !self.lang_region_global { "Selected".into() } else { "Switch ›".into() },
                        checked: Some(!self.lang_region_global),
                        action_id: "lang_region_native",
                    },
                    SettingRowItem {
                        section: "LANGUAGE REGION MENU",
                        title: "Global".into(),
                        subtitle: Some("International languages worldwide".into()),
                        value: if self.lang_region_global { "Selected".into() } else { "Switch ›".into() },
                        checked: Some(self.lang_region_global),
                        action_id: "lang_region_global",
                    },
                ];
                let sec = if self.lang_region_global {
                    "GLOBAL LANGUAGES"
                } else {
                    "NATIVE LANGUAGES (INDIA)"
                };
                for lang in ALL_LANGUAGES
                    .iter()
                    .filter(|l| l.is_native_india != self.lang_region_global)
                {
                    rows.push(SettingRowItem {
                        section: sec,
                        title: lang.title.into(),
                        subtitle: Some(lang.subtitle.into()),
                        value: String::new(),
                        checked: Some(self.lang_code == lang.code),
                        action_id: lang.action_id,
                    });
                }
                rows
            }
            SettingsCategory::Advanced => vec![
                SettingRowItem {
                    section: "ADB & SECURITY MODE",
                    title: "ADB operating mode".into(),
                    subtitle: Some(if self.sys.adb_mode_secure {
                        "Secure Mode — Grant WRITE_SECURE_SETTINGS & Close ADB".into()
                    } else {
                        "Persistent ADB Mode — Keep Network ADB Open (5555)".into()
                    }),
                    value: if self.sys.adb_mode_secure {
                        "Secure".into()
                    } else {
                        "Persistent".into()
                    },
                    checked: None,
                    action_id: "toggle_adb_mode",
                },
                SettingRowItem {
                    section: "ADB & SECURITY MODE",
                    title: "ADB connection status".into(),
                    subtitle: Some("Local ADB loopback check on 127.0.0.1:5555".into()),
                    value: if self.sys.adb_port_open {
                        "Connected (5555)".into()
                    } else if self.sys.write_secure {
                        "Secure Mode Ready".into()
                    } else {
                        "Port 5555 Closed".into()
                    },
                    checked: None,
                    action_id: "refresh_sys",
                },
                SettingRowItem {
                    section: "ADB & SECURITY MODE",
                    title: "Pair / Repair ADB connection".into(),
                    subtitle: Some("Auto-grant WRITE_SECURE_SETTINGS & DUMP via 127.0.0.1:5555".into()),
                    value: "Pair now ›".into(),
                    checked: None,
                    action_id: "adb_pair",
                },
                SettingRowItem {
                    section: "ADB & SECURITY MODE",
                    title: "Turn On / Off ADB".into(),
                    subtitle: None,
                    value: if self.sys.adb_port_open { "On".into() } else { "Off".into() },
                    checked: None,
                    action_id: "toggle_adb_port",
                },
                SettingRowItem {
                    section: "ADB & SECURITY MODE",
                    title: "Check permissions".into(),
                    subtitle: Some("Default Home · Accessibility · Secure Settings".into()),
                    value: format!(
                        "{} / 3 active",
                        (self.sys.default_home as u8)
                            + (self.sys.a11y_enabled as u8)
                            + (self.sys.write_secure as u8)
                    ),
                    checked: None,
                    action_id: "refresh_sys",
                },
                SettingRowItem {
                    section: "STARTUP & KEEP ALIVE",
                    title: "Keep Alive (Auto-repair Accessibility)".into(),
                    subtitle: None,
                    value: if self.keep_alive_enabled {
                        "On (Default)".into()
                    } else {
                        "Off".into()
                    },
                    checked: None,
                    action_id: "toggle_keep_alive",
                },
                SettingRowItem {
                    section: "STARTUP & KEEP ALIVE",
                    title: "Caffeine Mode".into(),
                    subtitle: Some("Keep TV screen awake and block idle sleep".into()),
                    value: if self.caffeine_enabled { "On".into() } else { "Off".into() },
                    checked: None,
                    action_id: "toggle_caffeine",
                },
            ],
            SettingsCategory::Updates => vec![
                SettingRowItem {
                    section: "UPDATES",
                    title: "Automatic updates".into(),
                    subtitle: None,
                    value: if self.auto_updates { "On".into() } else { "Off".into() },
                    checked: None,
                    action_id: "toggle_auto_updates",
                },
                SettingRowItem {
                    section: "VERSION INFO",
                    title: "Repository".into(),
                    subtitle: Some("eadxmz/FerroDeck (Rust + tiny-skia + ANativeWindow)".into()),
                    value: "v0.3.0".into(),
                    checked: None,
                    action_id: "noop",
                },
            ],
            SettingsCategory::About => vec![
                SettingRowItem {
                    section: "APPLICATION",
                    title: "Home screen".into(),
                    subtitle: None,
                    value: "FerroDeck".into(),
                    checked: None,
                    action_id: "noop",
                },
                SettingRowItem {
                    section: "APPLICATION",
                    title: "Developer".into(),
                    subtitle: None,
                    value: "eadxmz".into(),
                    checked: None,
                    action_id: "noop",
                },
                SettingRowItem {
                    section: "APPLICATION",
                    title: "Engine".into(),
                    subtitle: Some("Zero-View Rust NativeActivity · 0.0% idle CPU".into()),
                    value: "v0.3.0".into(),
                    checked: None,
                    action_id: "noop",
                },
                SettingRowItem {
                    section: "RESET",
                    title: "Reset settings to defaults".into(),
                    subtitle: None,
                    value: "Reset ›".into(),
                    checked: None,
                    action_id: "reset_defaults",
                },
            ],
        }
    }

    pub fn execute_setting_action(&mut self, action_id: &str) -> (bool, SideEffect, bool) {
        match action_id {
            "look_scenes" => {
                self.classic_look = false;
                self.clock_style = 0;
                (false, SideEffect::None, false)
            }
            "look_classic" => {
                self.classic_look = true;
                self.clock_style = 1;
                (false, SideEffect::None, false)
            }
            "cycle_scene" => {
                self.scene_idx = (self.scene_idx + 1) % SCENES.len();
                self.invalidate_scene_cache();
                (true, SideEffect::None, false)
            }
            "cycle_clock_style" => {
                self.clock_style = (self.clock_style + 1) % 3;
                (false, SideEffect::None, false)
            }
            "toggle_clock_24h" => {
                self.clock_24h = !self.clock_24h;
                (false, SideEffect::None, false)
            }
            "open_weather_card" => {
                self.settings.open = false;
                self.weather_popup_open = true;
                (false, SideEffect::None, false)
            }
            "open_recents" => {
                self.settings.open = false;
                self.recent_apps_open = true;
                self.recent_selected = 0;
                (false, SideEffect::RefreshSystemSnapshot, false)
            }
            "edit_profile_name" => (false, SideEffect::PromptProfileName, false),
            "toggle_weather_enabled" => {
                self.weather_enabled = !self.weather_enabled;
                (false, SideEffect::SavePrefs, false)
            }
            "weather_mode_auto" => {
                self.weather_enabled = true;
                self.weather_auto = true;
                self.settings.status_banner = Some("Detecting your location & live weather...".into());
                (false, SideEffect::FetchWeatherNow, false)
            }
            "weather_mode_manual" => {
                self.weather_enabled = true;
                self.weather_auto = false;
                (false, SideEffect::PromptWeatherCity, false)
            }
            "run_setup_again" => {
                self.settings.open = false;
                self.setup_done = false;
                self.settings.status_banner = None;
                self.setup.open_fresh();
                (false, SideEffect::SavePrefs, false)
            }
            "lang_region_native" => {
                self.lang_region_global = false;
                self.settings.row_idx = 0;
                (false, SideEffect::None, false)
            }
            "lang_region_global" => {
                self.lang_region_global = true;
                self.settings.row_idx = 1;
                (false, SideEffect::None, false)
            }
            "open_cc" => {
                self.open_control_center();
                (false, SideEffect::RefreshSystemSnapshot, false)
            }
            "open_wifi" => (false, SideEffect::OpenWifiSettings, false),
            "open_bt" => (false, SideEffect::OpenBluetoothSettings, false),
            "cycle_sleep" => {
                self.sleep_timer_mins = match self.sleep_timer_mins {
                    0 => 15,
                    15 => 30,
                    30 => 60,
                    _ => 0,
                };
                (false, SideEffect::None, false)
            }
            "open_a11y" => (false, SideEffect::OpenAccessibilitySettings, false),
            "capture_key" => (false, SideEffect::None, true),
            "open_home_picker" => (false, SideEffect::OpenDefaultHomePicker, false),
            "open_android_settings" => (false, SideEffect::OpenAndroidSettings, false),
            id if id.starts_with("set_lang:") => {
                let code = &id["set_lang:".len()..];
                if let Some(entry) = ALL_LANGUAGES.iter().find(|l| l.code == code) {
                    self.lang_code = entry.code;
                    let group = if entry.is_native_india { "Native · India" } else { "Global" };
                    self.settings.status_banner = Some(format!("Language: {} ({})", entry.title, group));
                }
                (false, SideEffect::SavePrefs, false)
            }
            "toggle_adb_mode" => {
                self.sys.adb_mode_secure = !self.sys.adb_mode_secure;
                self.settings.status_banner = Some(if self.sys.adb_mode_secure {
                    "Secure Mode selected — grants permissions & closes port 5555.".into()
                } else {
                    "Persistent ADB Mode selected — keeps port 5555 open.".into()
                });
                (false, SideEffect::None, false)
            }
            "adb_pair" => {
                self.settings.status_banner = Some("Connecting to local ADB 127.0.0.1:5555…".into());
                (false, SideEffect::RunAdbPairAndGrant, false)
            }
            "toggle_adb_port" => (false, SideEffect::ToggleAdbPort, false),
            "refresh_sys" => (false, SideEffect::RefreshSystemSnapshot, false),
            "toggle_keep_alive" => {
                self.keep_alive_enabled = !self.keep_alive_enabled;
                (false, SideEffect::None, false)
            }
            "toggle_caffeine" => {
                self.caffeine_enabled = !self.caffeine_enabled;
                (false, SideEffect::SetKeepScreenOn(self.caffeine_enabled), false)
            }
            "toggle_auto_updates" => {
                self.auto_updates = !self.auto_updates;
                (false, SideEffect::None, false)
            }
            "reset_defaults" => {
                self.classic_look = false;
                self.clock_style = 0;
                self.clock_24h = false;
                self.scene_idx = 0;
                self.invalidate_scene_cache();
                self.settings.status_banner = Some("Settings reset to defaults.".into());
                (true, SideEffect::None, false)
            }
            _ => (false, SideEffect::None, false),
        }
    }

    pub fn move_dpad(&mut self, dx: isize, dy: isize, row_count: usize) {
        if self.app_menu.open {
            let n = self.app_menu_items().len();
            if dy < 0 {
                self.app_menu.selected = if self.app_menu.selected == 0 {
                    n - 1
                } else {
                    self.app_menu.selected - 1
                };
            } else if dy > 0 {
                self.app_menu.selected = (self.app_menu.selected + 1) % n;
            }
            return;
        }

        if self.search_ui.open {
            let res_len = self.search_matching_apps().len().min(7);
            if dy > 0 && self.search_ui.focus_row == 0 && res_len > 0 {
                self.search_ui.focus_row = 1;
                self.search_ui.result_idx = 0;
            } else if dy < 0 && self.search_ui.focus_row == 1 {
                if self.search_ui.result_idx > 0 {
                    self.search_ui.result_idx -= 1;
                } else {
                    self.search_ui.focus_row = 0;
                }
            } else if dy > 0 && self.search_ui.focus_row == 1 && res_len > 0 {
                self.search_ui.result_idx = (self.search_ui.result_idx + 1) % res_len;
            } else if dx != 0 && self.search_ui.focus_row == 0 {
                let klen = SEARCH_KEYS.len() as isize;
                self.search_ui.key_idx =
                    (self.search_ui.key_idx as isize + dx).rem_euclid(klen) as usize;
            }
            return;
        }

        if self.ambient_mode_open {
            if dx != 0 {
                self.scene_idx =
                    (self.scene_idx as isize + if dx > 0 { 1 } else { -1 }).rem_euclid(SCENES.len() as isize) as usize;
                self.invalidate_scene_cache();
            }
            return;
        }

        if self.settings.open {
            if dy < 0 {
                self.settings.move_up(row_count);
            } else if dy > 0 {
                self.settings.move_down(row_count);
            } else if dx > 0 {
                self.settings.move_right(row_count);
            } else if dx < 0 {
                self.settings.move_left();
            }
            return;
        }

        if self.control_center_open {
            // 0..8 = 2x4 grid of stadium pills; 8..12 = 4 squircle buttons on bottom row
            let idx = self.cc_selected;
            if idx < 8 {
                let row = idx / 2;
                let col = idx % 2;
                if dx != 0 {
                    self.cc_selected = row * 2 + (1 - col);
                } else if dy < 0 {
                    if row > 0 {
                        self.cc_selected = (row - 1) * 2 + col;
                    } else {
                        self.cc_selected = 8 + col * 2;
                    }
                } else if dy > 0 {
                    if row < 3 {
                        self.cc_selected = (row + 1) * 2 + col;
                    } else {
                        self.cc_selected = 8 + col * 2;
                    }
                }
            } else {
                let bcol = idx - 8;
                if dx < 0 {
                    self.cc_selected = 8 + (bcol + 3) % 4;
                } else if dx > 0 {
                    self.cc_selected = 8 + (bcol + 1) % 4;
                } else if dy < 0 {
                    self.cc_selected = 6 + (bcol / 2);
                } else if dy > 0 {
                    self.cc_selected = bcol / 2;
                }
            }
            return;
        }

        if self.recent_apps_open {
            let max_items = self.apps.len().saturating_sub(1).max(1);
            if dx != 0 {
                self.recent_selected =
                    (self.recent_selected as isize + dx).rem_euclid(max_items as isize) as usize;
            }
            return;
        }

        if self.weather_popup_open {
            if dx != 0 {
                self.scene_idx =
                    (self.scene_idx as isize + if dx > 0 { 1 } else { -1 }).rem_euclid(SCENES.len() as isize) as usize;
                self.invalidate_scene_cache();
            }
            return;
        }

        // Home screen D-Pad: Up/Down moves between Dock and Top-Right pills
        if dy < 0 && self.focus_zone == FocusZone::Dock {
            self.focus_zone = FocusZone::TopControlCenter;
            return;
        }
        if dy > 0 && self.focus_zone != FocusZone::Dock {
            self.focus_zone = FocusZone::Dock;
            return;
        }

        if self.focus_zone != FocusZone::Dock {
            if dx != 0 {
                self.focus_zone = match self.focus_zone {
                    FocusZone::TopWeather => FocusZone::TopControlCenter,
                    FocusZone::TopControlCenter => FocusZone::TopWeather,
                    FocusZone::Dock => FocusZone::Dock,
                };
            }
            return;
        }

        let len = self.apps.len();
        if len == 0 {
            return;
        }
        if dx != 0 {
            let next = (self.selected_index as isize + dx).rem_euclid(len as isize) as usize;
            self.selected_index = next;
            if self.selected_index < self.page_offset {
                self.page_offset = self.selected_index;
            } else if self.selected_index >= self.page_offset + MAX_DOCK_TILES {
                self.page_offset = self.selected_index + 1 - MAX_DOCK_TILES;
            }
        }
    }

    pub fn activate_control_center_item(&mut self) -> (SideEffect, bool) {
        match self.cc_selected {
            0 => {
                // Wi-Fi
                self.control_center_open = false;
                (SideEffect::OpenWifiSettings, false)
            }
            1 => {
                // Bluetooth
                self.control_center_open = false;
                (SideEffect::OpenBluetoothSettings, false)
            }
            2 => {
                // Memory (Clean RAM)
                (SideEffect::CleanMemoryNow, false)
            }
            3 => {
                // Do Not Disturb
                self.sys.dnd_on = !self.sys.dnd_on;
                (SideEffect::None, false)
            }
            4 => {
                // Audio
                self.control_center_open = false;
                (SideEffect::OpenSoundSettings, false)
            }
            5 => {
                // Sleep Timer
                self.sleep_timer_mins = match self.sleep_timer_mins {
                    0 => 15,
                    15 => 30,
                    30 => 60,
                    _ => 0,
                };
                (SideEffect::None, false)
            }
            6 => {
                // Ambient Mode (Screensaver)
                self.control_center_open = false;
                self.ambient_mode_open = true;
                (SideEffect::None, false)
            }
            7 => {
                // Power Off / Screen Off
                self.control_center_open = false;
                (SideEffect::LockScreenNow, false)
            }
            8 => {
                // Inputs squircle
                self.control_center_open = false;
                (SideEffect::OpenInputsOrTvSettings, false)
            }
            9 => {
                // Button Mapper squircle
                self.control_center_open = false;
                self.settings.open_at(SettingsCategory::ButtonMapper);
                (SideEffect::None, false)
            }
            10 => {
                // Home Settings gear squircle
                self.control_center_open = false;
                self.settings.open_at(SettingsCategory::Appearance);
                (SideEffect::RefreshSystemSnapshot, false)
            }
            11 => {
                // Native Search Overlay (apps, YouTube/Browser search, or ask a question)
                self.control_center_open = false;
                self.search_ui.open = true;
                self.search_ui.query.clear();
                self.search_ui.focus_row = 0;
                self.search_ui.key_idx = 0;
                self.search_ui.result_idx = 0;
                (SideEffect::None, false)
            }
            _ => (SideEffect::None, false),
        }
    }

    pub fn handle_tap(&mut self, px: f32, py: f32, sw: f32, sh: f32) -> (Option<String>, SideEffect) {
        let unit = (sw / DESIGN_W).min(sh / DESIGN_H);

        if self.ambient_mode_open {
            self.ambient_mode_open = false;
            return (None, SideEffect::None);
        }

        if self.weather_popup_open {
            self.weather_popup_open = false;
            return (None, SideEffect::None);
        }

        if self.recent_apps_open {
            self.recent_apps_open = false;
            return (None, SideEffect::None);
        }

        if self.search_ui.open {
            let margin = (48.0 * unit).round();
            let box_w = sw - margin * 2.0;
            let key_y = (94.0 * unit).round();
            let key_h = (30.0 * unit).round();
            let key_gap = (4.0 * unit).round();
            let total_keys = SEARCH_KEYS.len();
            let visible_keys = 22usize;
            let k_start = if self.search_ui.key_idx >= visible_keys {
                self.search_ui.key_idx + 1 - visible_keys
            } else {
                0
            };
            if py >= key_y && py <= key_y + key_h {
                let mut kx = margin;
                for idx in k_start..(k_start + visible_keys).min(total_keys) {
                    let lbl = SEARCH_KEYS[idx];
                    let kw = if lbl.len() > 1 { (44.0 * unit).round() } else { (30.0 * unit).round() };
                    if px >= kx && px <= kx + kw {
                        self.search_ui.focus_row = 0;
                        self.search_ui.key_idx = idx;
                        match lbl {
                            "SPACE" => self.search_ui.query.push(' '),
                            "DEL" => { self.search_ui.query.pop(); }
                            "MIC" => return (None, SideEffect::TriggerVoiceAssistant),
                            ch => self.search_ui.query.push_str(&ch.to_lowercase()),
                        }
                        return (None, SideEffect::None);
                    }
                    kx += kw + key_gap;
                }
            }
            let results = self.search_matching_apps();
            let res_top = (146.0 * unit).round();
            let row_h = (38.0 * unit).round();
            let row_gap = (8.0 * unit).round();
            if px >= margin && px <= margin + box_w {
                for (i, (pkg, _lbl, is_action)) in results.iter().take(7).enumerate() {
                    let ry = res_top + (i as f32) * (row_h + row_gap);
                    if py >= ry && py <= ry + row_h {
                        self.search_ui.focus_row = 1;
                        self.search_ui.result_idx = i;
                        if !*is_action {
                            self.search_ui.open = false;
                            return (Some(pkg.clone()), SideEffect::None);
                        }
                        break;
                    }
                }
            }
            if py < (30.0 * unit) {
                self.search_ui.open = false;
            }
            return (None, SideEffect::None);
        }

        if self.control_center_open {
            // Check right-aligned Control Center panel geometry (matches draw_control_center)
            let pill_w = (152.0 * unit).round();
            let pill_h = (52.0 * unit).round();
            let col_gap = (12.0 * unit).round();
            let row_gap = (12.0 * unit).round();
            let grid_w = pill_w * 2.0 + col_gap;
            let grid_x = (sw - 34.0 * unit - grid_w).round();
            let grid_y = (76.0 * unit).round();

            for i in 0..8usize {
                let r = (i / 2) as f32;
                let c = (i % 2) as f32;
                let x = grid_x + c * (pill_w + col_gap);
                let y = grid_y + r * (pill_h + row_gap);
                if px >= x && px <= x + pill_w && py >= y && py <= y + pill_h {
                    self.cc_selected = i;
                    let (eff, _) = self.activate_control_center_item();
                    return (None, eff);
                }
            }

            let sq_y = grid_y + 4.0 * (pill_h + row_gap);
            let sq_gap = (12.0 * unit).round();
            let sq_size = ((grid_w - sq_gap * 3.0) / 4.0).round();
            for b in 0..4usize {
                let x = grid_x + (b as f32) * (sq_size + sq_gap);
                if px >= x && px <= x + sq_size && py >= sq_y && py <= sq_y + sq_size {
                    self.cc_selected = 8 + b;
                    let (eff, _) = self.activate_control_center_item();
                    return (None, eff);
                }
            }

            // Tap outside dismisses Control Center
            self.control_center_open = false;
            return (None, SideEffect::None);
        }

        if self.settings.open {
            // Left sidebar taps
            let side_x = (46.0 * unit).round();
            let side_w = (268.0 * unit).round();
            let pill_h = (31.0 * unit).round();
            if px >= side_x && px <= side_x + side_w {
                for i in 0..SettingsCategory::ALL.len() {
                    let ry = ((74.0 + (i as f32) * 35.0) * unit).round();
                    if py >= ry && py <= ry + pill_h {
                        self.settings.category_idx = i;
                        self.settings.row_idx = 0;
                        self.settings.in_right_pane = false;
                        return (None, SideEffect::None);
                    }
                }
            }

            // Right pane row taps (matches inline section headers + pills)
            let main_x = (348.0 * unit).round();
            let main_w = (sw - main_x - 52.0 * unit).round();
            let rows = self.build_settings_rows("", 0);
            let layouts = self.settings.compute_visible_layout(&rows, unit);
            if px >= main_x && px <= main_x + main_w {
                for lay in layouts {
                    if py >= lay.pill_y && py <= lay.pill_y + lay.pill_h {
                        self.settings.in_right_pane = true;
                        self.settings.row_idx = lay.abs_idx;
                        let (_, eff, _) = self.execute_setting_action(rows[lay.abs_idx].action_id);
                        return (None, eff);
                    }
                }
            }
            return (None, SideEffect::None);
        }

        // Top-Right Weather & Control Center circular pills
        let size = (38.0 * unit).round();
        let top_y = (18.0 * unit).round();
        let right_margin = (22.0 * unit).round();
        let gap = (10.0 * unit).round();
        let gear_x = sw - right_margin - size;
        let weather_x = gear_x - gap - size;

        if py >= top_y - 8.0 * unit && py <= top_y + size + 8.0 * unit {
            if px >= weather_x - 6.0 * unit && px <= weather_x + size + 6.0 * unit {
                self.focus_zone = FocusZone::TopWeather;
                self.weather_popup_open = !self.weather_popup_open;
                return (None, SideEffect::None);
            }
            if px >= gear_x - 6.0 * unit && px <= gear_x + size + 6.0 * unit {
                self.focus_zone = FocusZone::TopControlCenter;
                self.open_control_center();
                return (None, SideEffect::RefreshSystemSnapshot);
            }
        }

        // Dock Icons
        if !self.apps.is_empty() {
            let end = (self.page_offset + MAX_DOCK_TILES).min(self.apps.len());
            let count = end - self.page_offset;
            let icon_size = (46.0 * unit).round();
            let icon_gap = (10.0 * unit).round();
            let pad_h = (13.0 * unit).round();
            let pad_v = (10.0 * unit).round();
            let dock_w = pad_h * 2.0 + (count as f32) * icon_size + ((count.saturating_sub(1)) as f32) * icon_gap;
            let dock_h = pad_v * 2.0 + icon_size;
            let dock_x = ((sw - dock_w) * 0.5).round();
            let dock_y = (sh - (14.0 * unit).round() - dock_h).round();

            if py >= dock_y && py <= dock_y + dock_h && px >= dock_x && px <= dock_x + dock_w {
                self.focus_zone = FocusZone::Dock;
                let rel = ((px - (dock_x + pad_h)) / (icon_size + icon_gap)).floor() as isize;
                if rel >= 0 && (rel as usize) < count {
                    let target_idx = self.page_offset + (rel as usize);
                    self.selected_index = target_idx;
                    if let Some(tile) = self.apps.get(target_idx) {
                        if tile.package_name == SETTINGS_TILE_PKG {
                            self.settings.open_at(SettingsCategory::Appearance);
                            return (None, SideEffect::RefreshSystemSnapshot);
                        }
                        return (Some(tile.package_name.clone()), SideEffect::None);
                    }
                }
            }
        }

        (None, SideEffect::None)
    }

    pub fn tick_animations(&mut self) -> bool {
        let mut busy = self.settings.tick();
        if self.focus_scales.len() != self.apps.len() {
            self.focus_scales.resize(self.apps.len(), 1.0);
        }
        for (i, sc) in self.focus_scales.iter_mut().enumerate() {
            let target = if self.focus_zone == FocusZone::Dock
                && i == self.selected_index
                && !self.settings.open
                && !self.control_center_open
            {
                1.15
            } else {
                1.0
            };
            let diff = target - *sc;
            if diff.abs() > 0.004 {
                *sc += diff * 0.34;
                busy = true;
            } else {
                *sc = target;
            }
        }
        busy
    }

    pub fn render(
        &mut self,
        pm: &mut Pixmap,
        time_str: &str,
        greeting_str: &str,
        date_str: &str,
        last_key_label: &str,
        bound_count: usize,
    ) {
        self.ensure_procedural_scene();

        let sw = pm.width() as f32;
        let sh = pm.height() as f32;
        let unit = (sw / DESIGN_W).min(sh / DESIGN_H);
        let scene = self.current_scene();

        // 1. Scenic background
        if let Some(ref wp) = self.wallpaper_pm {
            draw_cover_pixmap(pm, wp);
        } else {
            pm.fill(Color::from_rgba8(14, 18, 32, 255));
        }

        // 2. Giant 1:1 Crisp Glass Clock (drawn BEFORE foreground mountain cutout so peaks overlap)
        match self.clock_style {
            0 => {
                let top_offset_u = 14.0;
                let clock_px = (scene.clock_size_u * unit).round();
                let clock_top = ((scene.clock_top_u + top_offset_u) * unit).round() as i32;
                self.clock
                    .draw_centered(pm, time_str, 0, sw * 0.5, clock_top, clock_px);
            }
            1 => {
                let clock_px = (24.0 * unit).round();
                let right_cx = sw - (155.0 * unit);
                self.clock
                    .draw_centered(pm, time_str, 1, right_cx, (22.0 * unit).round() as i32, clock_px);
            }
            _ => {}
        }

        // 3. Foreground mountain cutout overlapping lower clock digits
        if let Some(ref fg) = self.foreground_pm {
            draw_cover_pixmap(pm, fg);
        }

        // 4. Greeting & Date Header above the clock
        let date_y = ((scene.date_top_u + 14.0) * unit).round();
        let greet_y = date_y - (17.5 * unit).round();
        let greet_px = (12.5 * unit).round();
        let date_px = (13.5 * unit).round();

        let greet_w = measure_label_width(self.font_medium.as_ref(), greeting_str, greet_px);
        draw_crisp_label(
            pm,
            self.font_medium.as_ref(),
            greeting_str,
            ((sw - greet_w) * 0.5).round() as i32,
            greet_y as i32,
            greet_px,
            Color::from_rgba8(245, 247, 255, 235),
            true,
        );

        let full_date_line = if self.weather_enabled && !self.weather_temp.is_empty() {
            if self.weather_desc.is_empty() {
                format!("{}  ·  {}", date_str, self.weather_temp)
            } else {
                format!("{}  ·  {} {}", date_str, self.weather_temp, self.weather_desc)
            }
        } else {
            date_str.to_string()
        };
        let date_w = measure_label_width(self.font_medium.as_ref(), &full_date_line, date_px);
        draw_crisp_label(
            pm,
            self.font_medium.as_ref(),
            &full_date_line,
            ((sw - date_w) * 0.5).round() as i32,
            date_y as i32,
            date_px,
            Color::from_rgba8(255, 255, 255, 245),
            true,
        );

        if self.ambient_mode_open {
            let hint = "Ambient Screensaver  ·  Left/Right: Switch Scene  ·  Back: Exit";
            let hpx = (11.0 * unit).round();
            let hw = measure_label_width(self.font_medium.as_ref(), hint, hpx);
            draw_crisp_label(
                pm,
                self.font_medium.as_ref(),
                hint,
                ((sw - hw) * 0.5).round() as i32,
                (sh - 24.0 * unit) as i32,
                hpx,
                Color::from_rgba8(230, 236, 248, 210),
                true,
            );
            return;
        }

        // 5. Top-right circular frosted glass Weather & Control Center pills
        self.draw_top_right_pills(pm, sw, unit);

        // 6. Centered Floating Liquid Glass Squircle Dock + Floating Tooltip Pill
        self.draw_floating_dock(pm, sw, sh, unit);

        // 7. Weather Card (when opened from top-right Weather pill)
        if self.weather_popup_open {
            self.draw_weather_card(pm, sw, sh, unit, time_str, date_str);
        }

        // 8. Exact Control Center Overlay (image_4f8f03.jpg)
        if self.control_center_open {
            self.draw_control_center(pm, sw, sh, unit);
        }

        // 9. Recent Apps Switcher Overlay (image_4f8f85.jpg)
        if self.recent_apps_open {
            self.draw_recent_apps(pm, sw, sh, unit);
        }

        // 10. Full Two-Pane Home Settings Screen
        if self.settings.open {
            let rows = self.build_settings_rows(last_key_label, bound_count);
            self.settings
                .draw(pm, self.glass_backdrop_pm.as_ref(), unit, &rows);
        }

        // 11. App Long-Press Context Menu
        if self.app_menu.open {
            self.draw_app_context_menu(pm, sw, sh, unit);
        }

        // 12. Native In-Launcher Search Overlay
        if self.search_ui.open {
            self.draw_search_overlay(pm, sw, sh, unit);
        }

        // 13. First-Time Setup Wizard (full-screen Liquid Glass onboarding)
        if self.setup.open {
            let setup_opts = self.setup.build_options(
                self.lang_code,
                &self.user_name,
                self.weather_auto,
                self.weather_enabled,
                &self.weather_city,
                self.classic_look,
                last_key_label,
                self.sys.default_home,
                self.sys.a11y_enabled,
            );
            self.setup.draw(
                pm,
                self.wallpaper_pm.as_ref(),
                self.glass_backdrop_pm.as_ref(),
                unit,
                &setup_opts,
                &self.user_name,
                self.settings.status_banner.as_deref(),
            );
        }

        // 14. Inbuilt Voice Assistant HUD pill
        self.draw_voice_hud_banner(pm, sw, unit);
    }

    /// Renders the exact Control Center layout shown in `image_4f8f03.jpg`:
    /// - Full-screen blurred & dimmed backdrop
    /// - Right-aligned 2x4 grid of 8 stadium pills (Wi-Fi, Bluetooth, Memory with RAM bar,
    ///   Do Not Disturb, Audio, Sleep Timer, Ambient Mode, Power Off)
    /// - Bottom row of 4 rounded squircles (Inputs, Button Mapper, Home Settings, Search)
    fn draw_control_center(&self, pm: &mut Pixmap, sw: f32, sh: f32, unit: f32) {
        if let Some(ref glass) = self.glass_backdrop_pm {
            let sx = sw / glass.width() as f32;
            let sy = sh / glass.height() as f32;
            let mut gp = Paint::default();
            gp.shader = Pattern::new(
                glass.as_ref(),
                SpreadMode::Pad,
                FilterQuality::Bilinear,
                1.0,
                Transform::from_scale(sx, sy),
            );
            if let Some(r) = tiny_skia::Rect::from_xywh(0.0, 0.0, sw, sh) {
                pm.fill_rect(r, &gp, Transform::identity(), None);
            }
        }
        let mut scrim = Paint::default();
        scrim.set_color(Color::from_rgba8(8, 10, 16, 175));
        if let Some(r) = tiny_skia::Rect::from_xywh(0.0, 0.0, sw, sh) {
            pm.fill_rect(r, &scrim, Transform::identity(), None);
        }

        let pill_w = (152.0 * unit).round();
        let pill_h = (52.0 * unit).round();
        let pill_r = pill_h * 0.5;
        let col_gap = (12.0 * unit).round();
        let row_gap = (12.0 * unit).round();
        let grid_w = pill_w * 2.0 + col_gap;
        let grid_x = (sw - 34.0 * unit - grid_w).round();
        let grid_y = (76.0 * unit).round();

        let ram_pct = if self.sys.ram_total_mb > 0 {
            ((self.sys.ram_used_mb * 100) / self.sys.ram_total_mb).min(100)
        } else {
            65
        };

        let tiles: [(&str, String, Glyph, Option<[u8; 3]>); 8] = [
            ("Wi-Fi", self.sys.wifi_label.clone(), Glyph::Wifi, Some([24, 152, 255])),
            (
                "Bluetooth",
                if self.sys.bt_on { "On".into() } else { "Off".into() },
                Glyph::Bluetooth,
                Some([44, 132, 255]),
            ),
            (
                "Memory",
                format!("{}% used · Clean", ram_pct),
                Glyph::Memory,
                None,
            ),
            (
                "Do Not Dist...",
                if self.sys.dnd_on { "On".into() } else { "Off".into() },
                Glyph::Moon,
                None,
            ),
            ("Audio", "TV Speakers".into(), Glyph::Audio, None),
            (
                "Sleep Timer",
                if self.sleep_timer_mins == 0 {
                    "Off".into()
                } else {
                    format!("{} min", self.sleep_timer_mins)
                },
                Glyph::Timer,
                None,
            ),
            ("Ambient M...", "Screensaver".into(), Glyph::Ambient, Some([255, 212, 59])),
            ("Power Off", "Screen Off".into(), Glyph::Power, Some([255, 92, 84])),
        ];

        for (i, (title, sub, glyph, tint_rgb)) in tiles.iter().enumerate() {
            let r = (i / 2) as f32;
            let c = (i % 2) as f32;
            let x = grid_x + c * (pill_w + col_gap);
            let y = grid_y + r * (pill_h + row_gap);
            let focused = self.cc_selected == i;

            if let Some(pill) = rounded_rect_path(x, y, pill_w, pill_h, pill_r) {
                let mut fill = Paint::default();
                fill.anti_alias = true;
                if focused {
                    fill.set_color(Color::from_rgba8(245, 247, 250, 250));
                } else {
                    fill.set_color(Color::from_rgba8(24, 27, 36, 205));
                }
                pm.fill_path(&pill, &fill, FillRule::Winding, Transform::identity(), None);

                let mut rim = Paint::default();
                rim.anti_alias = true;
                rim.set_color(if focused {
                    Color::from_rgba8(255, 255, 255, 255)
                } else {
                    Color::from_rgba8(255, 255, 255, 68)
                });
                pm.stroke_path(
                    &pill,
                    &rim,
                    &Stroke {
                        width: (1.2 * unit).max(1.0),
                        ..Default::default()
                    },
                    Transform::identity(),
                    None,
                );
            }

            // Left crisp vector glyph (Wi-Fi waves, Bluetooth rune, Memory chip, Moon, etc.)
            let g_size = (20.0 * unit).round();
            let gx = x + 15.0 * unit;
            let gy = y + (pill_h - g_size) * 0.5;
            let g_col = if let Some(rgb) = tint_rgb {
                Color::from_rgba8(rgb[0], rgb[1], rgb[2], 255)
            } else if focused {
                Color::from_rgba8(18, 22, 32, 250)
            } else {
                Color::from_rgba8(244, 246, 252, 245)
            };
            draw_glyph(pm, *glyph, gx, gy, g_size, g_col);

            let title_col = if focused {
                Color::from_rgba8(16, 18, 26, 255)
            } else {
                Color::from_rgba8(244, 246, 252, 245)
            };
            let sub_col = if focused {
                Color::from_rgba8(64, 72, 88, 235)
            } else {
                Color::from_rgba8(178, 188, 206, 210)
            };

            let tx = (x + 42.0 * unit) as i32;
            draw_crisp_label(
                pm,
                self.font_medium.as_ref(),
                title,
                tx,
                (y + pill_h * 0.45) as i32,
                (11.8 * unit).round(),
                title_col,
                false,
            );
            draw_crisp_label(
                pm,
                self.font_regular.as_ref(),
                sub,
                tx,
                (y + pill_h * 0.76) as i32,
                (9.8 * unit).round(),
                sub_col,
                false,
            );

            // Live RAM progress bar inside the Memory pill (tile index 2)
            if i == 2 {
                let bar_x = x + 42.0 * unit;
                let bar_y = y + pill_h - 8.5 * unit;
                let bar_w = pill_w - 58.0 * unit;
                let bar_h = (3.0 * unit).max(2.0);
                if let Some(track) = rounded_rect_path(bar_x, bar_y, bar_w, bar_h, bar_h * 0.5) {
                    let mut tp = Paint::default();
                    tp.anti_alias = true;
                    tp.set_color(if focused {
                        Color::from_rgba8(0, 0, 0, 45)
                    } else {
                        Color::from_rgba8(255, 255, 255, 55)
                    });
                    pm.fill_path(&track, &tp, FillRule::Winding, Transform::identity(), None);
                }
                let fill_w = (bar_w * (ram_pct as f32 / 100.0)).max(6.0 * unit);
                if let Some(fill_bar) = rounded_rect_path(bar_x, bar_y, fill_w, bar_h, bar_h * 0.5) {
                    let mut fp = Paint::default();
                    fp.anti_alias = true;
                    fp.set_color(if focused {
                        Color::from_rgba8(24, 28, 38, 235)
                    } else {
                        Color::from_rgba8(245, 247, 252, 240)
                    });
                    pm.fill_path(&fill_bar, &fp, FillRule::Winding, Transform::identity(), None);
                }
            }
        }

        // Bottom row of 4 rounded squircle action buttons: Inputs, Button Mapper, Home Settings, Search
        let sq_y = grid_y + 4.0 * (pill_h + row_gap);
        let sq_gap = (12.0 * unit).round();
        let sq_size = ((grid_w - sq_gap * 3.0) / 4.0).round();
        let sq_r = (16.0 * unit).round();
        let sq_glyphs = [Glyph::Input, Glyph::Remote, Glyph::Gear, Glyph::Search];

        for (b, glyph) in sq_glyphs.iter().enumerate() {
            let x = grid_x + (b as f32) * (sq_size + sq_gap);
            let focused = self.cc_selected == 8 + b;
            if let Some(sq) = rounded_rect_path(x, sq_y, sq_size, sq_size, sq_r) {
                let mut fill = Paint::default();
                fill.anti_alias = true;
                if focused {
                    fill.set_color(Color::from_rgba8(245, 247, 250, 250));
                } else {
                    fill.set_color(Color::from_rgba8(24, 27, 36, 205));
                }
                pm.fill_path(&sq, &fill, FillRule::Winding, Transform::identity(), None);

                let mut rim = Paint::default();
                rim.anti_alias = true;
                rim.set_color(if focused {
                    Color::from_rgba8(255, 255, 255, 255)
                } else {
                    Color::from_rgba8(255, 255, 255, 68)
                });
                pm.stroke_path(
                    &sq,
                    &rim,
                    &Stroke {
                        width: (1.2 * unit).max(1.0),
                        ..Default::default()
                    },
                    Transform::identity(),
                    None,
                );
            }

            let g_size = (24.0 * unit).round();
            let gx = x + (sq_size - g_size) * 0.5;
            let gy = sq_y + (sq_size - g_size) * 0.5;
            let g_col = if focused {
                Color::from_rgba8(16, 18, 26, 255)
            } else {
                Color::from_rgba8(245, 247, 252, 245)
            };
            draw_glyph(pm, *glyph, gx, gy, g_size, g_col);
        }
    }

    /// Renders the Recent Apps Switcher shown in `image_4f8f85.jpg`:
    /// - Top-left "Recent Apps" title + Top-right live `RAM X.X GB used of X.X GB` pill meter
    /// - Centered horizontal app cards + bottom hint "Press ▲ on an app to close it"
    fn draw_recent_apps(&self, pm: &mut Pixmap, sw: f32, sh: f32, unit: f32) {
        let mut scrim = Paint::default();
        scrim.set_color(Color::from_rgba8(8, 10, 16, 212));
        if let Some(r) = tiny_skia::Rect::from_xywh(0.0, 0.0, sw, sh) {
            pm.fill_rect(r, &scrim, Transform::identity(), None);
        }

        draw_crisp_label(
            pm,
            self.font_medium.as_ref(),
            "Recent Apps",
            (54.0 * unit) as i32,
            (56.0 * unit) as i32,
            (19.0 * unit).round(),
            Color::from_rgba8(255, 255, 255, 250),
            false,
        );

        // Top-right RAM pill meter
        let ram_w = (290.0 * unit).round();
        let ram_h = (36.0 * unit).round();
        let ram_x = (sw - 54.0 * unit - ram_w).round();
        let ram_y = (32.0 * unit).round();
        if let Some(rp) = rounded_rect_path(ram_x, ram_y, ram_w, ram_h, ram_h * 0.5) {
            let mut bg = Paint::default();
            bg.anti_alias = true;
            bg.set_color(Color::from_rgba8(24, 27, 36, 220));
            pm.fill_path(&rp, &bg, FillRule::Winding, Transform::identity(), None);

            let mut rim = Paint::default();
            rim.anti_alias = true;
            rim.set_color(Color::from_rgba8(255, 255, 255, 76));
            pm.stroke_path(
                &rp,
                &rim,
                &Stroke {
                    width: 1.0,
                    ..Default::default()
                },
                Transform::identity(),
                None,
            );
        }

        let used_gb = self.sys.ram_used_mb as f32 / 1024.0;
        let total_gb = (self.sys.ram_total_mb as f32 / 1024.0).max(0.5);
        let ram_txt = format!("RAM {:.1} GB used of {:.1} GB", used_gb, total_gb);
        draw_crisp_label(
            pm,
            self.font_medium.as_ref(),
            &ram_txt,
            (ram_x + 16.0 * unit) as i32,
            (ram_y + ram_h * 0.64) as i32,
            (10.8 * unit).round(),
            Color::from_rgba8(242, 245, 252, 245),
            false,
        );

        // Horizontal app cards
        let app_count = self.apps.len().saturating_sub(1);
        if app_count > 0 {
            let card_w = (410.0 * unit).round();
            let card_h = (232.0 * unit).round();
            let card_gap = (24.0 * unit).round();
            let center_x = ((sw - card_w) * 0.5).round();
            let card_y = ((sh - card_h) * 0.54).round();

            for offset in -1isize..=1isize {
                let idx = (self.recent_selected as isize + offset).rem_euclid(app_count as isize) as usize;
                let Some(app) = self.apps.get(idx) else { continue };
                let cx = center_x + (offset as f32) * (card_w + card_gap);
                if cx + card_w < 0.0 || cx > sw {
                    continue;
                }

                // App name above card
                draw_crisp_label(
                    pm,
                    self.font_medium.as_ref(),
                    &app.label,
                    (cx + 8.0 * unit) as i32,
                    (card_y - 12.0 * unit) as i32,
                    (13.5 * unit).round(),
                    Color::from_rgba8(255, 255, 255, 245),
                    false,
                );

                if let Some(cp) = rounded_rect_path(cx, card_y, card_w, card_h, 24.0 * unit) {
                    let mut bg = Paint::default();
                    bg.anti_alias = true;
                    bg.set_color(if offset == 0 {
                        Color::from_rgba8(245, 247, 250, 250)
                    } else {
                        Color::from_rgba8(app.accent[0], app.accent[1], app.accent[2], 235)
                    });
                    pm.fill_path(&cp, &bg, FillRule::Winding, Transform::identity(), None);
                }

                if let Some(ref icon) = app.icon {
                    let isz = (86.0 * unit).round();
                    let ix = cx + (card_w - isz) * 0.5;
                    let iy = card_y + (card_h - isz) * 0.5;
                    let sx = isz / icon.width() as f32;
                    let sy = isz / icon.height() as f32;
                    pm.draw_pixmap(
                        0,
                        0,
                        icon.as_ref(),
                        &PixmapPaint {
                            quality: FilterQuality::Bilinear,
                            ..Default::default()
                        },
                        Transform::from_scale(sx, sy).post_translate(ix, iy),
                        None,
                    );
                }
            }
        }

        let hint = "Press ▲ on an app to close it  ·  OK to open  ·  Back to close";
        let hpx = (11.5 * unit).round();
        let hw = measure_label_width(self.font_medium.as_ref(), hint, hpx);
        draw_crisp_label(
            pm,
            self.font_medium.as_ref(),
            hint,
            ((sw - hw) * 0.5).round() as i32,
            (sh - 34.0 * unit) as i32,
            hpx,
            Color::from_rgba8(235, 240, 250, 230),
            false,
        );
    }

    fn draw_top_right_pills(&self, pm: &mut Pixmap, sw: f32, unit: f32) {
        let base_size = (38.0 * unit).round();
        let top_y = (18.0 * unit).round();
        let right_margin = (22.0 * unit).round();
        let gap = (10.0 * unit).round();

        let gear_x = sw - right_margin - base_size;
        let weather_x = gear_x - gap - base_size;

        let items = [
            (FocusZone::TopWeather, weather_x, "Weather"),
            (FocusZone::TopControlCenter, gear_x, "Control Center"),
        ];

        for (zone, base_x, label) in items {
            let focused = self.focus_zone == zone
                && !self.settings.open
                && !self.control_center_open
                && !self.recent_apps_open;
            let size = if focused { (base_size * 1.10).round() } else { base_size };
            let cx = base_x - (size - base_size) * 0.5;
            let cy = top_y - (size - base_size) * 0.5;

            if let Some(circle) = rounded_rect_path(cx, cy, size, size, size * 0.5) {
                let mut bg = Paint::default();
                bg.anti_alias = true;
                bg.set_color(if focused {
                    Color::from_rgba8(44, 52, 74, 215)
                } else {
                    Color::from_rgba8(24, 28, 40, 155)
                });
                pm.fill_path(&circle, &bg, FillRule::Winding, Transform::identity(), None);

                let mut rim = Paint::default();
                rim.anti_alias = true;
                rim.set_color(if focused {
                    Color::from_rgba8(255, 255, 255, 245)
                } else {
                    Color::from_rgba8(255, 255, 255, 92)
                });
                let mut st = Stroke::default();
                st.width = if focused { (2.0 * unit).max(1.5) } else { (1.1 * unit).max(1.0) };
                pm.stroke_path(&circle, &rim, &st, Transform::identity(), None);

                let g_size = (19.0 * unit).round();
                let gx = cx + (size - g_size) * 0.5;
                let gy = cy + (size - g_size) * 0.5;
                let top_glyph = if zone == FocusZone::TopWeather {
                    Glyph::Cloud
                } else {
                    Glyph::Sliders
                };
                draw_glyph(
                    pm,
                    top_glyph,
                    gx,
                    gy,
                    g_size,
                    Color::from_rgba8(244, 246, 252, 245),
                );
            }

            if focused {
                let tip_px = (11.0 * unit).round();
                let tw = measure_label_width(self.font_medium.as_ref(), label, tip_px);
                let pill_w = (tw + 18.0 * unit).round();
                let pill_h = (20.0 * unit).round();
                let pill_x = (cx + size * 0.5 - pill_w * 0.5)
                    .clamp(12.0 * unit, sw - pill_w - 12.0 * unit)
                    .round();
                let pill_y = (cy + size + 6.0 * unit).round();
                if let Some(tp) = rounded_rect_path(pill_x, pill_y, pill_w, pill_h, pill_h * 0.5) {
                    let mut fill = Paint::default();
                    fill.anti_alias = true;
                    fill.set_color(Color::from_rgba8(24, 28, 38, 235));
                    pm.fill_path(&tp, &fill, FillRule::Winding, Transform::identity(), None);
                }
                draw_crisp_label(
                    pm,
                    self.font_medium.as_ref(),
                    label,
                    (pill_x + (pill_w - tw) * 0.5).round() as i32,
                    (pill_y + pill_h * 0.68).round() as i32,
                    tip_px,
                    Color::from_rgba8(245, 247, 252, 250),
                    false,
                );
            }
        }
    }

    fn draw_weather_card(&self, pm: &mut Pixmap, sw: f32, sh: f32, unit: f32, time_str: &str, date_str: &str) {
        let cw = (360.0 * unit).round();
        let ch = (165.0 * unit).round();
        let cx = ((sw - cw) * 0.5).round();
        let cy = ((sh - ch) * 0.44).round();
        if let Some(card) = rounded_rect_path(cx, cy, cw, ch, 22.0 * unit) {
            if let Some(ref glass) = self.glass_backdrop_pm {
                let sx = sw / glass.width() as f32;
                let sy = sh / glass.height() as f32;
                let mut gp = Paint::default();
                gp.anti_alias = true;
                gp.shader = Pattern::new(
                    glass.as_ref(),
                    SpreadMode::Pad,
                    FilterQuality::Bilinear,
                    0.95,
                    Transform::from_scale(sx, sy),
                );
                pm.fill_path(&card, &gp, FillRule::Winding, Transform::identity(), None);
            }
            let mut bg = Paint::default();
            bg.anti_alias = true;
            bg.set_color(Color::from_rgba8(20, 24, 36, 155));
            pm.fill_path(&card, &bg, FillRule::Winding, Transform::identity(), None);

            let mut rim = Paint::default();
            rim.anti_alias = true;
            rim.set_color(Color::from_rgba8(255, 255, 255, 105));
            let mut st = Stroke::default();
            st.width = (1.2 * unit).max(1.0);
            pm.stroke_path(&card, &rim, &st, Transform::identity(), None);
        }

        let pad = (22.0 * unit).round();
        let city_title = if self.weather_city.is_empty() {
            format!("{}  ·  {}", time_str, date_str)
        } else if self.weather_temp.is_empty() {
            self.weather_city.clone()
        } else {
            format!("{}  ·  {} {}", self.weather_city, self.weather_temp, self.weather_desc)
        };
        draw_crisp_label(
            pm,
            self.font_medium.as_ref(),
            &city_title,
            (cx + pad) as i32,
            (cy + 38.0 * unit) as i32,
            (15.0 * unit).round(),
            Color::from_rgba8(255, 255, 255, 252),
            false,
        );
        draw_crisp_label(
            pm,
            self.font_regular.as_ref(),
            &format!("{}  ·  Scene: {}", date_str, self.current_scene().label),
            (cx + pad) as i32,
            (cy + 74.0 * unit) as i32,
            (12.5 * unit).round(),
            Color::from_rgba8(165, 218, 255, 240),
            false,
        );
        draw_crisp_label(
            pm,
            self.font_regular.as_ref(),
            "Left/Right: Switch Scene  ·  OK/Back: Close",
            (cx + pad) as i32,
            (cy + 118.0 * unit) as i32,
            (11.5 * unit).round(),
            Color::from_rgba8(215, 224, 238, 215),
            false,
        );
    }

    fn draw_floating_dock(&self, pm: &mut Pixmap, sw: f32, sh: f32, unit: f32) {
        if self.apps.is_empty() {
            return;
        }
        let end = (self.page_offset + MAX_DOCK_TILES).min(self.apps.len());
        let visible = &self.apps[self.page_offset..end];
        let count = visible.len();

        let icon_size = (46.0 * unit).round();
        let icon_gap = (10.0 * unit).round();
        let pad_h = (13.0 * unit).round();
        let pad_v = (10.0 * unit).round();
        let dock_w = pad_h * 2.0 + (count as f32) * icon_size + ((count.saturating_sub(1)) as f32) * icon_gap;
        let dock_h = pad_v * 2.0 + icon_size;
        let dock_x = ((sw - dock_w) * 0.5).round();
        let dock_y = (sh - (14.0 * unit).round() - dock_h).round();
        let dock_r = (22.0 * unit).round();
        let icon_r = (11.0 * unit).round();

        if let Some(dock_path) = rounded_rect_path(dock_x, dock_y, dock_w, dock_h, dock_r) {
            if let Some(ref glass) = self.glass_backdrop_pm {
                let sx = sw / glass.width() as f32;
                let sy = sh / glass.height() as f32;
                let mut gp = Paint::default();
                gp.anti_alias = true;
                gp.shader = Pattern::new(
                    glass.as_ref(),
                    SpreadMode::Pad,
                    FilterQuality::Bilinear,
                    0.92,
                    Transform::from_scale(sx, sy),
                );
                pm.fill_path(&dock_path, &gp, FillRule::Winding, Transform::identity(), None);
            }

            let mut tint = Paint::default();
            tint.anti_alias = true;
            tint.set_color(Color::from_rgba8(24, 22, 34, 118));
            pm.fill_path(&dock_path, &tint, FillRule::Winding, Transform::identity(), None);

            let mut rim = Paint::default();
            rim.anti_alias = true;
            rim.set_color(Color::from_rgba8(255, 255, 255, 96));
            let mut st = Stroke::default();
            st.width = (1.3 * unit).max(1.0);
            pm.stroke_path(&dock_path, &rim, &st, Transform::identity(), None);
        }

        let start_x = dock_x + pad_h;
        let start_y = dock_y + pad_v;
        let rel_sel = self.selected_index.saturating_sub(self.page_offset);
        let dock_has_focus = self.focus_zone == FocusZone::Dock
            && !self.settings.open
            && !self.control_center_open
            && !self.recent_apps_open;

        for (i, app) in visible.iter().enumerate() {
            if dock_has_focus && i == rel_sel {
                continue;
            }
            let ix = start_x + (i as f32) * (icon_size + icon_gap);
            self.draw_squircle_icon(pm, app, ix, start_y, icon_size, icon_r, 1.0, false, unit);
        }

        if dock_has_focus {
            if let Some(app) = visible.get(rel_sel) {
                let sc = *self.focus_scales.get(self.selected_index).unwrap_or(&1.15);
                let ix = start_x + (rel_sel as f32) * (icon_size + icon_gap);
                self.draw_squircle_icon(pm, app, ix, start_y, icon_size, icon_r, sc, true, unit);

                let tip_px = (11.5 * unit).round();
                let text_w = measure_label_width(self.font_medium.as_ref(), &app.label, tip_px);
                let tip_w = (text_w + 22.0 * unit).max(46.0 * unit).round();
                let tip_h = (22.0 * unit).round();
                let icon_center_x = ix + icon_size * 0.5;
                let tip_x = (icon_center_x - tip_w * 0.5)
                    .clamp(12.0 * unit, sw - tip_w - 12.0 * unit)
                    .round();
                let tip_y = (dock_y - tip_h - 8.0 * unit).round();

                if let Some(tip_path) = rounded_rect_path(tip_x, tip_y, tip_w, tip_h, 7.0 * unit) {
                    let mut fill = Paint::default();
                    fill.anti_alias = true;
                    fill.set_color(Color::from_rgba8(26, 29, 38, 236));
                    pm.fill_path(&tip_path, &fill, FillRule::Winding, Transform::identity(), None);

                    let mut rim = Paint::default();
                    rim.anti_alias = true;
                    rim.set_color(Color::from_rgba8(255, 255, 255, 76));
                    let mut st = Stroke::default();
                    st.width = (1.0 * unit).max(1.0);
                    pm.stroke_path(&tip_path, &rim, &st, Transform::identity(), None);
                }

                let lbl_x = (tip_x + (tip_w - text_w) * 0.5).round() as i32;
                let lbl_y = (tip_y + tip_h * 0.68).round() as i32;
                draw_crisp_label(
                    pm,
                    self.font_medium.as_ref(),
                    &app.label,
                    lbl_x,
                    lbl_y,
                    tip_px,
                    Color::from_rgba8(245, 246, 250, 250),
                    false,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_squircle_icon(
        &self,
        pm: &mut Pixmap,
        app: &AppTile,
        base_x: f32,
        base_y: f32,
        size: f32,
        radius: f32,
        scale: f32,
        focused: bool,
        unit: f32,
    ) {
        let w = (size * scale).round();
        let h = w;
        let x = (base_x - (w - size) * 0.5).round();
        let y = (base_y - (h - size) * 0.5 - if focused { 2.0 * unit } else { 0.0 }).round();
        let r = (radius * scale).round();

        if focused {
            if let Some(sh) = rounded_rect_path(x - 1.5 * unit, y + 4.0 * unit, w + 3.0 * unit, h + 3.0 * unit, r + 2.0) {
                let mut sp = Paint::default();
                sp.anti_alias = true;
                sp.set_color(Color::from_rgba8(0, 0, 0, 125));
                pm.fill_path(&sh, &sp, FillRule::Winding, Transform::identity(), None);
            }
        }

        if let Some(squircle) = rounded_rect_path(x, y, w, h, r) {
            let mut bg = Paint::default();
            bg.anti_alias = true;
            if app.icon.is_some() {
                bg.set_color(Color::from_rgba8(255, 255, 255, 22));
            } else {
                bg.set_color(Color::from_rgba8(app.accent[0], app.accent[1], app.accent[2], 235));
            }
            pm.fill_path(&squircle, &bg, FillRule::Winding, Transform::identity(), None);

            if let Some(ref icon) = app.icon {
                let sx = w / icon.width() as f32;
                let sy = h / icon.height() as f32;
                let mut ip = Paint::default();
                ip.anti_alias = true;
                ip.shader = Pattern::new(
                    icon.as_ref(),
                    SpreadMode::Pad,
                    FilterQuality::Bilinear,
                    1.0,
                    Transform::from_scale(sx, sy).post_translate(x, y),
                );
                pm.fill_path(&squircle, &ip, FillRule::Winding, Transform::identity(), None);
            } else {
                let initial = app.label.chars().next().unwrap_or('A').to_string();
                let fpx = (w * 0.46).round();
                let iw = measure_label_width(self.font_medium.as_ref(), &initial, fpx);
                draw_crisp_label(
                    pm,
                    self.font_medium.as_ref(),
                    &initial,
                    (x + (w - iw) * 0.5).round() as i32,
                    (y + h * 0.67).round() as i32,
                    fpx,
                    Color::from_rgba8(255, 255, 255, 245),
                    true,
                );
            }

            if focused {
                let mut rim = Paint::default();
                rim.anti_alias = true;
                rim.set_color(Color::from_rgba8(255, 255, 255, 240));
                let mut st = Stroke::default();
                st.width = (2.0 * unit).max(1.5);
                pm.stroke_path(&squircle, &rim, &st, Transform::identity(), None);
            }
        }
    }

    pub fn open_app_context_menu(&mut self) {
        if self.focus_zone != FocusZone::Dock || self.apps.is_empty() {
            return;
        }
        if let Some(tile) = self.apps.get(self.selected_index) {
            if tile.package_name == SETTINGS_TILE_PKG {
                return;
            }
            self.app_menu = AppContextMenu {
                open: true,
                app_idx: self.selected_index,
                selected: 0,
            };
        }
    }

    pub fn app_menu_items(&self) -> [&'static str; 7] {
        let is_builtin = self
            .apps
            .get(self.app_menu.app_idx)
            .map(|a| a.is_builtin)
            .unwrap_or(false);
        [
            "Open",
            "Rename",
            "Move Left",
            "Move Right",
            "Hide App",
            "App Details",
            if is_builtin { "Disable" } else { "Uninstall" },
        ]
    }

    pub fn search_matching_apps(&self) -> Vec<(String, String, bool)> {
        // Returns (package_name, display_label, is_web_action)
        let q = self.search_ui.query.trim();
        let q_low = q.to_lowercase();
        let mut out = Vec::new();
        for app in &self.apps {
            if app.package_name == SETTINGS_TILE_PKG {
                continue;
            }
            if q_low.is_empty()
                || app.label.to_lowercase().contains(&q_low)
                || app.package_name.to_lowercase().contains(&q_low)
            {
                out.push((app.package_name.clone(), app.label.clone(), false));
            }
        }
        if !q.is_empty() {
            // Find YouTube / TizenTube / SmartTube package if installed
            if let Some(yt) = self.apps.iter().find(|a| {
                let l = a.label.to_lowercase();
                let p = a.package_name.to_lowercase();
                l.contains("youtube") || p.contains("youtube") || p.contains("tizentube") || p.contains("smarttube")
            }) {
                out.push((
                    yt.package_name.clone(),
                    format!("Search \"{}\" in {}", q, yt.label),
                    true,
                ));
            }
            if let Some(br) = self.apps.iter().find(|a| {
                let l = a.label.to_lowercase();
                let p = a.package_name.to_lowercase();
                l.contains("koji") || l.contains("browser") || p.contains("koji") || p.contains("browser") || p.contains("chrome")
            }) {
                out.push((
                    br.package_name.clone(),
                    format!("Search \"{}\" in {}", q, br.label),
                    true,
                ));
            }
        }
        out
    }

    pub fn app_menu_geometry(&self, sw: f32, sh: f32, unit: f32) -> (f32, f32, f32, f32, f32, f32, f32, f32) {
        // Returns (px, py, panel_w, panel_h, pad, header_h, row_h, row_gap)
        let items_len = self.app_menu_items().len() as f32;
        let panel_w = (242.0 * unit).round();
        let row_h = (28.0 * unit).round();
        let row_gap = (5.0 * unit).round();
        let header_h = (36.0 * unit).round();
        let pad = (12.0 * unit).round();
        let panel_h = pad * 2.0 + header_h + items_len * row_h + (items_len - 1.0) * row_gap;
        let px = ((sw - panel_w) * 0.5).round();
        // Float neatly above the dock instead of covering the entire screen
        let py = (sh - (92.0 * unit) - panel_h).max(24.0 * unit).round();
        (px, py, panel_w, panel_h, pad, header_h, row_h, row_gap)
    }

    fn draw_app_context_menu(&self, pm: &mut Pixmap, sw: f32, sh: f32, unit: f32) {
        let Some(app) = self.apps.get(self.app_menu.app_idx) else {
            return;
        };
        let mut scrim = Paint::default();
        scrim.set_color(Color::from_rgba8(6, 8, 14, 96));
        if let Some(r) = tiny_skia::Rect::from_xywh(0.0, 0.0, sw, sh) {
            pm.fill_rect(r, &scrim, Transform::identity(), None);
        }

        let items = self.app_menu_items();
        let (px, py, panel_w, panel_h, pad, header_h, row_h, row_gap) =
            self.app_menu_geometry(sw, sh, unit);
        let card_r = (20.0 * unit).round();

        // Soft drop shadow beneath the floating Liquid Glass sheet
        if let Some(shadow) = rounded_rect_path(
            px - 2.0 * unit,
            py + 6.0 * unit,
            panel_w + 4.0 * unit,
            panel_h + 4.0 * unit,
            card_r + 2.0 * unit,
        ) {
            let mut sp = Paint::default();
            sp.anti_alias = true;
            sp.set_color(Color::from_rgba8(0, 0, 0, 108));
            pm.fill_path(&shadow, &sp, FillRule::Winding, Transform::identity(), None);
        }

        // True Liquid Glass card: live blurred glass backdrop + frosted tint + top specular sheen + crisp rim
        if let Some(card) = rounded_rect_path(px, py, panel_w, panel_h, card_r) {
            if let Some(ref glass) = self.glass_backdrop_pm {
                let sx = sw / glass.width() as f32;
                let sy = sh / glass.height() as f32;
                let mut gp = Paint::default();
                gp.anti_alias = true;
                gp.shader = Pattern::new(
                    glass.as_ref(),
                    SpreadMode::Pad,
                    FilterQuality::Bilinear,
                    0.96,
                    Transform::from_scale(sx, sy),
                );
                pm.fill_path(&card, &gp, FillRule::Winding, Transform::identity(), None);
            }

            let mut frost = Paint::default();
            frost.anti_alias = true;
            frost.set_color(Color::from_rgba8(20, 24, 36, 134));
            pm.fill_path(&card, &frost, FillRule::Winding, Transform::identity(), None);

            if let Some(sheen) = LinearGradient::new(
                Point::from_xy(px, py),
                Point::from_xy(px, py + panel_h * 0.45),
                vec![
                    GradientStop::new(0.0, Color::from_rgba8(255, 255, 255, 34)),
                    GradientStop::new(1.0, Color::from_rgba8(255, 255, 255, 0)),
                ],
                SpreadMode::Pad,
                Transform::identity(),
            ) {
                let mut sp = Paint::default();
                sp.anti_alias = true;
                sp.shader = sheen;
                pm.fill_path(&card, &sp, FillRule::Winding, Transform::identity(), None);
            }

            let mut rim = Paint::default();
            rim.anti_alias = true;
            rim.set_color(Color::from_rgba8(255, 255, 255, 118));
            pm.stroke_path(
                &card,
                &rim,
                &Stroke {
                    width: (1.3 * unit).max(1.0),
                    ..Default::default()
                },
                Transform::identity(),
                None,
            );
        }

        // Compact App Header
        draw_crisp_label(
            pm,
            self.font_medium.as_ref(),
            &app.label,
            (px + pad + 8.0 * unit) as i32,
            (py + pad + 14.0 * unit) as i32,
            (13.0 * unit).round(),
            Color::from_rgba8(255, 255, 255, 252),
            true,
        );
        let subtitle = if app.is_builtin {
            "Built-in app"
        } else {
            "Installed app"
        };
        draw_crisp_label(
            pm,
            self.font_regular.as_ref(),
            subtitle,
            (px + pad + 8.0 * unit) as i32,
            (py + pad + 27.0 * unit) as i32,
            (9.2 * unit).round(),
            Color::from_rgba8(190, 206, 232, 210),
            false,
        );

        let list_y = py + pad + header_h;
        let pill_w = panel_w - pad * 2.0;
        let pill_r = row_h * 0.5;
        for (i, label) in items.iter().enumerate() {
            let ry = list_y + (i as f32) * (row_h + row_gap);
            let focused = i == self.app_menu.selected;
            if let Some(pill) = rounded_rect_path(px + pad, ry, pill_w, row_h, pill_r) {
                let mut fill = Paint::default();
                fill.anti_alias = true;
                fill.set_color(if focused {
                    Color::from_rgba8(246, 248, 252, 248)
                } else {
                    Color::from_rgba8(255, 255, 255, 22)
                });
                pm.fill_path(&pill, &fill, FillRule::Winding, Transform::identity(), None);
            }
            let col = if focused {
                Color::from_rgba8(16, 18, 26, 255)
            } else if *label == "Uninstall" {
                Color::from_rgba8(255, 118, 112, 245)
            } else if *label == "Disable" {
                Color::from_rgba8(255, 204, 96, 248)
            } else {
                Color::from_rgba8(245, 248, 254, 242)
            };
            draw_crisp_label(
                pm,
                self.font_medium.as_ref(),
                label,
                (px + pad + 14.0 * unit) as i32,
                (ry + row_h * 0.66) as i32,
                (11.0 * unit).round(),
                col,
                false,
            );
        }
    }

    fn draw_search_overlay(&self, pm: &mut Pixmap, sw: f32, sh: f32, unit: f32) {
        if let Some(ref glass) = self.glass_backdrop_pm {
            let sx = sw / glass.width() as f32;
            let sy = sh / glass.height() as f32;
            let mut gp = Paint::default();
            gp.shader = Pattern::new(
                glass.as_ref(),
                SpreadMode::Pad,
                FilterQuality::Bilinear,
                1.0,
                Transform::from_scale(sx, sy),
            );
            if let Some(r) = tiny_skia::Rect::from_xywh(0.0, 0.0, sw, sh) {
                pm.fill_rect(r, &gp, Transform::identity(), None);
            }
        }
        let mut scrim = Paint::default();
        scrim.set_color(Color::from_rgba8(10, 12, 18, 218));
        if let Some(r) = tiny_skia::Rect::from_xywh(0.0, 0.0, sw, sh) {
            pm.fill_rect(r, &scrim, Transform::identity(), None);
        }

        let margin = (48.0 * unit).round();
        let box_w = sw - margin * 2.0;
        let box_h = (42.0 * unit).round();
        let box_y = (34.0 * unit).round();

        // Search input stadium pill
        if let Some(bar) = rounded_rect_path(margin, box_y, box_w, box_h, box_h * 0.5) {
            let mut fill = Paint::default();
            fill.anti_alias = true;
            fill.set_color(Color::from_rgba8(28, 32, 44, 235));
            pm.fill_path(&bar, &fill, FillRule::Winding, Transform::identity(), None);

            let mut rim = Paint::default();
            rim.anti_alias = true;
            rim.set_color(Color::from_rgba8(255, 255, 255, 95));
            pm.stroke_path(
                &bar,
                &rim,
                &Stroke {
                    width: 1.3,
                    ..Default::default()
                },
                Transform::identity(),
                None,
            );
        }

        let g_sz = (18.0 * unit).round();
        draw_glyph(
            pm,
            Glyph::Search,
            margin + 16.0 * unit,
            box_y + (box_h - g_sz) * 0.5,
            g_sz,
            Color::from_rgba8(185, 200, 224, 230),
        );

        let display_q = if self.search_ui.query.is_empty() {
            "Search apps, YouTube, or select MIC to speak…"
        } else {
            &self.search_ui.query
        };
        let q_col = if self.search_ui.query.is_empty() {
            Color::from_rgba8(160, 172, 195, 195)
        } else {
            Color::from_rgba8(255, 255, 255, 250)
        };
        draw_crisp_label(
            pm,
            self.font_medium.as_ref(),
            display_q,
            (margin + 44.0 * unit) as i32,
            (box_y + box_h * 0.65) as i32,
            (14.5 * unit).round(),
            q_col,
            false,
        );

        // One-row tvOS keyboard
        let key_y = (94.0 * unit).round();
        let key_h = (30.0 * unit).round();
        let key_gap = (4.0 * unit).round();
        let total_keys = SEARCH_KEYS.len();
        let visible_keys = 22usize;
        let k_start = if self.search_ui.key_idx >= visible_keys {
            self.search_ui.key_idx + 1 - visible_keys
        } else {
            0
        };
        let mut kx = margin;
        for idx in k_start..(k_start + visible_keys).min(total_keys) {
            let lbl = SEARCH_KEYS[idx];
            let kw = if lbl.len() > 1 {
                (44.0 * unit).round()
            } else {
                (30.0 * unit).round()
            };
            let focused = self.search_ui.focus_row == 0 && idx == self.search_ui.key_idx;
            if let Some(kp) = rounded_rect_path(kx, key_y, kw, key_h, key_h * 0.5) {
                let mut fill = Paint::default();
                fill.anti_alias = true;
                fill.set_color(if focused {
                    Color::from_rgba8(245, 247, 250, 250)
                } else if lbl == "MIC" {
                    Color::from_rgba8(42, 132, 255, 190)
                } else {
                    Color::from_rgba8(255, 255, 255, 24)
                });
                pm.fill_path(&kp, &fill, FillRule::Winding, Transform::identity(), None);
            }
            let fpx = (11.0 * unit).round();
            let tw = measure_label_width(self.font_medium.as_ref(), lbl, fpx);
            draw_crisp_label(
                pm,
                self.font_medium.as_ref(),
                lbl,
                (kx + (kw - tw) * 0.5) as i32,
                (key_y + key_h * 0.67) as i32,
                fpx,
                if focused {
                    Color::from_rgba8(16, 18, 26, 255)
                } else {
                    Color::from_rgba8(245, 247, 252, 245)
                },
                false,
            );
            kx += kw + key_gap;
            if kx > sw - margin - 30.0 * unit {
                break;
            }
        }

        // Results list (stadium pills)
        let results = self.search_matching_apps();
        let res_top = (146.0 * unit).round();
        let row_h = (38.0 * unit).round();
        let row_gap = (8.0 * unit).round();
        for (i, (_pkg, label, is_action)) in results.iter().take(7).enumerate() {
            let ry = res_top + (i as f32) * (row_h + row_gap);
            let focused = self.search_ui.focus_row == 1 && i == self.search_ui.result_idx;
            if let Some(rp) = rounded_rect_path(margin, ry, box_w, row_h, row_h * 0.5) {
                let mut fill = Paint::default();
                fill.anti_alias = true;
                fill.set_color(if focused {
                    Color::from_rgba8(245, 247, 250, 250)
                } else if *is_action {
                    Color::from_rgba8(42, 132, 255, 42)
                } else {
                    Color::from_rgba8(255, 255, 255, 18)
                });
                pm.fill_path(&rp, &fill, FillRule::Winding, Transform::identity(), None);
            }
            draw_crisp_label(
                pm,
                self.font_medium.as_ref(),
                label,
                (margin + 18.0 * unit) as i32,
                (ry + row_h * 0.65) as i32,
                (12.5 * unit).round(),
                if focused {
                    Color::from_rgba8(16, 18, 26, 255)
                } else {
                    Color::from_rgba8(244, 247, 252, 245)
                },
                false,
            );
        }
    }

    fn draw_voice_hud_banner(&self, pm: &mut Pixmap, sw: f32, unit: f32) {
        let Some((ref title, ref sub)) = self.voice_hud else {
            return;
        };
        let text = if sub.is_empty() {
            title.clone()
        } else {
            format!("{}  ·  {}", title, sub)
        };
        let fpx = (12.0 * unit).round();
        let tw = measure_label_width(self.font_medium.as_ref(), &text, fpx);
        let pill_w = (tw + 36.0 * unit).min(sw - 60.0 * unit).round();
        let pill_h = (32.0 * unit).round();
        let px = ((sw - pill_w) * 0.5).round();
        let py = (20.0 * unit).round();

        if let Some(pill) = rounded_rect_path(px, py, pill_w, pill_h, pill_h * 0.5) {
            let mut bg = Paint::default();
            bg.anti_alias = true;
            bg.set_color(Color::from_rgba8(18, 24, 38, 242));
            pm.fill_path(&pill, &bg, FillRule::Winding, Transform::identity(), None);

            let mut rim = Paint::default();
            rim.anti_alias = true;
            rim.set_color(Color::from_rgba8(72, 168, 255, 220));
            pm.stroke_path(
                &pill,
                &rim,
                &Stroke {
                    width: 1.5,
                    ..Default::default()
                },
                Transform::identity(),
                None,
            );
        }
        draw_crisp_label(
            pm,
            self.font_medium.as_ref(),
            &text,
            (px + 18.0 * unit) as i32,
            (py + pill_h * 0.66) as i32,
            fpx,
            Color::from_rgba8(255, 255, 255, 250),
            false,
        );
    }
}

fn draw_cover_pixmap(dst: &mut Pixmap, src: &Pixmap) {
    let dw = dst.width() as f32;
    let dh = dst.height() as f32;
    let sw = src.width() as f32;
    let sh = src.height() as f32;
    let scale = (dw / sw).max(dh / sh);
    let tx = (dw - sw * scale) * 0.5;
    let ty = (dh - sh * scale) * 0.5;
    dst.draw_pixmap(
        0,
        0,
        src.as_ref(),
        &PixmapPaint {
            quality: FilterQuality::Bilinear,
            ..Default::default()
        },
        Transform::from_scale(scale, scale).post_translate(tx, ty),
        None,
    );
}

fn bake_liquid_glass(src: &Pixmap) -> Pixmap {
    let gw = (src.width() / 6).max(64);
    let gh = (src.height() / 6).max(36);
    let mut out = Pixmap::new(gw, gh).unwrap_or_else(|| Pixmap::new(64, 36).unwrap());
    let sx = gw as f32 / src.width() as f32;
    let sy = gh as f32 / src.height() as f32;
    out.draw_pixmap(
        0,
        0,
        src.as_ref(),
        &PixmapPaint {
            quality: FilterQuality::Bilinear,
            ..Default::default()
        },
        Transform::from_scale(sx, sy),
        None,
    );

    for px in out.pixels_mut() {
        let r = px.red() as f32;
        let g = px.green() as f32;
        let b = px.blue() as f32;
        let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        let nr = (lum + (r - lum) * 1.55 + 12.0).clamp(0.0, 255.0) as u8;
        let ng = (lum + (g - lum) * 1.55 + 12.0).clamp(0.0, 255.0) as u8;
        let nb = (lum + (b - lum) * 1.55 + 12.0).clamp(0.0, 255.0) as u8;
        *px = tiny_skia::PremultipliedColorU8::from_rgba(nr, ng, nb, 255).unwrap_or(*px);
    }

    let w = gw as usize;
    let h = gh as usize;
    let mut tmp = vec![[0u32; 3]; w * h];
    for _ in 0..2 {
        for y in 0..h {
            for x in 0..w {
                let mut acc = [0u32; 3];
                let mut cnt = 0u32;
                for dx in -4isize..=4 {
                    let nx = (x as isize + dx).clamp(0, w as isize - 1) as usize;
                    let p = out.pixels()[y * w + nx];
                    acc[0] += p.red() as u32;
                    acc[1] += p.green() as u32;
                    acc[2] += p.blue() as u32;
                    cnt += 1;
                }
                tmp[y * w + x] = [acc[0] / cnt, acc[1] / cnt, acc[2] / cnt];
            }
        }
        for y in 0..h {
            for x in 0..w {
                let mut acc = [0u32; 3];
                let mut cnt = 0u32;
                for dy in -4isize..=4 {
                    let ny = (y as isize + dy).clamp(0, h as isize - 1) as usize;
                    let p = tmp[ny * w + x];
                    acc[0] += p[0];
                    acc[1] += p[1];
                    acc[2] += p[2];
                    cnt += 1;
                }
                let idx = y * w + x;
                out.pixels_mut()[idx] = tiny_skia::PremultipliedColorU8::from_rgba(
                    (acc[0] / cnt) as u8,
                    (acc[1] / cnt) as u8,
                    (acc[2] / cnt) as u8,
                    255,
                )
                .unwrap_or(out.pixels()[idx]);
            }
        }
    }
    out
}

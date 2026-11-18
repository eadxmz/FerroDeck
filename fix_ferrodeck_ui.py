#!/usr/bin/env python3
r"""
FerroDeck Complete UI, Font Fallback, Top-Bar Focus & Touch/D-Pad Fix
Fixes every issue seen in the emulator screenshot:
  1. Guarantees the Giant 1:1 Unblurred Glass Clock, Greeting, Date, Dock Tooltip
     text, and Rounded-Pill Settings text ALWAYS render by falling back to Android's
     built-in `/system/fonts/Roboto-*.ttf` when `assets/fonts/` is not bundled.
  2. Synthesizes rich multi-layer atmospheric Night Scenes (stars, aurora/sunset sky
     gradients, and layered mountain silhouettes behind the giant glass clock) whenever
     `assets/backgrounds/` images are absent — while still loading the exact `.jpg` +
     `.webp` cutouts whenever `tvOS-Launcher.zip` assets are present.
  3. Fixes D-Pad Up / Down navigation (`FocusZone::Dock` <-> `TopWeather` <->
     `TopControlCenter`) so pressing Up from the dock highlights the top-right Weather
     and Control Center pills with a crisp white focus ring, and pressing OK opens them.
  4. Adds native touch / mouse click support (`MotionEvent`) in `src/lib.rs` so clicking
     the top-right buttons, dock icons, or rounded-pill rows works on virtual devices too.

Usage (run inside C:\Users\eadxm\Documents\FerroDeck):
    python fix_ferrodeck_ui.py
"""

import os
import shutil
import stat
import zipfile
from pathlib import Path


RUST_UI_RS = """//! Exact Night Scenes + Top-Bar Focus + Liquid Glass Squircle Dock renderer in `tiny-skia`.
//! Supports both bundled scene assets (`scene-night.jpg` + `scene-night-fg.webp`) AND a
//! built-in procedural multi-layer mountain/sky renderer so the clock, mountain depth
//! overlap, and frosted Liquid Glass dock always render even with zero external assets.

use crate::clock::{draw_crisp_label, measure_label_width, CrispClock};
use crate::settings_pill::{rounded_rect_path, PillOption, PillSettingsPanel};
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
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UiAction {
    None,
    OpenWifiSettings,
    OpenDisplaySettings,
    OpenAndroidSettings,
    OpenAccessibilitySettings,
}

pub struct LauncherUi {
    pub apps: Vec<AppTile>,
    pub focus_zone: FocusZone,
    pub selected_index: usize,
    pub page_offset: usize,
    pub clock_style: u8, // 0 = Giant Scene Glass Clock, 1 = Top-Right Crisp, 2 = Hidden
    pub scene_idx: usize,
    pub user_name: String,
    pub weather_popup_open: bool,
    wallpaper_pm: Option<Pixmap>,
    foreground_pm: Option<Pixmap>,
    glass_backdrop_pm: Option<Pixmap>,
    settings_icon_pm: Option<Pixmap>,
    focus_scales: Vec<f32>,
    font_medium: Option<Font>,
    font_regular: Option<Font>,
    pub clock: CrispClock,
    pub settings: PillSettingsPanel,
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
            focus_zone: FocusZone::Dock,
            selected_index: 0,
            page_offset: 0,
            clock_style: 0,
            scene_idx: 0,
            user_name: "sadim".into(),
            weather_popup_open: false,
            wallpaper_pm: None,
            foreground_pm: None,
            glass_backdrop_pm: None,
            settings_icon_pm: settings_icon,
            focus_scales: Vec::new(),
            font_medium,
            font_regular,
            clock: CrispClock::new(big_font.or(bold_font), bold_font.or(med_font)),
            settings: PillSettingsPanel::new(bold_font.or(med_font), reg_font.or(med_font)),
        }
    }

    pub fn current_scene(&self) -> &'static SceneSpec {
        &SCENES[self.scene_idx % SCENES.len()]
    }

    pub fn set_apps(&mut self, mut apps: Vec<AppTile>) {
        apps.push(AppTile {
            package_name: SETTINGS_TILE_PKG.into(),
            label: "Control Center".into(),
            icon: self.settings_icon_pm.clone(),
            accent: [48, 54, 68],
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

    /// Generates an atmospheric procedural Night Scenes backdrop + mountain silhouettes
    /// when external image assets are not packaged in the APK.
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

            // Subtle starfield for Night scene
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

            // Distant mountain ridge on background layer
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

        // Foreground mountain cutout so peaks overlap the lower edge of the giant clock
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

    /// Resets transient overlays and ensures D-Pad selection responds immediately
    /// when returning from sideloaded apps like Coji Browser via the Home key.
    pub fn on_home_pressed(&mut self) {
        self.settings.open = false;
        self.weather_popup_open = false;
        self.focus_zone = FocusZone::Dock;
        if self.selected_index >= self.apps.len() {
            self.selected_index = 0;
        }
    }

    pub fn move_dpad(&mut self, dx: isize, dy: isize) {
        if self.settings.open {
            let opt_count = self.settings_options("").len();
            if dy < 0 {
                self.settings.move_up(opt_count);
            } else if dy > 0 {
                self.settings.move_down(opt_count);
            } else if dx != 0 {
                let _ = self.activate_selected_setting(dx);
            }
            return;
        }

        if self.weather_popup_open {
            if dx != 0 {
                let n = SCENES.len() as isize;
                self.scene_idx =
                    (self.scene_idx as isize + if dx > 0 { 1 } else { -1 }).rem_euclid(n) as usize;
                self.invalidate_scene_cache();
            }
            return;
        }

        // Vertical navigation between Dock and Top-Right Weather / Control Center pills
        if dy < 0 && self.focus_zone == FocusZone::Dock {
            self.focus_zone = FocusZone::TopControlCenter;
            return;
        }
        if dy > 0 && self.focus_zone != FocusZone::Dock {
            self.focus_zone = FocusZone::Dock;
            return;
        }

        // Horizontal navigation within Top-Right pills
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

        // Horizontal navigation within Dock
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

    pub fn activate_selected_setting(&mut self, dir: isize) -> (bool, UiAction) {
        match self.settings.selected {
            0 => {
                let n = SCENES.len() as isize;
                self.scene_idx =
                    (self.scene_idx as isize + if dir >= 0 { 1 } else { -1 }).rem_euclid(n) as usize;
                self.invalidate_scene_cache();
                (true, UiAction::None)
            }
            1 => {
                let next = (self.clock_style as isize + if dir >= 0 { 1 } else { -1 }).rem_euclid(3);
                self.clock_style = next as u8;
                (false, UiAction::None)
            }
            2 => (false, UiAction::OpenWifiSettings),
            3 => (false, UiAction::OpenDisplaySettings),
            4 => (false, UiAction::None), // Handled in lib.rs for key capture
            5 => (false, UiAction::OpenAccessibilitySettings),
            6 => (false, UiAction::OpenAndroidSettings),
            _ => (false, UiAction::None),
        }
    }

    pub fn settings_options(&self, last_key_label: &str) -> Vec<PillOption> {
        let clock_val = match self.clock_style {
            0 => "Big Glass (1:1)",
            1 => "Top-Right Crisp",
            _ => "Hidden",
        };
        vec![
            PillOption {
                title: "Background Scene".into(),
                value: self.current_scene().label.into(),
            },
            PillOption {
                title: "Clock Display".into(),
                value: clock_val.into(),
            },
            PillOption {
                title: "Network & Wi-Fi".into(),
                value: "Open Settings".into(),
            },
            PillOption {
                title: "Display & Resolution".into(),
                value: "Open Settings".into(),
            },
            PillOption {
                title: "Capture Remote Key / Mic".into(),
                value: last_key_label.into(),
            },
            PillOption {
                title: "Accessibility Key Filter".into(),
                value: "Enable Service".into(),
            },
            PillOption {
                title: "Android System Settings".into(),
                value: "Launch".into(),
            },
        ]
    }

    /// Handles touch / mouse click coordinates `(px, py)` in physical screen pixels.
    /// Returns `Some(package_name)` if an app tile was clicked, or `None` if handled internally.
    pub fn handle_tap(&mut self, px: f32, py: f32, sw: f32, sh: f32) -> (Option<String>, UiAction) {
        let unit = (sw / DESIGN_W).min(sh / DESIGN_H);

        if self.weather_popup_open {
            self.weather_popup_open = false;
            return (None, UiAction::None);
        }

        if self.settings.open {
            let panel_w = (540.0 * unit).round();
            let row_h = (44.0 * unit).round();
            let row_gap = (10.0 * unit).round();
            let header_h = (54.0 * unit).round();
            let pad = (22.0 * unit).round();
            let opts = self.settings_options("");
            let panel_h = pad * 2.0 + header_h + (opts.len() as f32) * (row_h + row_gap);
            let panel_x = ((sw - panel_w) * 0.5).round();
            let panel_y = ((sh - panel_h) * 0.5).round();

            if px < panel_x || px > panel_x + panel_w || py < panel_y || py > panel_y + panel_h {
                self.settings.open = false;
                return (None, UiAction::None);
            }

            let list_top = panel_y + pad + header_h;
            for idx in 0..opts.len() {
                let ry = list_top + (idx as f32) * (row_h + row_gap);
                if py >= ry && py <= ry + row_h {
                    self.settings.selected = idx;
                    let (_, act) = self.activate_selected_setting(1);
                    return (None, act);
                }
            }
            return (None, UiAction::None);
        }

        // Check Top-Right Weather & Control Center circular pills
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
                return (None, UiAction::None);
            }
            if px >= gear_x - 6.0 * unit && px <= gear_x + size + 6.0 * unit {
                self.focus_zone = FocusZone::TopControlCenter;
                self.settings.toggle();
                return (None, UiAction::None);
            }
        }

        // Check Dock Icons
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
                            self.settings.toggle();
                            return (None, UiAction::None);
                        }
                        return (Some(tile.package_name.clone()), UiAction::None);
                    }
                }
            }
        }

        (None, UiAction::None)
    }

    pub fn tick_animations(&mut self) -> bool {
        let mut busy = false;
        if self.focus_scales.len() != self.apps.len() {
            self.focus_scales.resize(self.apps.len(), 1.0);
        }
        for (i, sc) in self.focus_scales.iter_mut().enumerate() {
            let target = if self.focus_zone == FocusZone::Dock
                && i == self.selected_index
                && !self.settings.open
            {
                1.15
            } else {
                1.0
            };
            let diff = target - *sc;
            if diff.abs() > 0.004 {
                *sc += diff * 0.32;
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
    ) {
        self.ensure_procedural_scene();

        let sw = pm.width() as f32;
        let sh = pm.height() as f32;
        let unit = (sw / DESIGN_W).min(sh / DESIGN_H);
        let scene = self.current_scene();

        // 1. Layer 1: Full-screen scenic background
        if let Some(ref wp) = self.wallpaper_pm {
            draw_cover_pixmap(pm, wp);
        } else {
            pm.fill(Color::from_rgba8(14, 18, 32, 255));
        }

        // 2. Layer 2: Giant 1:1 Crisp Glass Clock (drawn BEFORE foreground mountain cutout so peaks overlap!)
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

        // 3. Layer 3: Foreground mountain cutout overlapping lower clock digits
        if let Some(ref fg) = self.foreground_pm {
            draw_cover_pixmap(pm, fg);
        }

        // 4. Layer 4: Centered Greeting & Date Header above the clock
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

        let date_w = measure_label_width(self.font_medium.as_ref(), date_str, date_px);
        draw_crisp_label(
            pm,
            self.font_medium.as_ref(),
            date_str,
            ((sw - date_w) * 0.5).round() as i32,
            date_y as i32,
            date_px,
            Color::from_rgba8(255, 255, 255, 245),
            true,
        );

        // 5. Layer 5: Top-right circular frosted glass Weather & Control Center pills (with D-Pad focus ring!)
        self.draw_top_right_pills(pm, sw, unit);

        // 6. Layer 6: Centered Floating Liquid Glass Squircle Dock + Floating Tooltip Pill
        self.draw_floating_dock(pm, sw, sh, unit);

        // 7. Layer 7: Weather & Scene Card (when opened from top-right Weather pill)
        if self.weather_popup_open {
            self.draw_weather_card(pm, sw, sh, unit, time_str, date_str);
        }

        // 8. Layer 8: Rounded-Pill Control Center & Settings Overlay (when opened from top-right ☰ or dock)
        if self.settings.open {
            let opts = self.settings_options(last_key_label);
            self.settings.draw(pm, unit, &opts);
        }
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
            let focused = self.focus_zone == zone && !self.settings.open;
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

                let mid_x = cx + size * 0.5;
                let mid_y = cy + size * 0.5;
                if zone == FocusZone::TopWeather {
                    if let Some(cloud) = rounded_rect_path(
                        mid_x - 7.5 * unit,
                        mid_y - 2.5 * unit,
                        15.0 * unit,
                        7.5 * unit,
                        3.7 * unit,
                    ) {
                        let mut wp = Paint::default();
                        wp.anti_alias = true;
                        wp.set_color(Color::from_rgba8(244, 246, 252, 240));
                        pm.fill_path(&cloud, &wp, FillRule::Winding, Transform::identity(), None);
                    }
                } else {
                    let mut wp = Paint::default();
                    wp.anti_alias = true;
                    wp.set_color(Color::from_rgba8(244, 246, 252, 240));
                    for row in [-4.0f32, 0.0, 4.0] {
                        if let Some(bar) = rounded_rect_path(
                            mid_x - 6.5 * unit,
                            mid_y + (row - 0.8) * unit,
                            13.0 * unit,
                            1.6 * unit,
                            0.8 * unit,
                        ) {
                            pm.fill_path(&bar, &wp, FillRule::Winding, Transform::identity(), None);
                        }
                    }
                }
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
        if let Some(card) = rounded_rect_path(cx, cy, cw, ch, 24.0 * unit) {
            let mut bg = Paint::default();
            bg.anti_alias = true;
            bg.set_color(Color::from_rgba8(20, 25, 38, 236));
            pm.fill_path(&card, &bg, FillRule::Winding, Transform::identity(), None);

            let mut rim = Paint::default();
            rim.anti_alias = true;
            rim.set_color(Color::from_rgba8(255, 255, 255, 85));
            let mut st = Stroke::default();
            st.width = (1.2 * unit).max(1.0);
            pm.stroke_path(&card, &rim, &st, Transform::identity(), None);
        }

        let pad = (22.0 * unit).round();
        draw_crisp_label(
            pm,
            self.font_medium.as_ref(),
            &format!("{}  ·  {}", time_str, date_str),
            (cx + pad) as i32,
            (cy + 36.0 * unit) as i32,
            (15.0 * unit).round(),
            Color::from_rgba8(255, 255, 255, 250),
            false,
        );
        draw_crisp_label(
            pm,
            self.font_regular.as_ref(),
            &format!("Active Scene: {}", self.current_scene().label),
            (cx + pad) as i32,
            (cy + 72.0 * unit) as i32,
            (13.5 * unit).round(),
            Color::from_rgba8(150, 215, 255, 240),
            false,
        );
        draw_crisp_label(
            pm,
            self.font_regular.as_ref(),
            "Left/Right: Switch Scene  ·  Back/Down: Close",
            (cx + pad) as i32,
            (cy + 114.0 * unit) as i32,
            (12.0 * unit).round(),
            Color::from_rgba8(200, 210, 225, 210),
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
        let dock_has_focus = self.focus_zone == FocusZone::Dock && !self.settings.open;

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
"""


RUST_LIB_RS = """//! FerroDeck — Event-driven Rust NativeActivity TV Launcher (`android-activity` 0.6).

mod clock;
mod input;
mod settings_pill;
mod ui;

use crate::input::{
    BoundAction, RemoteMapper, AKEYCODE_BACK, AKEYCODE_DPAD_CENTER, AKEYCODE_DPAD_DOWN,
    AKEYCODE_DPAD_LEFT, AKEYCODE_DPAD_RIGHT, AKEYCODE_DPAD_UP, AKEYCODE_ENTER, AKEYCODE_HOME,
};
use crate::ui::{AppTile, FocusZone, LauncherUi, UiAction, SETTINGS_TILE_PKG};
use android_activity::input::{InputEvent, KeyAction, MotionAction};
use android_activity::{AndroidApp, AndroidAppWaker, InputStatus, MainEvent, PollEvent};
use jni::objects::{JByteArray, JClass, JObject, JObjectArray, JString, JValue};
use jni::sys::jint;
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
    pending_voice_search: bool,
    pending_settings_action: Option<&'static str>,
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
        pending_voice_search: false,
        pending_settings_action: None,
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

fn format_clock_greeting_date(tz_offset_ms: i64, user_name: &str) -> (String, String, String) {
    let utc_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let local_secs = (utc_ms + tz_offset_ms).div_euclid(1000);
    let total_mins = local_secs.div_euclid(60).rem_euclid(24 * 60);
    let h24 = (total_mins / 60) as u32;
    let m = (total_mins % 60) as u32;
    let h12 = match h24 % 12 {
        0 => 12,
        v => v,
    };
    let time_str = format!("{}:{:02}", h12, m);

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
        format_clock_greeting_date(st.tz_offset_ms, &st.ui.user_name);
    let key_label = st.mapper.last_captured_label.clone();
    st.ui
        .render(&mut pixmap, &time_str, &greeting_str, &date_str, &key_label);

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

fn queue_ui_action(st: &mut SharedState, act: UiAction) {
    match act {
        UiAction::None => {}
        UiAction::OpenWifiSettings => st.pending_settings_action = Some("android.settings.WIFI_SETTINGS"),
        UiAction::OpenDisplaySettings => st.pending_settings_action = Some("android.settings.DISPLAY_SETTINGS"),
        UiAction::OpenAccessibilitySettings => st.pending_settings_action = Some("android.settings.ACCESSIBILITY_SETTINGS"),
        UiAction::OpenAndroidSettings => st.pending_settings_action = Some("android.settings.SETTINGS"),
    }
}

fn dispatch_java_side_effects(app: &AndroidApp, st: &mut SharedState) {
    let launch_pkg = st.pending_launch_pkg.take();
    let do_voice = std::mem::take(&mut st.pending_voice_search);
    let settings_act = st.pending_settings_action.take();

    if launch_pkg.is_none() && !do_voice && settings_act.is_none() {
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
            if do_voice {
                let _ = env.call_method(&act_obj, "triggerVoiceAssistFromRust", "()V", &[]);
            }
            if let Some(action_str) = settings_act {
                if let Ok(j_act) = env.new_string(action_str) {
                    let _ = env.call_method(
                        &act_obj,
                        "openSystemActionFromRust",
                        "(Ljava/lang/String;)V",
                        &[(&j_act).into()],
                    );
                }
            }
        }
    }
}

fn apply_bound_action(st: &mut SharedState, act: BoundAction) {
    match act {
        BoundAction::OpenSettingsPill => {
            st.ui.weather_popup_open = false;
            st.ui.settings.toggle();
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
            st.pending_voice_search = true;
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

    match kc {
        AKEYCODE_DPAD_LEFT => {
            st.ui.move_dpad(-1, 0);
            st.needs_redraw = true;
            true
        }
        AKEYCODE_DPAD_RIGHT => {
            st.ui.move_dpad(1, 0);
            st.needs_redraw = true;
            true
        }
        AKEYCODE_DPAD_UP => {
            st.ui.move_dpad(0, -1);
            st.needs_redraw = true;
            true
        }
        AKEYCODE_DPAD_DOWN => {
            st.ui.move_dpad(0, 1);
            st.needs_redraw = true;
            true
        }
        AKEYCODE_DPAD_CENTER | AKEYCODE_ENTER => {
            if st.ui.settings.open {
                if st.ui.settings.selected == 4 {
                    st.mapper.capture_mode = true;
                    st.mapper.last_captured_label = "Press button now...".into();
                } else {
                    let (dirty, act) = st.ui.activate_selected_setting(1);
                    if dirty {
                        st.scene_dirty = true;
                    }
                    queue_ui_action(st, act);
                }
            } else if st.ui.weather_popup_open {
                st.ui.weather_popup_open = false;
            } else {
                match st.ui.focus_zone {
                    FocusZone::TopWeather => {
                        st.ui.weather_popup_open = true;
                    }
                    FocusZone::TopControlCenter => {
                        st.ui.settings.toggle();
                    }
                    FocusZone::Dock => {
                        if let Some(app_tile) = st.ui.apps.get(st.ui.selected_index) {
                            if app_tile.package_name == SETTINGS_TILE_PKG {
                                st.ui.settings.toggle();
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
            if st.ui.settings.open {
                st.ui.settings.open = false;
                st.needs_redraw = true;
            } else if st.ui.weather_popup_open {
                st.ui.weather_popup_open = false;
                st.needs_redraw = true;
            } else if st.ui.focus_zone != FocusZone::Dock {
                st.ui.focus_zone = FocusZone::Dock;
                st.needs_redraw = true;
            }
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
                                let (pkg, act) = st.ui.handle_tap(x, y, sw, sh);
                                if let Some(p) = pkg {
                                    st.pending_launch_pkg = Some(p);
                                }
                                queue_ui_action(&mut st, act);
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
"""


JAVA_MAIN_ACTIVITY = """package com.eadxmz.ferrodeck;

import android.app.NativeActivity;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.content.pm.ResolveInfo;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.drawable.Drawable;
import android.os.Bundle;
import android.provider.Settings;
import android.view.KeyEvent;
import android.view.View;

import java.nio.ByteBuffer;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

public class MainActivity extends NativeActivity {
    static {
        System.loadLibrary("ferrodeck");
    }

    public static native void nativeOnHomePressed();
    public static native void nativeOnRemoteKey(int keyCode, int scanCode);
    public static native void nativeSetApps(String[] packages, String[] labels, byte[][] icons, int iconSize);

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        resetTvFocusLock();
        refreshInstalledApps();
        handleSpecialIntent(getIntent());
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);
        resetTvFocusLock();
        nativeOnHomePressed();
        handleSpecialIntent(intent);
    }

    @Override
    protected void onResume() {
        super.onResume();
        resetTvFocusLock();
        refreshInstalledApps();
    }

    @Override
    public void onWindowFocusChanged(boolean hasFocus) {
        super.onWindowFocusChanged(hasFocus);
        if (hasFocus) {
            resetTvFocusLock();
        }
    }

    private void resetTvFocusLock() {
        try {
            View decor = getWindow().getDecorView();
            decor.setFocusable(true);
            decor.setFocusableInTouchMode(true);
            decor.requestFocusFromTouch();
            decor.requestFocus();
        } catch (Throwable ignored) { }
    }

    private void handleSpecialIntent(Intent intent) {
        if (intent == null) return;
        String action = intent.getAction();
        if (Intent.ACTION_ASSIST.equals(action)
                || "android.speech.action.WEB_SEARCH".equals(action)
                || "android.intent.action.VOICE_COMMAND".equals(action)) {
            nativeOnRemoteKey(KeyEvent.KEYCODE_VOICE_ASSIST, 0);
        }
    }

    @Override
    public boolean dispatchKeyEvent(KeyEvent event) {
        if (event != null && event.getAction() == KeyEvent.ACTION_DOWN) {
            int kc = event.getKeyCode();
            int sc = event.getScanCode();
            if ((kc == KeyEvent.KEYCODE_UNKNOWN && sc > 0)
                    || kc == KeyEvent.KEYCODE_VOICE_ASSIST
                    || kc == KeyEvent.KEYCODE_ASSIST
                    || kc == KeyEvent.KEYCODE_PROG_YELLOW) {
                nativeOnRemoteKey(kc, sc);
                return true;
            }
        }
        return super.dispatchKeyEvent(event);
    }

    private static byte[] drawableToRgba(Drawable d, int size) {
        if (d == null) return null;
        try {
            Bitmap bmp = Bitmap.createBitmap(size, size, Bitmap.Config.ARGB_8888);
            Canvas c = new Canvas(bmp);
            d.setBounds(0, 0, size, size);
            d.draw(c);
            ByteBuffer buf = ByteBuffer.allocate(size * size * 4);
            bmp.copyPixelsToBuffer(buf);
            bmp.recycle();
            return buf.array();
        } catch (Throwable t) {
            return null;
        }
    }

    public void refreshInstalledApps() {
        PackageManager pm = getPackageManager();
        Map<String, ResolveInfo> found = new LinkedHashMap<>();

        Intent leanback = new Intent(Intent.ACTION_MAIN);
        leanback.addCategory(Intent.CATEGORY_LEANBACK_LAUNCHER);
        for (ResolveInfo ri : pm.queryIntentActivities(leanback, 0)) {
            if (ri.activityInfo == null) continue;
            String pkg = ri.activityInfo.packageName;
            if (!getPackageName().equals(pkg)) {
                found.put(pkg, ri);
            }
        }

        Intent standard = new Intent(Intent.ACTION_MAIN);
        standard.addCategory(Intent.CATEGORY_LAUNCHER);
        for (ResolveInfo ri : pm.queryIntentActivities(standard, 0)) {
            if (ri.activityInfo == null) continue;
            String pkg = ri.activityInfo.packageName;
            if (!getPackageName().equals(pkg) && !found.containsKey(pkg)) {
                found.put(pkg, ri);
            }
        }

        List<Map.Entry<String, ResolveInfo>> entries = new ArrayList<>(found.entrySet());
        Collections.sort(entries, (a, b) -> a.getValue().loadLabel(pm).toString()
                .compareToIgnoreCase(b.getValue().loadLabel(pm).toString()));

        final int iconSize = 96;
        String[] pkgs = new String[entries.size()];
        String[] lbls = new String[entries.size()];
        byte[][] icons = new byte[entries.size()][];
        for (int i = 0; i < entries.size(); i++) {
            String pkg = entries.get(i).getKey();
            ResolveInfo ri = entries.get(i).getValue();
            pkgs[i] = pkg;
            lbls[i] = ri.loadLabel(pm).toString();
            Drawable iconDrw = null;
            try {
                iconDrw = pm.getApplicationIcon(pkg);
            } catch (Throwable ignored) {
                iconDrw = ri.loadIcon(pm);
            }
            icons[i] = drawableToRgba(iconDrw, iconSize);
        }
        nativeSetApps(pkgs, lbls, icons, iconSize);
    }

    public void launchPackageFromRust(String pkg) {
        if (pkg == null || pkg.isEmpty()) return;
        try {
            PackageManager pm = getPackageManager();
            Intent i = pm.getLeanbackLaunchIntentForPackage(pkg);
            if (i == null) i = pm.getLaunchIntentForPackage(pkg);
            if (i != null) {
                i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                startActivity(i);
            }
        } catch (Throwable ignored) { }
    }

    public void triggerVoiceAssistFromRust() {
        try {
            Intent voice = new Intent("android.speech.action.WEB_SEARCH");
            voice.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            startActivity(voice);
        } catch (Throwable ignored) { }
    }

    public void openSystemActionFromRust(String action) {
        try {
            Intent i = new Intent(action != null && !action.isEmpty() ? action : Settings.ACTION_SETTINGS);
            i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            startActivity(i);
        } catch (Throwable ignored) {
            try {
                Intent fallback = new Intent(Settings.ACTION_SETTINGS);
                fallback.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                startActivity(fallback);
            } catch (Throwable ignored2) { }
        }
    }
}
"""


def write_lf(path: Path, content: str):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8", newline="\n")


def find_and_extract_tvos_zip(root: Path):
    home = Path.home()
    search_dirs = [
        root,
        root.parent,
        home / "Downloads",
        home / "Documents",
        home / "Desktop",
        home / "OneDrive" / "Documents",
        home / "OneDrive" / "Downloads",
        home / "OneDrive" / "Desktop",
    ]
    for d in search_dirs:
        zp = d / "tvOS-Launcher.zip"
        if zp.is_file():
            keep_prefixes = (
                "assets/backgrounds/",
                "assets/fonts/",
                "assets/app-logos/",
                "assets/icons/",
                "assets/sounds/",
            )
            copied = 0
            with zipfile.ZipFile(zp, "r") as zf:
                for member in zf.infolist():
                    if not member.is_dir() and member.filename.startswith(keep_prefixes):
                        dest = root / member.filename
                        dest.parent.mkdir(parents=True, exist_ok=True)
                        with zf.open(member) as src, open(dest, "wb") as dst:
                            shutil.copyfileobj(src, dst)
                        copied += 1
            print(f"[+] Extracted {copied} original Night Scenes backgrounds & fonts from {zp}")
            return
    print("[i] tvOS-Launcher.zip not found — built-in Android Roboto fonts & procedural Night Scenes mountains are active!")


def main():
    root = Path.cwd()
    if not (root / "AndroidManifest.xml").exists() and (root / "FerroDeck" / "AndroidManifest.xml").exists():
        root = root / "FerroDeck"
    if not (root / "AndroidManifest.xml").exists():
        raise SystemExit(f"Run this script inside your FerroDeck folder (current: {root})")

    find_and_extract_tvos_zip(root)
    write_lf(root / "src/ui.rs", RUST_UI_RS)
    write_lf(root / "src/lib.rs", RUST_LIB_RS)
    write_lf(root / "java/com/eadxmz/ferrodeck/MainActivity.java", JAVA_MAIN_ACTIVITY)

    print("[✓] Updated src/ui.rs, src/lib.rs, and MainActivity.java!")
    print("    Run:")
    print("      git add .")
    print("      git commit -m \"Add system font fallback, procedural Night Scenes, top-bar D-pad focus, and touch support\"")
    print("      git push")


if __name__ == "__main__":
    main()

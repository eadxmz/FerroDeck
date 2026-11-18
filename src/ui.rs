//! Exact Night Scenes + Liquid Glass Squircle Dock renderer in `tiny-skia`.
//! Matches the visual composition of the original Night Scenes layout:
//!   1. Full-screen scenic backdrop (`scene-night.jpg`, `scene-golden.jpg`, `scene-fog.jpg`, etc.)
//!   2. Centered greeting ("Good evening, sadim") + date header
//!   3. Giant 1:1 unblurred glass clock (`Antonio-Light`) layered BEHIND the foreground mountain
//!      cutout (`scene-night-fg.webp`) so peaks overlap the lower clock digits
//!   4. Top-right circular frosted glass Weather & Settings buttons
//!   5. Centered floating Liquid Glass squircle dock (`1:1` rounded-square app icons + Settings
//!      tile) with a floating macOS-style pointer tooltip bubble above the focused app.

use crate::clock::{draw_crisp_label, measure_label_width, CrispClock};
use crate::settings_pill::{rounded_rect_path, PillOption, PillSettingsPanel};
use fontdue::{Font, FontSettings};
use tiny_skia::{
    Color, FillRule, FilterQuality, Paint, Pattern, Pixmap, PixmapPaint, SpreadMode, Stroke,
    Transform,
};

pub const DESIGN_W: f32 = 960.0;
pub const DESIGN_H: f32 = 540.0;
pub const MAX_DOCK_TILES: usize = 10;
pub const SETTINGS_TILE_PKG: &str = "__ferrodeck_settings__";

#[derive(Clone, Debug)]
pub struct SceneSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub bg_asset: &'static str,
    pub fg_asset: Option<&'static str>,
    pub clock_top_u: f32,
    pub clock_size_u: f32,
    pub date_top_u: f32,
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
    },
    SceneSpec {
        id: "golden",
        label: "Golden Hour",
        bg_asset: "backgrounds/scene-golden.jpg",
        fg_asset: Some("backgrounds/scene-golden-fg.webp"),
        clock_top_u: 76.0,
        clock_size_u: 195.0,
        date_top_u: 53.0,
    },
    SceneSpec {
        id: "fog",
        label: "Fog",
        bg_asset: "backgrounds/scene-fog.jpg",
        fg_asset: Some("backgrounds/scene-fog-fg.webp"),
        clock_top_u: 86.0,
        clock_size_u: 210.0,
        date_top_u: 58.0,
    },
    SceneSpec {
        id: "dawn",
        label: "Rainforest Dawn",
        bg_asset: "backgrounds/scene-dawn.jpg",
        fg_asset: Some("backgrounds/scene-dawn-fg.webp"),
        clock_top_u: 115.0,
        clock_size_u: 190.0,
        date_top_u: 88.0,
    },
    SceneSpec {
        id: "water",
        label: "Waterline",
        bg_asset: "backgrounds/scene-water.jpg",
        fg_asset: None,
        clock_top_u: 110.0,
        clock_size_u: 195.0,
        date_top_u: 82.0,
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

pub struct LauncherUi {
    pub apps: Vec<AppTile>,
    pub selected_index: usize,
    pub page_offset: usize,
    pub clock_style: u8, // 0 = Giant Scene Glass Clock, 1 = Top-Right Crisp, 2 = Hidden
    pub scene_idx: usize,
    pub user_name: String,
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
        let font_medium = med_font.or(bold_font).and_then(|b| {
            Font::from_bytes(
                b,
                FontSettings {
                    scale: 32.0,
                    ..Default::default()
                },
            )
            .ok()
        });
        let font_regular = reg_font.or(med_font).and_then(|b| {
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
            selected_index: 0,
            page_offset: 0,
            clock_style: 0,
            scene_idx: 0,
            user_name: "sadim".into(),
            wallpaper_pm: None,
            foreground_pm: None,
            glass_backdrop_pm: None,
            settings_icon_pm: settings_icon,
            focus_scales: Vec::new(),
            font_medium,
            font_regular,
            clock: CrispClock::new(big_font, bold_font),
            settings: PillSettingsPanel::new(bold_font, reg_font),
        }
    }

    pub fn current_scene(&self) -> &'static SceneSpec {
        &SCENES[self.scene_idx % SCENES.len()]
    }

    pub fn set_apps(&mut self, mut apps: Vec<AppTile>) {
        // Append the built-in FerroDeck Settings gear tile at the end of the dock
        apps.push(AppTile {
            package_name: SETTINGS_TILE_PKG.into(),
            label: "FerroDeck Settings".into(),
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

    /// Resets transient overlays and ensures D-Pad selection responds immediately
    /// when returning from sideloaded apps like Coji Browser via the Home key.
    pub fn on_home_pressed(&mut self) {
        self.settings.open = false;
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
                self.activate_selected_setting(dx);
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

    pub fn activate_selected_setting(&mut self, dir: isize) -> bool {
        match self.settings.selected {
            0 => {
                // Scene switcher (Night -> Golden Hour -> Fog -> Rainforest Dawn -> Waterline)
                let n = SCENES.len() as isize;
                self.scene_idx =
                    (self.scene_idx as isize + if dir >= 0 { 1 } else { -1 }).rem_euclid(n) as usize;
                true
            }
            1 => {
                // Clock Display: Giant Scene Glass -> Top-Right Crisp -> Hidden
                let next = (self.clock_style as isize + if dir >= 0 { 1 } else { -1 }).rem_euclid(3);
                self.clock_style = next as u8;
                false
            }
            _ => false,
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
                title: "Capture Remote Key / Mic".into(),
                value: last_key_label.into(),
            },
            PillOption {
                title: "Accessibility Key Filter".into(),
                value: "Open Android Settings".into(),
            },
            PillOption {
                title: "Coji Browser Home Lock Fix".into(),
                value: "Active (Native Focus)".into(),
            },
        ]
    }

    pub fn tick_animations(&mut self) -> bool {
        let mut busy = false;
        if self.focus_scales.len() != self.apps.len() {
            self.focus_scales.resize(self.apps.len(), 1.0);
        }
        for (i, sc) in self.focus_scales.iter_mut().enumerate() {
            let target = if i == self.selected_index && !self.settings.open {
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
                let right_cx = sw - (145.0 * unit);
                self.clock
                    .draw_centered(pm, time_str, 1, right_cx, (22.0 * unit).round() as i32, clock_px);
            }
            _ => {}
        }

        // 3. Layer 3: Foreground depth cutout (`scene-night-fg.webp` mountains overlapping lower clock)
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

        // 5. Layer 5: Top-right circular frosted glass Weather & Controls pills
        self.draw_top_right_pills(pm, sw, unit);

        // 6. Layer 6: Centered Floating Liquid Glass Squircle Dock + Floating Tooltip Pill
        self.draw_floating_dock(pm, sw, sh, unit);

        // 7. Layer 7: Rounded-Pill Settings Overlay (when open)
        if self.settings.open {
            let opts = self.settings_options(last_key_label);
            self.settings.draw(pm, unit, &opts);
        }
    }

    fn draw_top_right_pills(&self, pm: &mut Pixmap, sw: f32, unit: f32) {
        let size = (38.0 * unit).round();
        let top_y = (18.0 * unit).round();
        let right_margin = (22.0 * unit).round();
        let gap = (10.0 * unit).round();

        let gear_x = sw - right_margin - size;
        let weather_x = gear_x - gap - size;

        for (idx, cx) in [weather_x, gear_x].iter().enumerate() {
            if let Some(circle) = rounded_rect_path(*cx, top_y, size, size, size * 0.5) {
                let mut bg = Paint::default();
                bg.anti_alias = true;
                bg.set_color(Color::from_rgba8(24, 28, 40, 145));
                pm.fill_path(&circle, &bg, FillRule::Winding, Transform::identity(), None);

                let mut rim = Paint::default();
                rim.anti_alias = true;
                rim.set_color(Color::from_rgba8(255, 255, 255, 85));
                let mut st = Stroke::default();
                st.width = (1.1 * unit).max(1.0);
                pm.stroke_path(&circle, &rim, &st, Transform::identity(), None);

                // Minimal crisp glyph inside each circle pill
                let mid_x = *cx + size * 0.5;
                let mid_y = top_y + size * 0.5;
                if idx == 0 {
                    // Cloud glyph
                    if let Some(cloud) = rounded_rect_path(
                        mid_x - 7.5 * unit,
                        mid_y - 2.5 * unit,
                        15.0 * unit,
                        7.5 * unit,
                        3.7 * unit,
                    ) {
                        let mut wp = Paint::default();
                        wp.anti_alias = true;
                        wp.set_color(Color::from_rgba8(244, 246, 252, 235));
                        pm.fill_path(&cloud, &wp, FillRule::Winding, Transform::identity(), None);
                    }
                } else {
                    // 3-line control sliders glyph
                    let mut wp = Paint::default();
                    wp.anti_alias = true;
                    wp.set_color(Color::from_rgba8(244, 246, 252, 235));
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
        }
    }

    fn draw_floating_dock(&self, pm: &mut Pixmap, sw: f32, sh: f32, unit: f32) {
        if self.apps.is_empty() {
            return;
        }
        let end = (self.page_offset + MAX_DOCK_TILES).min(self.apps.len());
        let visible = &self.apps[self.page_offset..end];
        let count = visible.len();

        // Exact SceneHome.java squircle dock proportions
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

        // 1. Frosted Liquid Glass dock pill background
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

        // 2. Unfocused squircle app icons
        let start_x = dock_x + pad_h;
        let start_y = dock_y + pad_v;
        let rel_sel = self.selected_index.saturating_sub(self.page_offset);

        for (i, app) in visible.iter().enumerate() {
            if i == rel_sel {
                continue;
            }
            let ix = start_x + (i as f32) * (icon_size + icon_gap);
            self.draw_squircle_icon(pm, app, ix, start_y, icon_size, icon_r, 1.0, false, unit);
        }

        // 3. Focused squircle icon (1.15x lift + bright rim + floating macOS tooltip above)
        if let Some(app) = visible.get(rel_sel) {
            let sc = *self.focus_scales.get(self.selected_index).unwrap_or(&1.15);
            let ix = start_x + (rel_sel as f32) * (icon_size + icon_gap);
            self.draw_squircle_icon(pm, app, ix, start_y, icon_size, icon_r, sc, true, unit);

            // Floating tooltip pill centered above the focused icon (`FerroDeck`, `YouTube`, etc.)
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
            // Subtle dark glass backing behind transparent icons
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
                // Draw first letter if no icon bitmap was supplied
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

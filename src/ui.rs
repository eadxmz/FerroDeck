//! Exact 960x540 tvOS Liquid Glass Dock & App Grid geometry rendered via `tiny-skia`.
//! Matches the original Java layout constants (`COLS = 6`, `dockOuter = 18`, `dockInset = 24`,
//! `gap = 18`, `tileH = tileW * 3 / 5`, `radius = tileW * 0.16`) while caching the saturated
//! blurred Liquid Glass backdrop once per wallpaper change for near-zero CPU/GPU cost.

use crate::clock::CrispClock;
use crate::settings_pill::{rounded_rect_path, PillOption, PillSettingsPanel};
use tiny_skia::{
    Color, FillRule, FilterQuality, Paint, Pattern, Pixmap, PixmapPaint, SpreadMode, Stroke,
    Transform,
};

pub const COLS: usize = 6;
pub const DESIGN_W: f32 = 960.0;
pub const DESIGN_H: f32 = 540.0;

#[derive(Clone)]
#[allow(dead_code)]
pub struct AppTile {
    pub package_name: String,
    pub label: String,
    pub icon: Option<Pixmap>,
    pub accent: [u8; 3],
}

pub struct LauncherUi {
    pub apps: Vec<AppTile>,
    pub selected_index: usize,
    pub dock_rows: usize,
    pub clock_style: u8,
    pub wallpaper_idx: usize,
    pub wallpaper_names: Vec<String>,
    wallpaper_pm: Option<Pixmap>,
    glass_backdrop_pm: Option<Pixmap>,
    focus_scales: Vec<f32>,
    pub clock: CrispClock,
    pub settings: PillSettingsPanel,
}

impl LauncherUi {
    pub fn new(
        big_font: Option<&[u8]>,
        bold_font: Option<&[u8]>,
        reg_font: Option<&[u8]>,
    ) -> Self {
        Self {
            apps: Vec::new(),
            selected_index: 0,
            dock_rows: 1,
            clock_style: 0,
            wallpaper_idx: 0,
            wallpaper_names: vec![
                "tessellated-pavement.jpg".into(),
                "grose-valley.jpg".into(),
                "vang-vieng-sunset.jpg".into(),
                "albury-storm.jpg".into(),
                "aresquiers-beach.jpg".into(),
            ],
            wallpaper_pm: None,
            glass_backdrop_pm: None,
            focus_scales: Vec::new(),
            clock: CrispClock::new(big_font, bold_font),
            settings: PillSettingsPanel::new(bold_font, reg_font),
        }
    }

    pub fn set_apps(&mut self, apps: Vec<AppTile>) {
        self.focus_scales = vec![1.0; apps.len()];
        if !self.focus_scales.is_empty() {
            self.focus_scales[0] = 1.16;
        }
        self.apps = apps;
        if self.selected_index >= self.apps.len() {
            self.selected_index = 0;
        }
    }

    pub fn set_wallpaper_rgba(&mut self, w: u32, h: u32, rgba: Vec<u8>) {
        if let Some(size) = tiny_skia::IntSize::from_wh(w, h) {
            if let Some(pm) = Pixmap::from_vec(rgba, size) {
                self.glass_backdrop_pm = Some(bake_liquid_glass(&pm));
                self.wallpaper_pm = Some(pm);
            }
        }
    }

    /// Resets transient overlays and ensures D-Pad selection is immediately responsive
    /// when returning from sideloaded apps like Koji Browser via the Home key.
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
        let mut idx = self.selected_index as isize;
        if dx != 0 {
            idx = (idx + dx).rem_euclid(len as isize);
        }
        if dy != 0 {
            let next = idx + dy * (COLS as isize);
            if next >= 0 && (next as usize) < len {
                idx = next;
            }
        }
        self.selected_index = idx as usize;
    }

    pub fn activate_selected_setting(&mut self, dir: isize) -> bool {
        match self.settings.selected {
            0 => {
                let next = (self.clock_style as isize + if dir >= 0 { 1 } else { -1 }).rem_euclid(3);
                self.clock_style = next as u8;
                false
            }
            1 => {
                let next = ((self.dock_rows as isize - 1 + if dir >= 0 { 1 } else { -1 }).rem_euclid(3)) + 1;
                self.dock_rows = next as usize;
                false
            }
            2 => {
                if !self.wallpaper_names.is_empty() {
                    let n = self.wallpaper_names.len() as isize;
                    self.wallpaper_idx = (self.wallpaper_idx as isize + if dir >= 0 { 1 } else { -1 }).rem_euclid(n) as usize;
                    return true;
                }
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
        let wall_name = self
            .wallpaper_names
            .get(self.wallpaper_idx)
            .map(|s| s.trim_end_matches(".jpg"))
            .unwrap_or("Default");
        vec![
            PillOption { title: "Clock Display".into(), value: clock_val.into() },
            PillOption { title: "Liquid Glass Dock Rows".into(), value: format!("{} Row(s)", self.dock_rows) },
            PillOption { title: "Wallpaper Scene".into(), value: wall_name.into() },
            PillOption { title: "Capture Remote Key / Mic".into(), value: last_key_label.into() },
            PillOption { title: "Accessibility Key Filter".into(), value: "Open Android Settings".into() },
            PillOption { title: "Koji Browser Home Lock Fix".into(), value: "Active (Native Focus)".into() },
        ]
    }

    pub fn tick_animations(&mut self) -> bool {
        let mut busy = false;
        if self.focus_scales.len() != self.apps.len() {
            self.focus_scales.resize(self.apps.len(), 1.0);
        }
        for (i, sc) in self.focus_scales.iter_mut().enumerate() {
            let target = if i == self.selected_index && !self.settings.open { 1.16 } else { 1.0 };
            let diff = target - *sc;
            if diff.abs() > 0.004 {
                *sc += diff * 0.30;
                busy = true;
            } else {
                *sc = target;
            }
        }
        busy
    }

    pub fn render(&mut self, pm: &mut Pixmap, time_str: &str, last_key_label: &str) {
        let sw = pm.width() as f32;
        let sh = pm.height() as f32;
        let unit = (sw / DESIGN_W).min(sh / DESIGN_H);

        if let Some(ref wp) = self.wallpaper_pm {
            let sx = sw / wp.width() as f32;
            let sy = sh / wp.height() as f32;
            pm.draw_pixmap(
                0,
                0,
                wp.as_ref(),
                &PixmapPaint { quality: FilterQuality::Bilinear, ..Default::default() },
                Transform::from_scale(sx, sy),
                None,
            );
        } else {
            pm.fill(Color::from_rgba8(12, 18, 28, 255));
        }

        let dock_outer = (18.0 * unit).round();
        let dock_inset = (24.0 * unit).round();
        let side = dock_outer + dock_inset;
        let gap = (18.0 * unit).round();
        let tile_w = ((sw - 2.0 * side - ((COLS as f32 - 1.0) * gap)) / COLS as f32).round();
        let tile_h = (tile_w * 3.0 / 5.0).round();
        let radius = (tile_w * 0.16).round();
        let dock_pad_v = (19.0 * unit).round();

        let visible_rows = ((self.apps.len() + COLS - 1) / COLS).max(1).min(3);
        let active_dock_rows = self.dock_rows.min(visible_rows);
        let dock_h = active_dock_rows as f32 * tile_h
            + (active_dock_rows.saturating_sub(1) as f32) * gap
            + dock_pad_v * 2.0;
        let dock_x = dock_outer;
        let dock_w = sw - dock_outer * 2.0;
        let dock_y = sh - dock_outer - dock_h;
        let dock_r = radius + dock_inset * 0.45;

        match self.clock_style {
            0 => {
                let clock_px = (92.0 * unit).round();
                let cx = ((sw * 0.5) - (clock_px * 1.35)).round() as i32;
                let cy = ((dock_y * 0.34) - (clock_px * 0.35)).max(24.0 * unit).round() as i32;
                self.clock.draw(pm, time_str, 0, cx, cy, clock_px);
            }
            1 => {
                let clock_px = (26.0 * unit).round();
                let cx = (sw - side - clock_px * 3.1).round() as i32;
                let cy = (26.0 * unit).round() as i32;
                self.clock.draw(pm, time_str, 1, cx, cy, clock_px);
            }
            _ => {}
        }

        if let Some(dock_path) = rounded_rect_path(dock_x, dock_y, dock_w, dock_h, dock_r) {
            if let Some(ref glass) = self.glass_backdrop_pm {
                let sx = sw / glass.width() as f32;
                let sy = sh / glass.height() as f32;
                let mut glass_paint = Paint::default();
                glass_paint.anti_alias = true;
                glass_paint.shader = Pattern::new(
                    glass.as_ref(),
                    SpreadMode::Pad,
                    FilterQuality::Bilinear,
                    0.92,
                    Transform::from_scale(sx, sy),
                );
                pm.fill_path(&dock_path, &glass_paint, FillRule::Winding, Transform::identity(), None);
            }

            let mut tint = Paint::default();
            tint.anti_alias = true;
            tint.set_color(Color::from_rgba8(255, 255, 255, 26));
            pm.fill_path(&dock_path, &tint, FillRule::Winding, Transform::identity(), None);

            let mut rim = Paint::default();
            rim.anti_alias = true;
            rim.set_color(Color::from_rgba8(255, 255, 255, 88));
            let mut st = Stroke::default();
            st.width = (1.4 * unit).max(1.0);
            pm.stroke_path(&dock_path, &rim, &st, Transform::identity(), None);
        }

        let max_cards = visible_rows * COLS;
        for (i, app) in self.apps.iter().take(max_cards).enumerate() {
            if i == self.selected_index {
                continue;
            }
            self.draw_card(pm, app, i, side, dock_y + dock_pad_v, tile_w, tile_h, gap, radius, 1.0, false, unit);
        }
        if let Some(app) = self.apps.get(self.selected_index) {
            let sc = *self.focus_scales.get(self.selected_index).unwrap_or(&1.16);
            self.draw_card(
                pm,
                app,
                self.selected_index,
                side,
                dock_y + dock_pad_v,
                tile_w,
                tile_h,
                gap,
                radius,
                sc,
                true,
                unit,
            );
        }

        if self.settings.open {
            let opts = self.settings_options(last_key_label);
            self.settings.draw(pm, unit, &opts);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_card(
        &self,
        pm: &mut Pixmap,
        app: &AppTile,
        idx: usize,
        start_x: f32,
        start_y: f32,
        tile_w: f32,
        tile_h: f32,
        gap: f32,
        radius: f32,
        scale: f32,
        focused: bool,
        unit: f32,
    ) {
        let col = (idx % COLS) as f32;
        let row = (idx / COLS) as f32;
        let base_x = start_x + col * (tile_w + gap);
        let base_y = start_y + row * (tile_h + gap);

        let w = (tile_w * scale).round();
        let h = (tile_h * scale).round();
        let x = (base_x - (w - tile_w) * 0.5).round();
        let y = (base_y - (h - tile_h) * 0.5).round();
        let r = (radius * scale).round();

        if focused {
            if let Some(sh_path) = rounded_rect_path(x - 2.0 * unit, y + 6.0 * unit, w + 4.0 * unit, h + 4.0 * unit, r + 2.0) {
                let mut sp = Paint::default();
                sp.anti_alias = true;
                sp.set_color(Color::from_rgba8(0, 0, 0, 115));
                pm.fill_path(&sh_path, &sp, FillRule::Winding, Transform::identity(), None);
            }
        }

        if let Some(card_path) = rounded_rect_path(x, y, w, h, r) {
            let mut bg = Paint::default();
            bg.anti_alias = true;
            bg.set_color(Color::from_rgba8(app.accent[0], app.accent[1], app.accent[2], 235));
            pm.fill_path(&card_path, &bg, FillRule::Winding, Transform::identity(), None);

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
                pm.fill_path(&card_path, &ip, FillRule::Winding, Transform::identity(), None);
            }

            if focused {
                let mut rim = Paint::default();
                rim.anti_alias = true;
                rim.set_color(Color::from_rgba8(255, 255, 255, 235));
                let mut st = Stroke::default();
                st.width = (2.2 * unit).max(1.5);
                pm.stroke_path(&card_path, &rim, &st, Transform::identity(), None);
            }
        }
    }
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
        &PixmapPaint { quality: FilterQuality::Bilinear, ..Default::default() },
        Transform::from_scale(sx, sy),
        None,
    );

    for px in out.pixels_mut() {
        let r = px.red() as f32;
        let g = px.green() as f32;
        let b = px.blue() as f32;
        let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        let nr = (lum + (r - lum) * 1.6 + 14.0).clamp(0.0, 255.0) as u8;
        let ng = (lum + (g - lum) * 1.6 + 14.0).clamp(0.0, 255.0) as u8;
        let nb = (lum + (b - lum) * 1.6 + 14.0).clamp(0.0, 255.0) as u8;
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

//! Full-Screen Two-Pane Home Settings with Fluid Open/Close Transitions & Dual Gliding Pills.

use crate::anim::{ease_out_cubic, smooth_to, with_alpha};
use crate::clock::{draw_crisp_label, measure_label_width};
use crate::glyphs::{draw_glyph, Glyph};
use fontdue::{Font, FontSettings};
use tiny_skia::{
    Color, FillRule, FilterQuality, Paint, Path, PathBuilder, Pattern, Pixmap, Rect, SpreadMode,
    Stroke, Transform,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SettingsCategory {
    Appearance = 0,
    Weather = 1,
    Apps = 2,
    People = 3,
    ControlPanel = 4,
    ButtonMapper = 5,
    QuickSettings = 6,
    Language = 7,
    Advanced = 8,
    Updates = 9,
    About = 10,
}

impl SettingsCategory {
    pub const ALL: [SettingsCategory; 11] = [
        SettingsCategory::Appearance,
        SettingsCategory::Weather,
        SettingsCategory::Apps,
        SettingsCategory::People,
        SettingsCategory::ControlPanel,
        SettingsCategory::ButtonMapper,
        SettingsCategory::QuickSettings,
        SettingsCategory::Language,
        SettingsCategory::Advanced,
        SettingsCategory::Updates,
        SettingsCategory::About,
    ];

    pub fn title(self) -> &'static str {
        match self {
            SettingsCategory::Appearance => "Appearance and scenes",
            SettingsCategory::Weather => "Weather and location",
            SettingsCategory::Apps => "Apps",
            SettingsCategory::People => "People",
            SettingsCategory::ControlPanel => "Control Panel",
            SettingsCategory::ButtonMapper => "Button mapper",
            SettingsCategory::QuickSettings => "Quick settings",
            SettingsCategory::Language => "Language",
            SettingsCategory::Advanced => "Advanced",
            SettingsCategory::Updates => "Updates",
            SettingsCategory::About => "About",
        }
    }
}

#[derive(Clone, Debug)]
pub struct SettingRowItem {
    pub section: &'static str,
    pub title: String,
    pub subtitle: Option<String>,
    pub value: String,
    pub checked: Option<bool>,
    pub action_id: &'static str,
}

pub struct VisibleRowLayout {
    pub abs_idx: usize,
    pub header: Option<&'static str>,
    pub header_y: f32,
    pub pill_y: f32,
    pub pill_h: f32,
}

pub struct HomeSettingsScreen {
    pub open: bool,
    pub open_anim: f32,
    pub category_idx: usize,
    pub row_idx: usize,
    pub in_right_pane: bool,
    pub status_banner: Option<String>,
    pub side_pill_y_u: f32,
    pub right_pill_y_u: f32,
    pub right_pill_target_u: f32,
    font_display: Option<Font>,
    font_medium: Option<Font>,
    font_regular: Option<Font>,
}

impl HomeSettingsScreen {
    pub fn new(
        bold_bytes: Option<&[u8]>,
        med_bytes: Option<&[u8]>,
        reg_bytes: Option<&[u8]>,
    ) -> Self {
        Self {
            open: false,
            open_anim: 0.0,
            category_idx: 0,
            row_idx: 0,
            in_right_pane: false,
            status_banner: None,
            side_pill_y_u: -1.0,
            right_pill_y_u: -1.0,
            right_pill_target_u: -1.0,
            font_display: bold_bytes.and_then(|b| {
                Font::from_bytes(
                    b,
                    FontSettings {
                        scale: 48.0,
                        ..Default::default()
                    },
                )
                .ok()
            }),
            font_medium: med_bytes.or(bold_bytes).and_then(|b| {
                Font::from_bytes(
                    b,
                    FontSettings {
                        scale: 32.0,
                        ..Default::default()
                    },
                )
                .ok()
            }),
            font_regular: reg_bytes.or(med_bytes).and_then(|b| {
                Font::from_bytes(
                    b,
                    FontSettings {
                        scale: 32.0,
                        ..Default::default()
                    },
                )
                .ok()
            }),
        }
    }

    pub fn open_at(&mut self, cat: SettingsCategory) {
        self.open = true;
        self.category_idx = cat as usize;
        self.row_idx = 0;
        self.in_right_pane = false;
        self.status_banner = None;
        self.side_pill_y_u = 74.0 + (self.category_idx as f32) * 35.0;
        self.right_pill_y_u = -1.0;
        self.right_pill_target_u = -1.0;
    }

    pub fn is_visible(&self) -> bool {
        self.open || self.open_anim > 0.005
    }

    pub fn current_category(&self) -> SettingsCategory {
        SettingsCategory::ALL[self.category_idx % SettingsCategory::ALL.len()]
    }

    pub fn move_up(&mut self, row_count: usize) {
        if self.in_right_pane {
            if row_count > 0 {
                self.row_idx = if self.row_idx == 0 {
                    row_count - 1
                } else {
                    self.row_idx - 1
                };
            }
        } else {
            let n = SettingsCategory::ALL.len();
            self.category_idx = if self.category_idx == 0 {
                n - 1
            } else {
                self.category_idx - 1
            };
            self.row_idx = 0;
            self.right_pill_y_u = -1.0;
        }
    }

    pub fn move_down(&mut self, row_count: usize) {
        if self.in_right_pane {
            if row_count > 0 {
                self.row_idx = (self.row_idx + 1) % row_count;
            }
        } else {
            let n = SettingsCategory::ALL.len();
            self.category_idx = (self.category_idx + 1) % n;
            self.row_idx = 0;
            self.right_pill_y_u = -1.0;
        }
    }

    pub fn move_right(&mut self, row_count: usize) {
        if !self.in_right_pane && row_count > 0 {
            self.in_right_pane = true;
            self.row_idx = self.row_idx.min(row_count - 1);
            self.right_pill_y_u = -1.0;
        }
    }

    pub fn move_left(&mut self) {
        if self.in_right_pane {
            self.in_right_pane = false;
        }
    }

    pub fn tick(&mut self, dt: f32) -> bool {
        let mut busy = smooth_to(
            &mut self.open_anim,
            if self.open { 1.0 } else { 0.0 },
            15.0,
            dt,
            0.003,
        );
        if !self.is_visible() {
            return busy;
        }

        let target_side_u = 74.0 + (self.category_idx as f32) * 35.0;
        if self.side_pill_y_u < 0.0 {
            self.side_pill_y_u = target_side_u;
        } else if smooth_to(&mut self.side_pill_y_u, target_side_u, 22.0, dt, 0.12) {
            busy = true;
        }

        if self.in_right_pane && self.right_pill_target_u >= 0.0 {
            if self.right_pill_y_u < 0.0 {
                self.right_pill_y_u = self.right_pill_target_u;
            } else if smooth_to(&mut self.right_pill_y_u, self.right_pill_target_u, 22.0, dt, 0.12) {
                busy = true;
            }
        }
        busy
    }

    /// Computes exact vertical layout for visible rows including inline section headers
    /// (such as `NATIVE (INDIA)` and `GLOBAL` in Language settings).
    pub fn compute_visible_layout(&self, rows: &[SettingRowItem], unit: f32) -> Vec<VisibleRowLayout> {
        let max_visible = 7usize;
        let page_start = if self.row_idx >= max_visible {
            self.row_idx + 1 - max_visible
        } else {
            0
        };
        let end = (page_start + max_visible).min(rows.len());
        let row_h = (38.0 * unit).round();
        let row_gap = (7.0 * unit).round();
        let header_h = (22.0 * unit).round();

        let mut cursor_y = (68.0 * unit).round();
        let mut last_section: Option<&'static str> = None;
        let mut out = Vec::with_capacity(end.saturating_sub(page_start));

        for abs_i in page_start..end {
            let item = &rows[abs_i];
            let mut header = None;
            let mut header_y = cursor_y;
            if !item.section.is_empty() && last_section != Some(item.section) {
                if last_section.is_some() {
                    cursor_y += (6.0 * unit).round();
                }
                header_y = cursor_y + (14.0 * unit).round();
                header = Some(item.section);
                cursor_y += header_h;
                last_section = Some(item.section);
            }

            let pill_y = cursor_y;
            out.push(VisibleRowLayout {
                abs_idx: abs_i,
                header,
                header_y,
                pill_y,
                pill_h: row_h,
            });
            cursor_y += row_h + row_gap;
        }
        out
    }

    pub fn draw(
        &mut self,
        pm: &mut Pixmap,
        glass_bg: Option<&Pixmap>,
        unit: f32,
        rows: &[SettingRowItem],
    ) {
        if !self.is_visible() {
            return;
        }
        let sw = pm.width() as f32;
        let sh = pm.height() as f32;
        let a = self.open_anim.clamp(0.0, 1.0);
        let slide_y = ((1.0 - ease_out_cubic(a)) * 16.0 * unit).round();

        if let Some(glass) = glass_bg {
            let sx = sw / glass.width() as f32;
            let sy = sh / glass.height() as f32;
            let mut gp = Paint::default();
            gp.shader = Pattern::new(
                glass.as_ref(),
                SpreadMode::Pad,
                FilterQuality::Bilinear,
                a,
                Transform::from_scale(sx, sy),
            );
            if let Some(r) = Rect::from_xywh(0.0, 0.0, sw, sh) {
                pm.fill_rect(r, &gp, Transform::identity(), None);
            }
        }
        let mut scrim = Paint::default();
        scrim.set_color(with_alpha(Color::from_rgba8(10, 11, 18, 206), a));
        if let Some(r) = Rect::from_xywh(0.0, 0.0, sw, sh) {
            pm.fill_rect(r, &scrim, Transform::identity(), None);
        }

        // Left Sidebar
        let side_x = (46.0 * unit).round();
        let side_w = (268.0 * unit).round();
        draw_crisp_label(
            pm,
            self.font_display.as_ref(),
            "Home settings",
            (side_x + 14.0 * unit) as i32,
            (54.0 * unit + slide_y) as i32,
            (22.0 * unit).round(),
            with_alpha(Color::from_rgba8(255, 255, 255, 250), a),
            false,
        );

        let pill_h = (31.0 * unit).round();
        let pill_r = pill_h * 0.5;
        let anim_side_y = (self.side_pill_y_u * unit + slide_y).round();

        if let Some(sp) = rounded_rect_path(side_x, anim_side_y, side_w, pill_h, pill_r) {
            let mut fill = Paint::default();
            fill.anti_alias = true;
            let col = if !self.in_right_pane {
                Color::from_rgba8(245, 247, 250, 248)
            } else {
                Color::from_rgba8(255, 255, 255, 34)
            };
            fill.set_color(with_alpha(col, a));
            pm.fill_path(&sp, &fill, FillRule::Winding, Transform::identity(), None);
        }

        for (i, cat) in SettingsCategory::ALL.iter().enumerate() {
            let ry = ((74.0 + (i as f32) * 35.0) * unit + slide_y).round();
            let is_active = i == self.category_idx;
            let text_col = with_alpha(
                if is_active && !self.in_right_pane {
                    Color::from_rgba8(16, 18, 26, 255)
                } else if is_active {
                    Color::from_rgba8(255, 255, 255, 255)
                } else {
                    Color::from_rgba8(220, 226, 238, 215)
                },
                a,
            );

            let glyph = match cat {
                SettingsCategory::Appearance => Glyph::Scene,
                SettingsCategory::Weather => Glyph::Cloud,
                SettingsCategory::Apps => Glyph::Apps,
                SettingsCategory::People => Glyph::Person,
                SettingsCategory::ControlPanel => Glyph::Sliders,
                SettingsCategory::ButtonMapper => Glyph::Remote,
                SettingsCategory::QuickSettings => Glyph::Bolt,
                SettingsCategory::Language => Glyph::Globe,
                SettingsCategory::Advanced => Glyph::Shield,
                SettingsCategory::Updates => Glyph::Update,
                SettingsCategory::About => Glyph::Info,
            };
            let g_size = (14.5 * unit).round();
            let gx = side_x + 12.0 * unit;
            let gy = ry + (pill_h - g_size) * 0.5;
            draw_glyph(pm, glyph, gx, gy, g_size, text_col);

            draw_crisp_label(
                pm,
                self.font_medium.as_ref(),
                cat.title(),
                (side_x + 32.0 * unit) as i32,
                (ry + pill_h * 0.67) as i32,
                (12.5 * unit).round(),
                text_col,
                false,
            );
        }

        // Right Content Column
        let main_x = (348.0 * unit).round();
        let main_w = (sw - main_x - 52.0 * unit).round();
        let active_cat = self.current_category();

        draw_crisp_label(
            pm,
            self.font_display.as_ref(),
            active_cat.title(),
            (main_x + 12.0 * unit) as i32,
            (54.0 * unit + slide_y) as i32,
            (19.0 * unit).round(),
            with_alpha(Color::from_rgba8(255, 255, 255, 248), a),
            false,
        );

        let layouts = self.compute_visible_layout(rows, unit);

        // Update gliding right-pane focus pill target in design units
        if self.in_right_pane {
            if let Some(foc_lay) = layouts.iter().find(|l| l.abs_idx == self.row_idx) {
                let target_u = foc_lay.pill_y / unit.max(0.01);
                self.right_pill_target_u = target_u;
                if self.right_pill_y_u < 0.0 {
                    self.right_pill_y_u = target_u;
                }
            }
        }

        // 1. Draw all unfocused frosted row tracks & section headers first
        for lay in &layouts {
            if let Some(sec_title) = lay.header {
                draw_crisp_label(
                    pm,
                    self.font_medium.as_ref(),
                    sec_title,
                    (main_x + 14.0 * unit) as i32,
                    (lay.header_y + slide_y) as i32,
                    (9.8 * unit).round(),
                    with_alpha(Color::from_rgba8(165, 205, 245, 215), a),
                    false,
                );
            }

            let ry = lay.pill_y + slide_y;
            let row_h = lay.pill_h;
            let row_r = row_h * 0.5;
            if let Some(pill) = rounded_rect_path(main_x, ry, main_w, row_h, row_r) {
                let mut fill = Paint::default();
                fill.anti_alias = true;
                fill.set_color(with_alpha(Color::from_rgba8(255, 255, 255, 18), a));
                pm.fill_path(&pill, &fill, FillRule::Winding, Transform::identity(), None);

                let mut rim = Paint::default();
                rim.anti_alias = true;
                rim.set_color(with_alpha(Color::from_rgba8(255, 255, 255, 26), a));
                pm.stroke_path(
                    &pill,
                    &rim,
                    &Stroke {
                        width: 1.0,
                        ..Default::default()
                    },
                    Transform::identity(),
                    None,
                );
            }
        }

        // 2. Draw smooth gliding white active pill in right pane
        if self.in_right_pane && self.right_pill_y_u >= 0.0 {
            let row_h = (38.0 * unit).round();
            let anim_ry = (self.right_pill_y_u * unit + slide_y).round();
            if let Some(hi_pill) = rounded_rect_path(main_x, anim_ry, main_w, row_h, row_h * 0.5) {
                let mut fill = Paint::default();
                fill.anti_alias = true;
                fill.set_color(with_alpha(Color::from_rgba8(245, 247, 250, 250), a));
                pm.fill_path(&hi_pill, &fill, FillRule::Winding, Transform::identity(), None);
            }
        }

        // 3. Draw crisp row labels & values on top of the gliding pill
        for lay in &layouts {
            let item = &rows[lay.abs_idx];
            let ry = lay.pill_y + slide_y;
            let row_h = lay.pill_h;
            let is_focused = self.in_right_pane && lay.abs_idx == self.row_idx;

            let title_col = with_alpha(
                if is_focused {
                    Color::from_rgba8(16, 18, 26, 255)
                } else {
                    Color::from_rgba8(242, 245, 252, 242)
                },
                a,
            );
            let sub_col = with_alpha(
                if is_focused {
                    Color::from_rgba8(68, 76, 94, 235)
                } else {
                    Color::from_rgba8(172, 184, 204, 210)
                },
                a,
            );
            let val_col = with_alpha(
                if is_focused {
                    Color::from_rgba8(24, 28, 38, 245)
                } else {
                    Color::from_rgba8(210, 222, 240, 220)
                },
                a,
            );

            let pad_x = (18.0 * unit).round();
            if let Some(ref sub) = item.subtitle {
                draw_crisp_label(
                    pm,
                    self.font_medium.as_ref(),
                    &item.title,
                    (main_x + pad_x) as i32,
                    (ry + row_h * 0.44) as i32,
                    (12.2 * unit).round(),
                    title_col,
                    false,
                );
                draw_crisp_label(
                    pm,
                    self.font_regular.as_ref(),
                    sub,
                    (main_x + pad_x) as i32,
                    (ry + row_h * 0.80) as i32,
                    (9.8 * unit).round(),
                    sub_col,
                    false,
                );
            } else {
                draw_crisp_label(
                    pm,
                    self.font_medium.as_ref(),
                    &item.title,
                    (main_x + pad_x) as i32,
                    (ry + row_h * 0.64) as i32,
                    (12.5 * unit).round(),
                    title_col,
                    false,
                );
            }

            if item.checked == Some(true) {
                draw_crisp_label(
                    pm,
                    self.font_display.as_ref(),
                    "✓",
                    (main_x + main_w - 28.0 * unit) as i32,
                    (ry + row_h * 0.66) as i32,
                    (13.5 * unit).round(),
                    with_alpha(Color::from_rgba8(46, 204, 113, 255), a),
                    false,
                );
            } else if !item.value.is_empty() {
                let vpx = (11.5 * unit).round();
                let vw = measure_label_width(self.font_medium.as_ref(), &item.value, vpx);
                draw_crisp_label(
                    pm,
                    self.font_medium.as_ref(),
                    &item.value,
                    (main_x + main_w - pad_x - vw) as i32,
                    (ry + row_h * 0.64) as i32,
                    vpx,
                    val_col,
                    false,
                );
            }
        }

        if let Some(ref msg) = self.status_banner {
            let mpx = (10.5 * unit).round();
            let mw = measure_label_width(self.font_medium.as_ref(), msg, mpx);
            let box_w = (mw + 24.0 * unit).min(main_w).round();
            let box_h = (22.0 * unit).round();
            let bx = main_x + 10.0 * unit;
            let by = (sh - 56.0 * unit + slide_y).round();
            if let Some(bp) = rounded_rect_path(bx, by, box_w, box_h, box_h * 0.5) {
                let mut bg = Paint::default();
                bg.anti_alias = true;
                bg.set_color(with_alpha(Color::from_rgba8(22, 28, 42, 232), a));
                pm.fill_path(&bp, &bg, FillRule::Winding, Transform::identity(), None);
            }
            draw_crisp_label(
                pm,
                self.font_medium.as_ref(),
                msg,
                (bx + 12.0 * unit) as i32,
                (by + box_h * 0.68) as i32,
                mpx,
                with_alpha(Color::from_rgba8(143, 211, 255, 250), a),
                false,
            );
        }

        let hint_text = if self.in_right_pane {
            "Up and down to browse, OK to change, Back to go back"
        } else {
            "Up and down to browse, OK to change, Back to close"
        };
        let hpx = (10.2 * unit).round();
        let hw = measure_label_width(self.font_medium.as_ref(), hint_text, hpx);
        let pill_hint_w = (hw + 28.0 * unit).round();
        let pill_hint_h = (23.0 * unit).round();
        let hx = (main_x + 10.0 * unit).round();
        let hy = (sh - 30.0 * unit + slide_y).round();
        if let Some(hp) = rounded_rect_path(hx, hy, pill_hint_w, pill_hint_h, pill_hint_h * 0.5) {
            let mut bg = Paint::default();
            bg.anti_alias = true;
            bg.set_color(with_alpha(Color::from_rgba8(24, 25, 36, 224), a));
            pm.fill_path(&hp, &bg, FillRule::Winding, Transform::identity(), None);

            let mut rim = Paint::default();
            rim.anti_alias = true;
            rim.set_color(with_alpha(Color::from_rgba8(255, 255, 255, 42), a));
            pm.stroke_path(
                &hp,
                &rim,
                &Stroke {
                    width: 1.0,
                    ..Default::default()
                },
                Transform::identity(),
                None,
            );
        }
        draw_crisp_label(
            pm,
            self.font_medium.as_ref(),
            hint_text,
            (hx + 14.0 * unit) as i32,
            (hy + pill_hint_h * 0.67) as i32,
            hpx,
            with_alpha(Color::from_rgba8(225, 230, 240, 215), a),
            false,
        );
    }
}

pub fn rounded_rect_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
    let r = r.min(w * 0.5).min(h * 0.5).max(0.0);
    let k = r * 0.5522848;
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
    pb.line_to(x, y + r);
    pb.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    pb.close();
    pb.finish()
}

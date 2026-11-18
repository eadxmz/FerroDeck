//! Full-Screen Two-Pane Home Settings (`image_4f9a6c.jpg`) + Rounded-Pill Menu Renderer.
//! Features:
//! - Left sidebar (`Home settings`): 11 rounded-pill categories with solid white active pill
//!   (`Appearance and scenes`, `Weather and location`, `Apps`, `People`, `Control Panel`,
//!    `Button mapper`, `Quick settings`, `Language`, `Advanced`, `Updates`, `About`).
//! - Right content pane: Section headers + stadium-pill option rows (`corner_radius = height / 2`)
//!   with smooth 60fps highlight interpolation (fixed so `tick()` always converges cleanly!).
//! - Bottom centered pill hint bar ("Up and down to browse, OK to change, Back to close").

use crate::clock::{draw_crisp_label, measure_label_width};
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

pub struct HomeSettingsScreen {
    pub open: bool,
    pub category_idx: usize,
    pub row_idx: usize,
    pub in_right_pane: bool,
    pub status_banner: Option<String>,
    /// Animated Y coordinate in design units (`540p` space) so resolution changes never glitch.
    pub side_pill_y_u: f32,
    pub row_pill_y_u: f32,
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
            category_idx: 0,
            row_idx: 0,
            in_right_pane: false,
            status_banner: None,
            side_pill_y_u: -1.0,
            row_pill_y_u: -1.0,
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
        self.side_pill_y_u = -1.0;
        self.row_pill_y_u = -1.0;
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
            self.row_pill_y_u = -1.0;
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
            self.row_pill_y_u = -1.0;
        }
    }

    pub fn move_right(&mut self, row_count: usize) {
        if !self.in_right_pane && row_count > 0 {
            self.in_right_pane = true;
            self.row_idx = self.row_idx.min(row_count - 1);
            self.row_pill_y_u = -1.0;
        }
    }

    pub fn move_left(&mut self) {
        if self.in_right_pane {
            self.in_right_pane = false;
        }
    }

    /// Advances smooth pill spring animation in design units; returns `true` while still moving
    /// so `render_window` keeps `needs_redraw = true` until settled.
    pub fn tick(&mut self) -> bool {
        if !self.open {
            return false;
        }
        let target_side_u = 74.0 + (self.category_idx as f32) * 35.0;
        let mut busy = false;

        if self.side_pill_y_u < 0.0 {
            self.side_pill_y_u = target_side_u;
        } else {
            let d = target_side_u - self.side_pill_y_u;
            if d.abs() > 0.15 {
                self.side_pill_y_u += d * 0.42;
                busy = true;
            } else {
                self.side_pill_y_u = target_side_u;
            }
        }

        if self.in_right_pane {
            let target_row_u = self.compute_row_y_u(self.row_idx);
            if self.row_pill_y_u < 0.0 {
                self.row_pill_y_u = target_row_u;
            } else {
                let d = target_row_u - self.row_pill_y_u;
                if d.abs() > 0.15 {
                    self.row_pill_y_u += d * 0.42;
                    busy = true;
                } else {
                    self.row_pill_y_u = target_row_u;
                }
            }
        }

        busy
    }

    fn compute_row_y_u(&self, idx: usize) -> f32 {
        // Uniform row stride in right column (with section headers accounted for)
        96.0 + (idx as f32) * 46.0
    }

    pub fn draw(
        &mut self,
        pm: &mut Pixmap,
        glass_bg: Option<&Pixmap>,
        unit: f32,
        rows: &[SettingRowItem],
    ) {
        if !self.open {
            return;
        }
        let sw = pm.width() as f32;
        let sh = pm.height() as f32;
        let _ = self.tick();

        // 1. Full-screen blurred scene backdrop + dark frosted scrim (matching image_4f9a6c.jpg)
        if let Some(glass) = glass_bg {
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
            if let Some(r) = Rect::from_xywh(0.0, 0.0, sw, sh) {
                pm.fill_rect(r, &gp, Transform::identity(), None);
            }
        }
        let mut scrim = Paint::default();
        scrim.set_color(Color::from_rgba8(10, 11, 18, 206));
        if let Some(r) = Rect::from_xywh(0.0, 0.0, sw, sh) {
            pm.fill_rect(r, &scrim, Transform::identity(), None);
        }

        // 2. Left Sidebar: "Home settings" + 11 rounded-pill categories
        let side_x = (46.0 * unit).round();
        let side_w = (268.0 * unit).round();
        draw_crisp_label(
            pm,
            self.font_display.as_ref(),
            "Home settings",
            (side_x + 14.0 * unit) as i32,
            (54.0 * unit) as i32,
            (22.0 * unit).round(),
            Color::from_rgba8(255, 255, 255, 250),
            false,
        );

        let pill_h = (31.0 * unit).round();
        let pill_r = pill_h * 0.5;
        let anim_side_y = (self.side_pill_y_u * unit).round();

        // Animated active sidebar stadium pill (solid white when left pane focused, soft glass when right pane focused)
        if let Some(sp) = rounded_rect_path(side_x, anim_side_y, side_w, pill_h, pill_r) {
            let mut fill = Paint::default();
            fill.anti_alias = true;
            if !self.in_right_pane {
                fill.set_color(Color::from_rgba8(245, 247, 250, 248));
            } else {
                fill.set_color(Color::from_rgba8(255, 255, 255, 34));
            }
            pm.fill_path(&sp, &fill, FillRule::Winding, Transform::identity(), None);
        }

        for (i, cat) in SettingsCategory::ALL.iter().enumerate() {
            let ry = ((74.0 + (i as f32) * 35.0) * unit).round();
            let is_active = i == self.category_idx;
            let text_col = if is_active && !self.in_right_pane {
                Color::from_rgba8(16, 18, 26, 255)
            } else if is_active {
                Color::from_rgba8(255, 255, 255, 255)
            } else {
                Color::from_rgba8(220, 226, 238, 215)
            };

            // Minimal crisp badge dot/icon on left of sidebar item
            let dot_x = side_x + 14.0 * unit;
            let dot_y = ry + pill_h * 0.5 - 4.5 * unit;
            if let Some(badge) = rounded_rect_path(dot_x, dot_y, 9.0 * unit, 9.0 * unit, 3.0 * unit) {
                let mut bp = Paint::default();
                bp.anti_alias = true;
                bp.set_color(if is_active && !self.in_right_pane {
                    Color::from_rgba8(32, 36, 48, 220)
                } else {
                    Color::from_rgba8(210, 220, 235, 175)
                });
                pm.stroke_path(
                    &badge,
                    &bp,
                    &Stroke {
                        width: (1.1 * unit).max(1.0),
                        ..Default::default()
                    },
                    Transform::identity(),
                    None,
                );
            }

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

        // 3. Right Content Column: Section Header + Stadium-Pill Option Rows
        let main_x = (348.0 * unit).round();
        let main_w = (sw - main_x - 52.0 * unit).round();
        let active_cat = self.current_category();

        draw_crisp_label(
            pm,
            self.font_display.as_ref(),
            active_cat.title(),
            (main_x + 12.0 * unit) as i32,
            (54.0 * unit) as i32,
            (19.0 * unit).round(),
            Color::from_rgba8(255, 255, 255, 248),
            false,
        );

        if let Some(first) = rows.first() {
            if !first.section.is_empty() {
                draw_crisp_label(
                    pm,
                    self.font_medium.as_ref(),
                    first.section,
                    (main_x + 14.0 * unit) as i32,
                    (82.0 * unit) as i32,
                    (9.8 * unit).round(),
                    Color::from_rgba8(175, 186, 206, 195),
                    false,
                );
            }
        }

        // Visible window of up to 8 rows so Advanced / Appearance never overflow vertically
        let max_visible = 8usize;
        let page_start = if self.row_idx >= max_visible {
            self.row_idx + 1 - max_visible
        } else {
            0
        };
        let visible_rows = &rows[page_start..(page_start + max_visible).min(rows.len())];

        let row_h = (39.0 * unit).round();
        let row_stride = (46.0 * unit).round();
        let row_r = row_h * 0.5; // True stadium pill radius!
        let list_top = (94.0 * unit).round();

        for (vis_i, item) in visible_rows.iter().enumerate() {
            let abs_i = page_start + vis_i;
            let ry = list_top + (vis_i as f32) * row_stride;
            let is_focused = self.in_right_pane && abs_i == self.row_idx;

            if let Some(pill) = rounded_rect_path(main_x, ry, main_w, row_h, row_r) {
                let mut fill = Paint::default();
                fill.anti_alias = true;
                if is_focused {
                    fill.set_color(Color::from_rgba8(245, 247, 250, 248));
                } else {
                    fill.set_color(Color::from_rgba8(255, 255, 255, 18));
                }
                pm.fill_path(&pill, &fill, FillRule::Winding, Transform::identity(), None);

                if !is_focused {
                    let mut rim = Paint::default();
                    rim.anti_alias = true;
                    rim.set_color(Color::from_rgba8(255, 255, 255, 26));
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

            let title_col = if is_focused {
                Color::from_rgba8(16, 18, 26, 255)
            } else {
                Color::from_rgba8(242, 245, 252, 242)
            };
            let sub_col = if is_focused {
                Color::from_rgba8(68, 76, 94, 235)
            } else {
                Color::from_rgba8(172, 184, 204, 210)
            };
            let val_col = if is_focused {
                Color::from_rgba8(24, 28, 38, 245)
            } else {
                Color::from_rgba8(210, 222, 240, 220)
            };

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

            // Right-aligned value or green checkmark
            if item.checked == Some(true) {
                draw_crisp_label(
                    pm,
                    self.font_display.as_ref(),
                    "✓",
                    (main_x + main_w - 28.0 * unit) as i32,
                    (ry + row_h * 0.66) as i32,
                    (13.5 * unit).round(),
                    Color::from_rgba8(46, 204, 113, 255),
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

        // 4. Optional status banner above hint bar
        if let Some(ref msg) = self.status_banner {
            let mpx = (10.5 * unit).round();
            let mw = measure_label_width(self.font_medium.as_ref(), msg, mpx);
            let box_w = (mw + 24.0 * unit).min(main_w).round();
            let box_h = (22.0 * unit).round();
            let bx = main_x + 10.0 * unit;
            let by = (sh - 56.0 * unit).round();
            if let Some(bp) = rounded_rect_path(bx, by, box_w, box_h, box_h * 0.5) {
                let mut bg = Paint::default();
                bg.anti_alias = true;
                bg.set_color(Color::from_rgba8(22, 28, 42, 232));
                pm.fill_path(&bp, &bg, FillRule::Winding, Transform::identity(), None);
            }
            draw_crisp_label(
                pm,
                self.font_medium.as_ref(),
                msg,
                (bx + 12.0 * unit) as i32,
                (by + box_h * 0.68) as i32,
                mpx,
                Color::from_rgba8(143, 211, 255, 250),
                false,
            );
        }

        // 5. Bottom Centered Hint Pill (matching image_4f9a6c.jpg)
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
        let hy = (sh - 30.0 * unit).round();
        if let Some(hp) = rounded_rect_path(hx, hy, pill_hint_w, pill_hint_h, pill_hint_h * 0.5) {
            let mut bg = Paint::default();
            bg.anti_alias = true;
            bg.set_color(Color::from_rgba8(24, 25, 36, 224));
            pm.fill_path(&hp, &bg, FillRule::Winding, Transform::identity(), None);

            let mut rim = Paint::default();
            rim.anti_alias = true;
            rim.set_color(Color::from_rgba8(255, 255, 255, 42));
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
            Color::from_rgba8(225, 230, 240, 215),
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

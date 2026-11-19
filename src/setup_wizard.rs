//! Native Rust 6-Step Liquid Glass First-Time Setup Wizard (`tiny-skia` + `fontdue`).
//! Matches the tvOS-inspired Setup experience:
//!   Step 0: Welcome & Language (Native India [English default] vs Global)
//!   Step 1: Create Your Profile (Custom Name dialog + Avatar color badge)
//!   Step 2: Weather & Location (Automatic IP/Network Location vs Manual City entry vs Off)
//!   Step 3: Appearance (Night scenes vs Classic)
//!   Step 4: Button Mapper & Voice Assistant Keybind
//!   Step 5: Finish Setting Up (Easy Setup via ADB / Manual Permissions / Finish)

use crate::anim::{ease_out_cubic, smooth_to, with_alpha};
use crate::clock::{draw_crisp_label, measure_label_width};
use crate::settings_pill::rounded_rect_path;
use crate::ui::ALL_LANGUAGES;
use fontdue::{Font, FontSettings};
use tiny_skia::{
    Color, FillRule, FilterQuality, Paint, Pattern, Pixmap, Rect, SpreadMode, Stroke, Transform,
};

pub const SETUP_TOTAL_STEPS: usize = 6;

pub const AVATAR_COLORS: &[[u8; 3]] = &[
    [52, 132, 244],  // Ocean Blue
    [235, 87, 87],   // Coral Red
    [46, 204, 113],  // Emerald
    [155, 81, 224],  // Violet
    [242, 153, 74],  // Amber
    [45, 180, 190],  // Teal
];

#[derive(Clone, Debug)]
pub struct SetupOption {
    pub title: String,
    pub subtitle: Option<String>,
    pub checked: bool,
    pub action_id: &'static str,
}

pub struct SetupWizard {
    pub open: bool,
    pub step: usize,
    pub selected: usize,
    pub lang_region_global: bool,
    pub avatar_idx: usize,
    pub step_anim: f32,
    pub step_dir: f32,
    pub pill_y_u: f32,
    font_display: Option<Font>,
    font_medium: Option<Font>,
    font_regular: Option<Font>,
}

impl SetupWizard {
    pub fn new(
        bold_bytes: Option<&[u8]>,
        med_bytes: Option<&[u8]>,
        reg_bytes: Option<&[u8]>,
    ) -> Self {
        Self {
            open: false,
            step: 0,
            selected: 0,
            lang_region_global: false,
            avatar_idx: 0,
            step_anim: 1.0,
            step_dir: 1.0,
            pill_y_u: -1.0,
            font_display: bold_bytes.and_then(|b| {
                Font::from_bytes(
                    b,
                    FontSettings {
                        scale: 52.0,
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

    pub fn open_fresh(&mut self) {
        self.open = true;
        self.step = 0;
        self.selected = 0;
        self.step_anim = 0.0;
        self.step_dir = 1.0;
        self.pill_y_u = -1.0;
    }

    pub fn open_at_step(&mut self, step: usize) {
        self.open = true;
        self.step = step.min(SETUP_TOTAL_STEPS - 1);
        self.selected = 0;
        self.step_anim = 0.0;
        self.step_dir = 1.0;
        self.pill_y_u = -1.0;
    }

    pub fn tick(&mut self, dt: f32, animations_enabled: bool) -> bool {
        if !self.open {
            return false;
        }
        let list_top_u = if self.step == 1 { 192.0 } else { 146.0 };
        let max_vis = 6usize;
        let start_idx = if self.selected >= max_vis {
            self.selected + 1 - max_vis
        } else {
            0
        };
        let vis_sel = self.selected.saturating_sub(start_idx);
        let target_y_u = list_top_u + (vis_sel as f32) * 51.0;
        if !animations_enabled {
            self.step_anim = 1.0;
            self.pill_y_u = target_y_u;
            return false;
        }
        let mut busy = smooth_to(&mut self.step_anim, 1.0, 24.0, dt, 0.018);
        if self.pill_y_u < 0.0 {
            self.pill_y_u = target_y_u;
        } else if smooth_to(&mut self.pill_y_u, target_y_u, 32.0, dt, 0.7) {
            busy = true;
        }
        busy
    }

    pub fn step_header(&self) -> (&'static str, &'static str) {
        match self.step {
            0 => ("Welcome", "Let's set up FerroDeck. Choose Native (India) or Global language."),
            1 => ("Create Your Profile", "Set your name for the Home greeting and choose an avatar."),
            2 => ("Weather & Location", "Show live weather on Home — choose Automatic or pick a city manually."),
            3 => ("Appearance", "Choose how you want your Home Screen to look."),
            4 => ("Button Mapper & Voice", "Choose your remote button for the inbuilt Voice Assistant."),
            _ => ("Finish Setting Up", "Set up Default Home, Accessibility Key Filter, and Microphone."),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn build_options(
        &self,
        lang_code: &str,
        profile_name: &str,
        weather_auto: bool,
        weather_enabled: bool,
        weather_city: &str,
        classic_look: bool,
        voice_key_label: &str,
        cc_key_label: &str,
        default_home: bool,
        a11y_enabled: bool,
    ) -> Vec<SetupOption> {
        match self.step {
            0 => {
                let mut opts = vec![
                    SetupOption {
                        title: "Get Started".into(),
                        subtitle: None,
                        checked: false,
                        action_id: "setup_next",
                    },
                    SetupOption {
                        title: if self.lang_region_global {
                            "Language Region: Global  (Press OK for Native India)".into()
                        } else {
                            "Language Region: Native (India)  (Press OK for Global)".into()
                        },
                        subtitle: Some("English is kept under Native (India) alongside Indian languages".into()),
                        checked: false,
                        action_id: "setup_toggle_lang_region",
                    },
                ];
                for lang in ALL_LANGUAGES
                    .iter()
                    .filter(|l| l.is_native_india != self.lang_region_global)
                    .take(5)
                {
                    opts.push(SetupOption {
                        title: lang.title.into(),
                        subtitle: Some(lang.subtitle.into()),
                        checked: lang_code == lang.code,
                        action_id: lang.action_id,
                    });
                }
                opts
            }
            1 => vec![
                SetupOption {
                    title: "Profile Name".into(),
                    subtitle: Some(format!("{}  (Press OK to edit name)", profile_name)),
                    checked: false,
                    action_id: "setup_edit_name",
                },
                SetupOption {
                    title: "Avatar Color".into(),
                    subtitle: Some("Press OK or Left/Right to change avatar badge color".into()),
                    checked: false,
                    action_id: "setup_cycle_avatar",
                },
                SetupOption {
                    title: "Continue".into(),
                    subtitle: None,
                    checked: false,
                    action_id: "setup_next",
                },
            ],
            2 => vec![
                SetupOption {
                    title: "Use My Location (Automatic)".into(),
                    subtitle: Some(if weather_enabled && weather_auto {
                        format!("Active · {}", if weather_city.is_empty() { "Detecting city..." } else { weather_city })
                    } else {
                        "Auto-detect your city and live temperature".into()
                    }),
                    checked: weather_enabled && weather_auto,
                    action_id: "setup_weather_auto",
                },
                SetupOption {
                    title: "Choose a City Manually...".into(),
                    subtitle: Some(if weather_enabled && !weather_auto && !weather_city.is_empty() {
                        format!("Selected city: {}", weather_city)
                    } else {
                        "Type any city name (e.g. Srinagar, Mumbai, Delhi)".into()
                    }),
                    checked: weather_enabled && !weather_auto,
                    action_id: "setup_weather_manual",
                },
                SetupOption {
                    title: "Don't Show Weather".into(),
                    subtitle: Some("Hide weather display from the Home Screen".into()),
                    checked: !weather_enabled,
                    action_id: "setup_weather_off",
                },
                SetupOption {
                    title: "Next".into(),
                    subtitle: None,
                    checked: false,
                    action_id: "setup_next",
                },
            ],
            3 => vec![
                SetupOption {
                    title: "Night scenes".into(),
                    subtitle: Some("Rotating nature scenes with a crisp 1:1 glass clock and bottom dock".into()),
                    checked: !classic_look,
                    action_id: "setup_look_scenes",
                },
                SetupOption {
                    title: "Classic".into(),
                    subtitle: Some("Streamlined Home Screen with top-right crisp clock".into()),
                    checked: classic_look,
                    action_id: "setup_look_classic",
                },
                SetupOption {
                    title: "Continue".into(),
                    subtitle: None,
                    checked: false,
                    action_id: "setup_next",
                },
            ],
            4 => vec![
                SetupOption {
                    title: "Set Voice Assistant Button...".into(),
                    subtitle: Some(format!("Current: {}  (Press OK, then press remote key)", voice_key_label)),
                    checked: false,
                    action_id: "setup_capture_voice",
                },
                SetupOption {
                    title: "Bind Control Center Button...".into(),
                    subtitle: Some(format!("Current: {}  (Press OK, then press remote key)", cc_key_label)),
                    checked: false,
                    action_id: "setup_capture_cc",
                },
                SetupOption {
                    title: "Continue".into(),
                    subtitle: None,
                    checked: false,
                    action_id: "setup_next",
                },
            ],
            _ => vec![
                SetupOption {
                    title: "Set as Default Home Screen".into(),
                    subtitle: Some(if default_home {
                        "FerroDeck is already your default launcher".into()
                    } else {
                        "Open Android Home picker to choose FerroDeck".into()
                    }),
                    checked: default_home,
                    action_id: "setup_default_home",
                },
                SetupOption {
                    title: "Enable Accessibility Key Filter & Voice".into(),
                    subtitle: Some(if a11y_enabled {
                        "Active — Voice Assistant works over all apps".into()
                    } else {
                        "Required for yellow mic & in-app voice search over YouTube/Browser".into()
                    }),
                    checked: a11y_enabled,
                    action_id: "setup_a11y",
                },
                SetupOption {
                    title: "Easy Setup (Automatic via ADB)".into(),
                    subtitle: Some("Check Local ADB 127.0.0.1:5555 for automatic permission grant".into()),
                    checked: false,
                    action_id: "setup_adb_easy",
                },
                SetupOption {
                    title: "Finish & Go to Home".into(),
                    subtitle: None,
                    checked: false,
                    action_id: "setup_finish",
                },
            ],
        }
    }

    pub fn move_up(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        self.selected = if self.selected == 0 {
            count - 1
        } else {
            self.selected - 1
        };
    }

    pub fn move_down(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        self.selected = (self.selected + 1) % count;
    }

    pub fn next_step(&mut self) {
        if self.step + 1 < SETUP_TOTAL_STEPS {
            self.step += 1;
            self.selected = 0;
            self.step_anim = 0.0;
            self.step_dir = 1.0;
            self.pill_y_u = -1.0;
        } else {
            self.open = false;
        }
    }

    pub fn prev_step(&mut self) {
        if self.step > 0 {
            self.step -= 1;
            self.selected = 0;
            self.step_anim = 0.0;
            self.step_dir = -1.0;
            self.pill_y_u = -1.0;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        pm: &mut Pixmap,
        wallpaper: Option<&Pixmap>,
        glass_bg: Option<&Pixmap>,
        unit: f32,
        options: &[SetupOption],
        profile_name: &str,
        status_banner: Option<&str>,
    ) {
        if !self.open {
            return;
        }
        let sw = pm.width() as f32;
        let sh = pm.height() as f32;

        // 1. Background: deep black on Welcome/Profile (like image_4f966d.png), scenic backdrop on steps 2..5
        if self.step <= 1 {
            pm.fill(Color::from_rgba8(6, 7, 11, 255));
        } else if let Some(wp) = wallpaper {
            let sx = sw / wp.width() as f32;
            let sy = sh / wp.height() as f32;
            let scale = sx.max(sy);
            let tx = (sw - wp.width() as f32 * scale) * 0.5;
            let ty = (sh - wp.height() as f32 * scale) * 0.5;
            let mut p = Paint::default();
            p.shader = Pattern::new(
                wp.as_ref(),
                SpreadMode::Pad,
                FilterQuality::Bilinear,
                1.0,
                Transform::from_scale(scale, scale).post_translate(tx, ty),
            );
            if let Some(r) = Rect::from_xywh(0.0, 0.0, sw, sh) {
                pm.fill_rect(r, &p, Transform::identity(), None);
            }
            let mut scrim = Paint::default();
            scrim.set_color(Color::from_rgba8(6, 8, 14, 128));
            if let Some(r) = Rect::from_xywh(0.0, 0.0, sw, sh) {
                pm.fill_rect(r, &scrim, Transform::identity(), None);
            }
        } else {
            pm.fill(Color::from_rgba8(10, 12, 20, 255));
        }

        // 2. Top-right "Skip Setup >" pill (matching image_4f966d.png)
        let skip_lbl = "Skip Setup >";
        let spx = (11.5 * unit).round();
        let sw_lbl = measure_label_width(self.font_medium.as_ref(), skip_lbl, spx);
        let skip_w = (sw_lbl + 26.0 * unit).round();
        let skip_h = (28.0 * unit).round();
        let skip_x = (sw - 34.0 * unit - skip_w).round();
        let skip_y = (24.0 * unit).round();
        if let Some(sp) = rounded_rect_path(skip_x, skip_y, skip_w, skip_h, skip_h * 0.5) {
            let mut fill = Paint::default();
            fill.anti_alias = true;
            fill.set_color(Color::from_rgba8(32, 35, 44, 210));
            pm.fill_path(&sp, &fill, FillRule::Winding, Transform::identity(), None);
            let mut rim = Paint::default();
            rim.anti_alias = true;
            rim.set_color(Color::from_rgba8(255, 255, 255, 76));
            pm.stroke_path(
                &sp,
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
            skip_lbl,
            (skip_x + 13.0 * unit) as i32,
            (skip_y + skip_h * 0.66) as i32,
            spx,
            Color::from_rgba8(245, 247, 252, 238),
            false,
        );

        // 3. Centered Title & Subtitle
        let (title, sub) = self.step_header();
        let title_px = (32.0 * unit).round();
        let tw = measure_label_width(self.font_display.as_ref(), title, title_px);
        draw_crisp_label(
            pm,
            self.font_display.as_ref(),
            title,
            ((sw - tw) * 0.5).round() as i32,
            (84.0 * unit) as i32,
            title_px,
            Color::from_rgba8(255, 255, 255, 252),
            true,
        );

        let sub_px = (12.8 * unit).round();
        let sub_w = measure_label_width(self.font_regular.as_ref(), sub, sub_px);
        draw_crisp_label(
            pm,
            self.font_regular.as_ref(),
            sub,
            ((sw - sub_w) * 0.5).round() as i32,
            (116.0 * unit) as i32,
            sub_px,
            Color::from_rgba8(205, 212, 228, 220),
            true,
        );

        // 4. Optional Avatar Badge row on Step 1 (Profile)
        let mut list_top = (146.0 * unit).round();
        if self.step == 1 {
            let av_size = (36.0 * unit).round();
            let av_gap = (12.0 * unit).round();
            let total_w = (AVATAR_COLORS.len() as f32) * av_size
                + ((AVATAR_COLORS.len() - 1) as f32) * av_gap;
            let start_x = ((sw - total_w) * 0.5).round();
            let av_y = (138.0 * unit).round();
            let initial = profile_name
                .chars()
                .next()
                .unwrap_or('F')
                .to_uppercase()
                .to_string();
            for (idx, rgb) in AVATAR_COLORS.iter().enumerate() {
                let ax = start_x + (idx as f32) * (av_size + av_gap);
                let is_sel = idx == (self.avatar_idx % AVATAR_COLORS.len());
                if let Some(c) = rounded_rect_path(ax, av_y, av_size, av_size, av_size * 0.5) {
                    let mut fill = Paint::default();
                    fill.anti_alias = true;
                    fill.set_color(Color::from_rgba8(rgb[0], rgb[1], rgb[2], 240));
                    pm.fill_path(&c, &fill, FillRule::Winding, Transform::identity(), None);
                    if is_sel {
                        let mut rim = Paint::default();
                        rim.anti_alias = true;
                        rim.set_color(Color::from_rgba8(255, 255, 255, 255));
                        pm.stroke_path(
                            &c,
                            &rim,
                            &Stroke {
                                width: (2.2 * unit).max(1.8),
                                ..Default::default()
                            },
                            Transform::identity(),
                            None,
                        );
                    }
                }
                let ipx = (15.0 * unit).round();
                let iw = measure_label_width(self.font_display.as_ref(), &initial, ipx);
                draw_crisp_label(
                    pm,
                    self.font_display.as_ref(),
                    &initial,
                    (ax + (av_size - iw) * 0.5).round() as i32,
                    (av_y + av_size * 0.67).round() as i32,
                    ipx,
                    Color::from_rgba8(255, 255, 255, 250),
                    false,
                );
            }
            list_top = (192.0 * unit).round();
        }

        // 5. Centered Frosted Pill Option Stack with smooth step slide & gliding white focus pill
        let eased_step = ease_out_cubic(self.step_anim);
        let slide_x = ((1.0 - eased_step) * 34.0 * unit * self.step_dir).round();
        let step_alpha = (0.25 + 0.75 * eased_step).clamp(0.0, 1.0);

        let card_w = (460.0 * unit).round();
        let card_x = ((sw - card_w) * 0.5 + slide_x).round();
        let row_h = (42.0 * unit).round();
        let row_gap = (9.0 * unit).round();
        let row_r = (13.0 * unit).round();

        let max_vis = 6usize;
        let start_idx = if self.selected >= max_vis {
            self.selected + 1 - max_vis
        } else {
            0
        };
        let visible = &options[start_idx..(start_idx + max_vis).min(options.len())];

        // 5a. Draw dark frosted row tracks first
        for vis_i in 0..visible.len() {
            let ry = list_top + (vis_i as f32) * (row_h + row_gap);
            if let Some(pill) = rounded_rect_path(card_x, ry, card_w, row_h, row_r) {
                if self.step >= 2 {
                    if let Some(glass) = glass_bg {
                        let sx = sw / glass.width() as f32;
                        let sy = sh / glass.height() as f32;
                        let mut gp = Paint::default();
                        gp.anti_alias = true;
                        gp.shader = Pattern::new(
                            glass.as_ref(),
                            SpreadMode::Pad,
                            FilterQuality::Bilinear,
                            0.85 * step_alpha,
                            Transform::from_scale(sx, sy),
                        );
                        pm.fill_path(&pill, &gp, FillRule::Winding, Transform::identity(), None);
                    }
                }
                let mut fill = Paint::default();
                fill.anti_alias = true;
                fill.set_color(with_alpha(Color::from_rgba8(30, 33, 42, 195), step_alpha));
                pm.fill_path(&pill, &fill, FillRule::Winding, Transform::identity(), None);

                let mut rim = Paint::default();
                rim.anti_alias = true;
                rim.set_color(with_alpha(Color::from_rgba8(255, 255, 255, 38), step_alpha));
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

        // 5b. Draw smooth gliding white active pill
        let fallback_y = list_top + (self.selected.saturating_sub(start_idx) as f32) * (row_h + row_gap);
        let hi_y = if self.pill_y_u >= 0.0 {
            (self.pill_y_u * unit).round()
        } else {
            fallback_y
        };
        if let Some(hi_pill) = rounded_rect_path(card_x, hi_y, card_w, row_h, row_r) {
            let mut fill = Paint::default();
            fill.anti_alias = true;
            fill.set_color(with_alpha(Color::from_rgba8(245, 247, 250, 252), step_alpha));
            pm.fill_path(&hi_pill, &fill, FillRule::Winding, Transform::identity(), None);
        }

        // 5c. Draw crisp labels & checkmarks
        for (vis_i, opt) in visible.iter().enumerate() {
            let abs_i = start_idx + vis_i;
            let ry = list_top + (vis_i as f32) * (row_h + row_gap);
            let focused = abs_i == self.selected;

            let tcol = if focused {
                Color::from_rgba8(14, 16, 22, 255)
            } else {
                Color::from_rgba8(246, 248, 252, 245)
            };
            let scol = if focused {
                Color::from_rgba8(72, 80, 96, 235)
            } else {
                Color::from_rgba8(180, 190, 208, 215)
            };

            let pad_x = (18.0 * unit).round();
            if let Some(ref sub_txt) = opt.subtitle {
                draw_crisp_label(
                    pm,
                    self.font_medium.as_ref(),
                    &opt.title,
                    (card_x + pad_x) as i32,
                    (ry + row_h * 0.45) as i32,
                    (12.5 * unit).round(),
                    tcol,
                    false,
                );
                draw_crisp_label(
                    pm,
                    self.font_regular.as_ref(),
                    sub_txt,
                    (card_x + pad_x) as i32,
                    (ry + row_h * 0.80) as i32,
                    (9.8 * unit).round(),
                    scol,
                    false,
                );
            } else {
                draw_crisp_label(
                    pm,
                    self.font_medium.as_ref(),
                    &opt.title,
                    (card_x + pad_x) as i32,
                    (ry + row_h * 0.63) as i32,
                    (12.8 * unit).round(),
                    tcol,
                    false,
                );
            }

            if opt.checked {
                draw_crisp_label(
                    pm,
                    self.font_display.as_ref(),
                    "✓",
                    (card_x + card_w - 28.0 * unit) as i32,
                    (ry + row_h * 0.65) as i32,
                    (13.5 * unit).round(),
                    Color::from_rgba8(46, 204, 113, 255),
                    false,
                );
            }
        }

        // 5b. Live status pill above pagination dots (e.g. Easy ADB feedback)
        if let Some(msg) = status_banner {
            if !msg.is_empty() {
                let mpx = (11.0 * unit).round();
                let mw = measure_label_width(self.font_medium.as_ref(), msg, mpx);
                let bw = (mw + 28.0 * unit).min(sw - 64.0 * unit).round();
                let bh = (24.0 * unit).round();
                let bx = ((sw - bw) * 0.5).round();
                let by = (sh - 60.0 * unit).round();
                if let Some(bp) = rounded_rect_path(bx, by, bw, bh, bh * 0.5) {
                    let mut bg = Paint::default();
                    bg.anti_alias = true;
                    bg.set_color(Color::from_rgba8(20, 26, 40, 236));
                    pm.fill_path(&bp, &bg, FillRule::Winding, Transform::identity(), None);
                    let mut rim = Paint::default();
                    rim.anti_alias = true;
                    rim.set_color(Color::from_rgba8(120, 195, 255, 130));
                    pm.stroke_path(
                        &bp,
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
                    msg,
                    (bx + 14.0 * unit) as i32,
                    (by + bh * 0.68) as i32,
                    mpx,
                    Color::from_rgba8(152, 218, 255, 250),
                    false,
                );
            }
        }

        // 6. Pagination Dots at Bottom (matching image_4f96a4.png)
        let dot_r = (3.2 * unit).max(2.5);
        let dot_gap = (12.0 * unit).round();
        let dots_w = (SETUP_TOTAL_STEPS as f32) * (dot_r * 2.0)
            + ((SETUP_TOTAL_STEPS - 1) as f32) * dot_gap;
        let dot_start_x = ((sw - dots_w) * 0.5).round();
        let dot_y = (sh - 26.0 * unit).round();
        for d in 0..SETUP_TOTAL_STEPS {
            let dx = dot_start_x + (d as f32) * (dot_r * 2.0 + dot_gap);
            if let Some(dp) = rounded_rect_path(dx, dot_y, dot_r * 2.0, dot_r * 2.0, dot_r) {
                let mut p = Paint::default();
                p.anti_alias = true;
                p.set_color(if d == self.step {
                    Color::from_rgba8(255, 255, 255, 248)
                } else {
                    Color::from_rgba8(255, 255, 255, 85)
                });
                pm.fill_path(&dp, &p, FillRule::Winding, Transform::identity(), None);
            }
        }
    }
}

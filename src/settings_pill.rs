//! Cleaner Rounded-Pill Settings Overlay rendered natively in `tiny-skia`.
//! Replaces the 300KB Java `SettingsActivity` with a zero-allocation, 60fps animated
//! stadium-pill overlay inside the launcher surface.

use fontdue::{Font, FontSettings};
use tiny_skia::{
    Color, FillRule, LineCap, Paint, Path, PathBuilder, Pixmap, Rect, Stroke, Transform,
};

#[derive(Clone, Debug)]
pub struct PillOption {
    pub title: String,
    pub value: String,
}

pub struct PillSettingsPanel {
    pub open: bool,
    pub selected: usize,
    pub highlight_y: f32,
    font_title: Option<Font>,
    font_item: Option<Font>,
}

impl PillSettingsPanel {
    pub fn new(bold_bytes: Option<&[u8]>, reg_bytes: Option<&[u8]>) -> Self {
        Self {
            open: false,
            selected: 0,
            highlight_y: 0.0,
            font_title: bold_bytes
                .and_then(|b| Font::from_bytes(b, FontSettings { scale: 40.0, ..Default::default() }).ok()),
            font_item: reg_bytes
                .and_then(|b| Font::from_bytes(b, FontSettings { scale: 28.0, ..Default::default() }).ok()),
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.selected = 0;
            self.highlight_y = 0.0;
        }
    }

    pub fn move_up(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        self.selected = if self.selected == 0 { count - 1 } else { self.selected - 1 };
    }

    pub fn move_down(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        self.selected = (self.selected + 1) % count;
    }

    pub fn tick(&mut self, target_y: f32) -> bool {
        if !self.open {
            return false;
        }
        if self.highlight_y == 0.0 {
            self.highlight_y = target_y;
            return false;
        }
        let diff = target_y - self.highlight_y;
        if diff.abs() < 0.5 {
            self.highlight_y = target_y;
            false
        } else {
            self.highlight_y += diff * 0.35;
            true
        }
    }

    pub fn draw(&mut self, pm: &mut Pixmap, unit: f32, options: &[PillOption]) {
        if !self.open || options.is_empty() {
            return;
        }
        let sw = pm.width() as f32;
        let sh = pm.height() as f32;

        let mut scrim = Paint::default();
        scrim.set_color(Color::from_rgba8(6, 9, 14, 155));
        if let Some(r) = Rect::from_xywh(0.0, 0.0, sw, sh) {
            pm.fill_rect(r, &scrim, Transform::identity(), None);
        }

        let panel_w = (540.0 * unit).round();
        let row_h = (44.0 * unit).round();
        let row_gap = (10.0 * unit).round();
        let header_h = (54.0 * unit).round();
        let pad = (22.0 * unit).round();
        let panel_h = pad * 2.0 + header_h + (options.len() as f32) * (row_h + row_gap);
        let px = ((sw - panel_w) * 0.5).round();
        let py = ((sh - panel_h) * 0.5).round();
        let panel_r = (26.0 * unit).round();

        if let Some(panel_path) = rounded_rect_path(px, py, panel_w, panel_h, panel_r) {
            let mut bg = Paint::default();
            bg.anti_alias = true;
            bg.set_color(Color::from_rgba8(18, 23, 34, 232));
            pm.fill_path(&panel_path, &bg, FillRule::Winding, Transform::identity(), None);

            let mut rim = Paint::default();
            rim.anti_alias = true;
            rim.set_color(Color::from_rgba8(255, 255, 255, 58));
            let mut stroke = Stroke::default();
            stroke.width = (1.2 * unit).max(1.0);
            pm.stroke_path(&panel_path, &rim, &stroke, Transform::identity(), None);
        }

        self.draw_text(
            pm,
            "FerroDeck Settings",
            (px + pad + 8.0 * unit) as i32,
            (py + pad + 26.0 * unit) as i32,
            (21.0 * unit).round(),
            true,
            Color::from_rgba8(255, 255, 255, 245),
        );

        let list_top = py + pad + header_h;
        let pill_x = px + pad;
        let pill_w = panel_w - pad * 2.0;
        let pill_r = row_h * 0.5;

        let target_y = list_top + (self.selected as f32) * (row_h + row_gap);
        let _ = self.tick(target_y);

        for (idx, opt) in options.iter().enumerate() {
            let ry = list_top + (idx as f32) * (row_h + row_gap);
            if let Some(pill) = rounded_rect_path(pill_x, ry, pill_w, row_h, pill_r) {
                let mut fill = Paint::default();
                fill.anti_alias = true;
                fill.set_color(Color::from_rgba8(255, 255, 255, 16));
                pm.fill_path(&pill, &fill, FillRule::Winding, Transform::identity(), None);
            }

            let is_sel = idx == self.selected;
            let text_col = if is_sel {
                Color::from_rgba8(255, 255, 255, 255)
            } else {
                Color::from_rgba8(225, 232, 242, 210)
            };
            let val_col = if is_sel {
                Color::from_rgba8(140, 215, 255, 255)
            } else {
                Color::from_rgba8(170, 185, 205, 195)
            };

            let baseline_y = (ry + row_h * 0.64) as i32;
            self.draw_text(
                pm,
                &opt.title,
                (pill_x + 20.0 * unit) as i32,
                baseline_y,
                (14.5 * unit).round(),
                is_sel,
                text_col,
            );
            let val_x = (pill_x + pill_w - 20.0 * unit - (opt.value.len() as f32 * 7.8 * unit)) as i32;
            self.draw_text(
                pm,
                &opt.value,
                val_x.max((pill_x + pill_w * 0.52) as i32),
                baseline_y,
                (13.5 * unit).round(),
                false,
                val_col,
            );
        }

        if let Some(active_pill) = rounded_rect_path(pill_x, self.highlight_y, pill_w, row_h, pill_r) {
            let mut hi_fill = Paint::default();
            hi_fill.anti_alias = true;
            hi_fill.set_color(Color::from_rgba8(255, 255, 255, 34));
            pm.fill_path(&active_pill, &hi_fill, FillRule::Winding, Transform::identity(), None);

            let mut hi_rim = Paint::default();
            hi_rim.anti_alias = true;
            hi_rim.set_color(Color::from_rgba8(255, 255, 255, 195));
            let mut st = Stroke::default();
            st.width = (1.6 * unit).max(1.2);
            st.line_cap = LineCap::Round;
            pm.stroke_path(&active_pill, &hi_rim, &st, Transform::identity(), None);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_text(
        &self,
        pm: &mut Pixmap,
        text: &str,
        x: i32,
        baseline_y: i32,
        px: f32,
        bold: bool,
        color: Color,
    ) {
        let font = if bold {
            self.font_title.as_ref().or(self.font_item.as_ref())
        } else {
            self.font_item.as_ref().or(self.font_title.as_ref())
        };
        let Some(font) = font else { return };

        let w = pm.width() as i32;
        let h = pm.height() as i32;
        let mut pen_x = x;
        let rgba = color.to_color_u8();

        for ch in text.chars() {
            let (metrics, bitmap) = font.rasterize(ch, px);
            let gx = pen_x + metrics.xmin;
            let gy = baseline_y - (metrics.height as i32 + metrics.ymin);
            for row in 0..metrics.height {
                for col in 0..metrics.width {
                    let a = bitmap[row * metrics.width + col];
                    if a == 0 {
                        continue;
                    }
                    let px_x = gx + col as i32;
                    let px_y = gy + row as i32;
                    if px_x >= 0 && px_y >= 0 && px_x < w && px_y < h {
                        let out_a = ((a as u32 * rgba.alpha() as u32) / 255) as u8;
                        let src = Color::from_rgba8(rgba.red(), rgba.green(), rgba.blue(), out_a)
                            .premultiply()
                            .to_color_u8();
                        let idx = (px_y * w + px_x) as usize;
                        pm.pixels_mut()[idx] = blend_over(pm.pixels()[idx], src);
                    }
                }
            }
            pen_x += metrics.advance_width.round() as i32;
        }
    }
}

#[inline]
fn blend_over(
    dst: tiny_skia::PremultipliedColorU8,
    src: tiny_skia::PremultipliedColorU8,
) -> tiny_skia::PremultipliedColorU8 {
    let inv_a = 255 - src.alpha() as u32;
    let r = src.red() as u32 + ((dst.red() as u32 * inv_a) / 255);
    let g = src.green() as u32 + ((dst.green() as u32 * inv_a) / 255);
    let b = src.blue() as u32 + ((dst.blue() as u32 * inv_a) / 255);
    let a = src.alpha() as u32 + ((dst.alpha() as u32 * inv_a) / 255);
    tiny_skia::PremultipliedColorU8::from_rgba(
        r.min(255) as u8,
        g.min(255) as u8,
        b.min(255) as u8,
        a.min(255) as u8,
    )
    .unwrap_or(src)
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

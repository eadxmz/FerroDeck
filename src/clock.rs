//! Crisp 1:1 pixel-aligned clock rasterizer using `fontdue`.
//! Eliminates Android TextView/sp bilinear density scaling blur by rendering glyphs
//! directly into physical framebuffer coordinates and caching the rendered pixmap
//! until the formatted time string changes.

use fontdue::{Font, FontSettings};
use tiny_skia::{Color, Pixmap, PixmapPaint, Transform};

pub struct CrispClock {
    font_big: Option<Font>,
    font_ui: Option<Font>,
    cached_text: String,
    cached_style: u8,
    cached_size_px: u32,
    cached_pixmap: Option<Pixmap>,
}

impl CrispClock {
    pub fn new(big_font_bytes: Option<&[u8]>, ui_font_bytes: Option<&[u8]>) -> Self {
        let font_big = big_font_bytes
            .and_then(|b| Font::from_bytes(b, FontSettings { scale: 120.0, ..Default::default() }).ok());
        let font_ui = ui_font_bytes
            .and_then(|b| Font::from_bytes(b, FontSettings { scale: 48.0, ..Default::default() }).ok());
        Self {
            font_big,
            font_ui,
            cached_text: String::new(),
            cached_style: 255,
            cached_size_px: 0,
            cached_pixmap: None,
        }
    }

    /// Renders the clock string onto `target` at integer-snapped physical coordinates `(x, y)`.
    /// `style`: 0 = Big Glass Clock, 1 = Top-Right Header Clock, 2 = Hidden.
    pub fn draw(
        &mut self,
        target: &mut Pixmap,
        time_str: &str,
        style: u8,
        x: i32,
        y: i32,
        size_px: f32,
    ) {
        if style == 2 || time_str.is_empty() {
            return;
        }
        let snapped_size = size_px.round().max(12.0) as u32;
        if self.cached_text != time_str
            || self.cached_style != style
            || self.cached_size_px != snapped_size
            || self.cached_pixmap.is_none()
        {
            self.cached_text.clear();
            self.cached_text.push_str(time_str);
            self.cached_style = style;
            self.cached_size_px = snapped_size;
            self.cached_pixmap = self.rasterize(time_str, style, snapped_size as f32);
        }

        if let Some(ref pm) = self.cached_pixmap {
            target.draw_pixmap(
                x,
                y,
                pm.as_ref(),
                &PixmapPaint::default(),
                Transform::identity(),
                None,
            );
        }
    }

    fn rasterize(&self, text: &str, style: u8, px: f32) -> Option<Pixmap> {
        let font = if style == 0 {
            self.font_big.as_ref().or(self.font_ui.as_ref())
        } else {
            self.font_ui.as_ref().or(self.font_big.as_ref())
        }?;

        let mut glyphs = Vec::with_capacity(text.len());
        let mut total_w: i32 = 0;
        let mut max_ascent: i32 = 0;
        let mut max_descent: i32 = 0;

        for ch in text.chars() {
            let (metrics, bitmap) = font.rasterize(ch, px);
            let ascent = metrics.height as i32 + metrics.ymin;
            let descent = (-metrics.ymin).max(0);
            if ascent > max_ascent {
                max_ascent = ascent;
            }
            if descent > max_descent {
                max_descent = descent;
            }
            total_w += metrics.advance_width.round() as i32;
            glyphs.push((metrics, bitmap));
        }

        let pad = 6;
        let w = (total_w + pad * 2).max(1) as u32;
        let h = (max_ascent + max_descent + pad * 2).max(1) as u32;
        let mut pm = Pixmap::new(w, h)?;
        let baseline = pad + max_ascent;
        let mut pen_x = pad;

        for (metrics, bitmap) in glyphs {
            let gx = pen_x + metrics.xmin;
            let gy = baseline - (metrics.height as i32 + metrics.ymin);
            for row in 0..metrics.height {
                for col in 0..metrics.width {
                    let alpha = bitmap[row * metrics.width + col];
                    if alpha == 0 {
                        continue;
                    }
                    let px_x = gx + col as i32;
                    let px_y = gy + row as i32;
                    if px_x >= 0 && px_y >= 0 && (px_x as u32) < w && (px_y as u32) < h {
                        let shadow_x = px_x + 1;
                        let shadow_y = px_y + 2;
                        if (shadow_x as u32) < w && (shadow_y as u32) < h {
                            let s_idx = (shadow_y as u32 * w + shadow_x as u32) as usize;
                            let sa = ((alpha as u32 * 90) / 255) as u8;
                            if pm.pixels()[s_idx].alpha() == 0 {
                                pm.pixels_mut()[s_idx] =
                                    Color::from_rgba8(0, 0, 0, sa).premultiply().to_color_u8();
                            }
                        }
                        let idx = (px_y as u32 * w + px_x as u32) as usize;
                        let fg_a = if style == 0 {
                            ((alpha as u32 * 238) / 255) as u8
                        } else {
                            alpha
                        };
                        pm.pixels_mut()[idx] =
                            Color::from_rgba8(255, 255, 255, fg_a).premultiply().to_color_u8();
                    }
                }
            }
            pen_x += metrics.advance_width.round() as i32;
        }
        Some(pm)
    }
}

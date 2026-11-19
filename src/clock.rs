//! Crisp 1:1 pixel-aligned Glass Clock & Cached Multi-Script Text rasterizer using `fontdue`.

use fontdue::{Font, FontSettings, Metrics};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tiny_skia::{Color, Pixmap, PixmapPaint, PremultipliedColorU8, Transform};

static SCRIPT_FONTS: Lazy<Mutex<HashMap<&'static str, Option<Font>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

type GlyphKey = (usize, char, u16);
static GLYPH_CACHE: Lazy<Mutex<HashMap<GlyphKey, Arc<(Metrics, Vec<u8>)>>>> =
    Lazy::new(|| Mutex::new(HashMap::with_capacity(512)));

fn script_bucket(ch: char) -> Option<(&'static str, &'static [&'static str])> {
    let code = ch as u32;
    match code {
        0x0900..=0x097F => Some((
            "deva",
            &[
                "/system/fonts/NotoSansDevanagari-Regular.ttf",
                "/system/fonts/NotoSansDevanagariUI-Regular.ttf",
                "/system/fonts/NotoSansDevanagari-VF.ttf",
            ],
        )),
        0x0980..=0x09FF => Some((
            "beng",
            &[
                "/system/fonts/NotoSansBengali-Regular.ttf",
                "/system/fonts/NotoSansBengaliUI-Regular.ttf",
            ],
        )),
        0x0A00..=0x0A7F => Some((
            "guru",
            &[
                "/system/fonts/NotoSansGurmukhi-Regular.ttf",
                "/system/fonts/NotoSansGurmukhiUI-Regular.ttf",
            ],
        )),
        0x0A80..=0x0AFF => Some((
            "gujr",
            &[
                "/system/fonts/NotoSansGujarati-Regular.ttf",
                "/system/fonts/NotoSansGujaratiUI-Regular.ttf",
            ],
        )),
        0x0B80..=0x0BFF => Some((
            "taml",
            &[
                "/system/fonts/NotoSansTamil-Regular.ttf",
                "/system/fonts/NotoSansTamilUI-Regular.ttf",
            ],
        )),
        0x0C00..=0x0C7F => Some((
            "telu",
            &[
                "/system/fonts/NotoSansTelugu-Regular.ttf",
                "/system/fonts/NotoSansTeluguUI-Regular.ttf",
            ],
        )),
        0x0C80..=0x0CFF => Some((
            "knda",
            &[
                "/system/fonts/NotoSansKannada-Regular.ttf",
                "/system/fonts/NotoSansKannadaUI-Regular.ttf",
            ],
        )),
        0x0D00..=0x0D7F => Some((
            "mlym",
            &[
                "/system/fonts/NotoSansMalayalam-Regular.ttf",
                "/system/fonts/NotoSansMalayalamUI-Regular.ttf",
            ],
        )),
        0x0600..=0x06FF | 0x0750..=0x077F => Some((
            "arab",
            &[
                "/system/fonts/NotoNaskhArabic-Regular.ttf",
                "/system/fonts/NotoSansArabic-Regular.ttf",
                "/system/fonts/NotoNaskhArabicUI-Regular.ttf",
            ],
        )),
        0x3040..=0x30FF | 0x4E00..=0x9FFF | 0xAC00..=0xD7AF => Some((
            "cjk",
            &[
                "/system/fonts/NotoSansCJK-Regular.ttc",
                "/system/fonts/NotoSansJP-Regular.otf",
                "/system/fonts/NotoSansKR-Regular.otf",
                "/system/fonts/NotoSansSC-Regular.otf",
                "/system/fonts/DroidSansFallback.ttf",
            ],
        )),
        _ => None,
    }
}

fn rasterize_uncached(primary: &Font, ch: char, px: f32) -> (Metrics, Vec<u8>) {
    if let Some((key, paths)) = script_bucket(ch) {
        if let Ok(mut cache) = SCRIPT_FONTS.lock() {
            let entry = cache.entry(key).or_insert_with(|| {
                for path in paths {
                    if let Ok(bytes) = std::fs::read(path) {
                        if let Ok(f) = Font::from_bytes(
                            bytes,
                            FontSettings {
                                scale: 32.0,
                                ..Default::default()
                            },
                        ) {
                            return Some(f);
                        }
                    }
                }
                None
            });
            if let Some(ref fallback_font) = entry {
                if fallback_font.lookup_glyph_index(ch) != 0 {
                    return fallback_font.rasterize(ch, px);
                }
            }
        }
    }
    if primary.lookup_glyph_index(ch) == 0 && !ch.is_whitespace() {
        return primary.rasterize(' ', px);
    }
    primary.rasterize(ch, px)
}

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
        let font_big = big_font_bytes.and_then(|b| {
            Font::from_bytes(
                b,
                FontSettings {
                    scale: 180.0,
                    ..Default::default()
                },
            )
            .ok()
        });
        let font_ui = ui_font_bytes.and_then(|b| {
            Font::from_bytes(
                b,
                FontSettings {
                    scale: 48.0,
                    ..Default::default()
                },
            )
            .ok()
        });
        Self {
            font_big,
            font_ui,
            cached_text: String::new(),
            cached_style: 255,
            cached_size_px: 0,
            cached_pixmap: None,
        }
    }

    pub fn draw_centered(
        &mut self,
        target: &mut Pixmap,
        time_str: &str,
        style: u8,
        center_x: f32,
        top_y: i32,
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
            let x = (center_x - (pm.width() as f32 * 0.5)).round() as i32;
            target.draw_pixmap(
                x,
                top_y,
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
            let (metrics, bitmap) = rasterize_uncached(font, ch, px);
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

        let pad = 8;
        let w = (total_w + pad * 2).max(1) as u32;
        let h = (max_ascent + max_descent + pad * 2).max(1) as u32;
        let mut pm = Pixmap::new(w, h)?;
        let baseline = pad + max_ascent;
        let mut pen_x = pad;

        for (metrics, bitmap) in glyphs {
            let gx = pen_x + metrics.xmin;
            let gy = baseline - (metrics.height as i32 + metrics.ymin);
            let gh = metrics.height.max(1) as f32;

            for row in 0..metrics.height {
                let t = (row as f32 / gh).clamp(0.0, 1.0);
                let rim_boost = if t < 0.14 || t > 0.86 { 1.08 } else { 0.94 };

                for col in 0..metrics.width {
                    let alpha = bitmap[row * metrics.width + col];
                    if alpha == 0 {
                        continue;
                    }
                    let px_x = gx + col as i32;
                    let px_y = gy + row as i32;
                    if px_x >= 0 && px_y >= 0 && (px_x as u32) < w && (px_y as u32) < h {
                        let sx = px_x + 2;
                        let sy = px_y + 3;
                        if (sx as u32) < w && (sy as u32) < h {
                            let s_idx = (sy as u32 * w + sx as u32) as usize;
                            if pm.pixels()[s_idx].alpha() == 0 {
                                let sa = ((alpha as u32 * 72) / 255) as u8;
                                pm.pixels_mut()[s_idx] =
                                    Color::from_rgba8(8, 10, 18, sa).premultiply().to_color_u8();
                            }
                        }

                        let idx = (px_y as u32 * w + px_x as u32) as usize;
                        let base_alpha = if style == 0 { 222.0 } else { 248.0 };
                        let fg_a = ((alpha as f32 / 255.0) * base_alpha * rim_boost)
                            .clamp(0.0, 255.0) as u8;
                        let r = (242.0 + t * 13.0).min(255.0) as u8;
                        let g = (244.0 + t * 8.0).min(255.0) as u8;
                        let b = 255u8;
                        pm.pixels_mut()[idx] =
                            Color::from_rgba8(r, g, b, fg_a).premultiply().to_color_u8();
                    }
                }
            }
            pen_x += metrics.advance_width.round() as i32;
        }
        Some(pm)
    }
}

pub fn draw_crisp_label(
    pm: &mut Pixmap,
    font: Option<&Font>,
    text: &str,
    x: i32,
    baseline_y: i32,
    px: f32,
    color: Color,
    with_shadow: bool,
) {
    let Some(font) = font else { return };
    if text.is_empty() {
        return;
    }
    let w = pm.width() as i32;
    let h = pm.height() as i32;
    let rgba = color.to_color_u8();
    if rgba.alpha() == 0 {
        return;
    }
    let px_key = (px.round().clamp(6.0, 400.0)) as u16;
    let px_f = px_key as f32;
    let font_id = font as *const Font as usize;
    let mut pen_x = x;

    let Ok(mut cache) = GLYPH_CACHE.lock() else {
        return;
    };
    if cache.len() > 2400 {
        cache.clear();
    }

    let pixels = pm.pixels_mut();
    let fg_r = rgba.red() as u32;
    let fg_g = rgba.green() as u32;
    let fg_b = rgba.blue() as u32;
    let fg_a = rgba.alpha() as u32;

    for ch in text.chars() {
        let entry = cache
            .entry((font_id, ch, px_key))
            .or_insert_with(|| Arc::new(rasterize_uncached(font, ch, px_f)));
        let (ref metrics, ref bitmap) = **entry;

        let gx = pen_x + metrics.xmin;
        let gy = baseline_y - (metrics.height as i32 + metrics.ymin);
        let mw = metrics.width;
        let mh = metrics.height;

        for row in 0..mh {
            let px_y = gy + row as i32;
            if px_y < 0 || px_y >= h {
                continue;
            }
            let row_off = row * mw;
            let dst_row = (px_y * w) as usize;
            let sh_row = if with_shadow && px_y + 1 < h {
                Some(((px_y + 1) * w) as usize)
            } else {
                None
            };

            for col in 0..mw {
                let a = bitmap[row_off + col] as u32;
                if a == 0 {
                    continue;
                }
                let px_x = gx + col as i32;
                if px_x < 0 || px_x >= w {
                    continue;
                }

                if let Some(sr) = sh_row {
                    let sx = px_x + 1;
                    if sx < w {
                        let s_idx = sr + sx as usize;
                        let sa = ((a * fg_a * 105) / (255 * 255)) as u8;
                        let sh = PremultipliedColorU8::from_rgba(0, 0, 0, sa).unwrap_or(PremultipliedColorU8::TRANSPARENT);
                        pixels[s_idx] = blend_over(pixels[s_idx], sh);
                    }
                }

                let out_a = (a * fg_a) / 255;
                let pr = ((fg_r * out_a + 127) / 255) as u8;
                let pg = ((fg_g * out_a + 127) / 255) as u8;
                let pb = ((fg_b * out_a + 127) / 255) as u8;
                let src = PremultipliedColorU8::from_rgba(pr, pg, pb, out_a as u8).unwrap_or(PremultipliedColorU8::TRANSPARENT);
                let idx = dst_row + px_x as usize;
                pixels[idx] = blend_over(pixels[idx], src);
            }
        }
        pen_x += metrics.advance_width.round() as i32;
    }
}

pub fn measure_label_width(font: Option<&Font>, text: &str, px: f32) -> f32 {
    let Some(font) = font else {
        return text.len() as f32 * px * 0.5;
    };
    let px_f = px.round().clamp(6.0, 400.0);
    let mut sum = 0.0f32;
    for ch in text.chars() {
        if script_bucket(ch).is_none() && font.lookup_glyph_index(ch) != 0 {
            sum += font.metrics(ch, px_f).advance_width;
        } else {
            let px_key = px_f as u16;
            let font_id = font as *const Font as usize;
            if let Ok(mut cache) = GLYPH_CACHE.lock() {
                let entry = cache
                    .entry((font_id, ch, px_key))
                    .or_insert_with(|| Arc::new(rasterize_uncached(font, ch, px_f)));
                sum += entry.0.advance_width;
            } else {
                sum += font.metrics(ch, px_f).advance_width;
            }
        }
    }
    sum
}

#[inline]
pub fn blend_over(dst: PremultipliedColorU8, src: PremultipliedColorU8) -> PremultipliedColorU8 {
    let inv_a = 255 - src.alpha() as u32;
    let r = src.red() as u32 + ((dst.red() as u32 * inv_a) / 255);
    let g = src.green() as u32 + ((dst.green() as u32 * inv_a) / 255);
    let b = src.blue() as u32 + ((dst.blue() as u32 * inv_a) / 255);
    let a = src.alpha() as u32 + ((dst.alpha() as u32 * inv_a) / 255);
    PremultipliedColorU8::from_rgba(
        r.min(255) as u8,
        g.min(255) as u8,
        b.min(255) as u8,
        a.min(255) as u8,
    )
    .unwrap_or(src)
}

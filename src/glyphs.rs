//! Crisp 1:1 SF-Symbols-style vector glyphs drawn natively in `tiny-skia`.

use std::f32::consts::PI;
use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, Stroke, Transform,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Glyph {
    Wifi,
    Bluetooth,
    Memory,
    Moon,
    Audio,
    Timer,
    Ambient,
    Power,
    Input,
    Remote,
    Gear,
    Search,
    Cloud,
    Sliders,
    Scene,
    Apps,
    Person,
    Bolt,
    Globe,
    Shield,
    Update,
    Info,
}

fn stroke_paint(color: Color, width: f32) -> (Paint<'static>, Stroke) {
    let mut p = Paint::default();
    p.anti_alias = true;
    p.set_color(color);
    let mut st = Stroke::default();
    st.width = width.max(1.1);
    st.line_cap = LineCap::Round;
    st.line_join = LineJoin::Round;
    (p, st)
}

fn fill_paint(color: Color) -> Paint<'static> {
    let mut p = Paint::default();
    p.anti_alias = true;
    p.set_color(color);
    p
}

fn circle_path(cx: f32, cy: f32, r: f32) -> Option<Path> {
    PathBuilder::from_circle(cx, cy, r.max(0.5))
}

fn round_rect_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
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

fn arc_path(cx: f32, cy: f32, rx: f32, ry: f32, start_deg: f32, sweep_deg: f32) -> Option<Path> {
    let steps = ((sweep_deg.abs() / 12.0).ceil() as usize).max(6);
    let mut pb = PathBuilder::new();
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let ang = (start_deg + sweep_deg * t) * (PI / 180.0);
        let px = cx + rx * ang.cos();
        let py = cy + ry * ang.sin();
        if i == 0 {
            pb.move_to(px, py);
        } else {
            pb.line_to(px, py);
        }
    }
    pb.finish()
}

/// Draws `glyph` inside the square `[x, y, size, size]` in physical framebuffer pixels.
pub fn draw_glyph(pm: &mut Pixmap, glyph: Glyph, x: f32, y: f32, size: f32, color: Color) {
    let s = size.max(4.0);
    let m = s * 0.5;
    let sw = s * 0.085;
    let (sp, st) = stroke_paint(color, sw);
    let fp = fill_paint(color);

    match glyph {
        Glyph::Wifi => {
            for i in 0..3 {
                let rad = s * (0.17 + 0.155 * i as f32);
                if let Some(arc) = arc_path(x + m, y + s * 0.78, rad, rad, -135.0, 90.0) {
                    pm.stroke_path(&arc, &sp, &st, Transform::identity(), None);
                }
            }
            if let Some(dot) = circle_path(x + m, y + s * 0.78, s * 0.065) {
                pm.fill_path(&dot, &fp, FillRule::Winding, Transform::identity(), None);
            }
        }
        Glyph::Bluetooth => {
            let mut pb = PathBuilder::new();
            pb.move_to(x + s * 0.30, y + s * 0.32);
            pb.line_to(x + s * 0.70, y + s * 0.68);
            pb.line_to(x + m, y + s * 0.86);
            pb.line_to(x + m, y + s * 0.14);
            pb.line_to(x + s * 0.70, y + s * 0.32);
            pb.line_to(x + s * 0.30, y + s * 0.68);
            if let Some(path) = pb.finish() {
                pm.stroke_path(&path, &sp, &st, Transform::identity(), None);
            }
        }
        Glyph::Memory => {
            if let Some(body) = round_rect_path(x + s * 0.22, y + s * 0.28, s * 0.56, s * 0.44, s * 0.07) {
                pm.stroke_path(&body, &sp, &st, Transform::identity(), None);
            }
            let mut pins = PathBuilder::new();
            for i in 0..4 {
                let px = x + s * (0.30 + i as f32 * 0.133);
                pins.move_to(px, y + s * 0.15);
                pins.line_to(px, y + s * 0.28);
                pins.move_to(px, y + s * 0.72);
                pins.line_to(px, y + s * 0.85);
            }
            if let Some(pp) = pins.finish() {
                pm.stroke_path(&pp, &sp, &st, Transform::identity(), None);
            }
            if let Some(die) = round_rect_path(x + s * 0.37, y + s * 0.42, s * 0.15, s * 0.16, s * 0.02) {
                pm.fill_path(&die, &fp, FillRule::Winding, Transform::identity(), None);
            }
        }
        Glyph::Moon => {
            // Crescent moon built from two smooth cubic curves
            let mut pb = PathBuilder::new();
            pb.move_to(x + s * 0.56, y + s * 0.16);
            pb.cubic_to(
                x + s * 0.26,
                y + s * 0.18,
                x + s * 0.16,
                y + s * 0.46,
                x + s * 0.26,
                y + s * 0.68,
            );
            pb.cubic_to(
                x + s * 0.36,
                y + s * 0.86,
                x + s * 0.64,
                y + s * 0.88,
                x + s * 0.80,
                y + s * 0.72,
            );
            pb.cubic_to(
                x + s * 0.56,
                y + s * 0.74,
                x + s * 0.42,
                y + s * 0.56,
                x + s * 0.46,
                y + s * 0.36,
            );
            pb.cubic_to(
                x + s * 0.48,
                y + s * 0.26,
                x + s * 0.52,
                y + s * 0.20,
                x + s * 0.56,
                y + s * 0.16,
            );
            pb.close();
            if let Some(path) = pb.finish() {
                pm.fill_path(&path, &fp, FillRule::Winding, Transform::identity(), None);
            }
        }
        Glyph::Audio => {
            for i in 0..2 {
                let rad = s * (0.20 + 0.16 * i as f32);
                if let Some(arc) = arc_path(x + m, y + m, rad, rad, 140.0, 260.0) {
                    pm.stroke_path(&arc, &sp, &st, Transform::identity(), None);
                }
            }
            let mut pb = PathBuilder::new();
            pb.move_to(x + m, y + s * 0.52);
            pb.line_to(x + m + s * 0.20, y + s * 0.86);
            pb.line_to(x + m - s * 0.20, y + s * 0.86);
            pb.close();
            if let Some(cone) = pb.finish() {
                pm.fill_path(&cone, &fp, FillRule::Winding, Transform::identity(), None);
            }
        }
        Glyph::Timer => {
            if let Some(c) = circle_path(x + m, y + m, s * 0.35) {
                pm.stroke_path(&c, &sp, &st, Transform::identity(), None);
            }
            let mut pb = PathBuilder::new();
            pb.move_to(x + m, y + s * 0.28);
            pb.line_to(x + m, y + m);
            pb.line_to(x + m + s * 0.15, y + m + s * 0.10);
            if let Some(hands) = pb.finish() {
                pm.stroke_path(&hands, &sp, &st, Transform::identity(), None);
            }
        }
        Glyph::Ambient => {
            // 4-point main star + top-right companion star
            let draw_star = |pm: &mut Pixmap, cx: f32, cy: f32, r: f32| {
                let mut pb = PathBuilder::new();
                pb.move_to(cx, cy - r);
                pb.quad_to(cx, cy, cx + r, cy);
                pb.quad_to(cx, cy, cx, cy + r);
                pb.quad_to(cx, cy, cx - r, cy);
                pb.quad_to(cx, cy, cx, cy - r);
                pb.close();
                if let Some(star) = pb.finish() {
                    pm.fill_path(&star, &fp, FillRule::Winding, Transform::identity(), None);
                }
            };
            draw_star(pm, x + s * 0.44, y + s * 0.54, s * 0.31);
            draw_star(pm, x + s * 0.75, y + s * 0.26, s * 0.14);
        }
        Glyph::Power => {
            if let Some(arc) = arc_path(x + m, y + s * 0.52, s * 0.32, s * 0.32, -55.0, 290.0) {
                pm.stroke_path(&arc, &sp, &st, Transform::identity(), None);
            }
            let mut pb = PathBuilder::new();
            pb.move_to(x + m, y + s * 0.12);
            pb.line_to(x + m, y + s * 0.48);
            if let Some(stem) = pb.finish() {
                pm.stroke_path(&stem, &sp, &st, Transform::identity(), None);
            }
        }
        Glyph::Input => {
            let mut pb = PathBuilder::new();
            pb.move_to(x + s * 0.30, y + s * 0.36);
            pb.line_to(x + s * 0.30, y + s * 0.26);
            pb.quad_to(x + s * 0.30, y + s * 0.20, x + s * 0.38, y + s * 0.20);
            pb.line_to(x + s * 0.80, y + s * 0.20);
            pb.quad_to(x + s * 0.88, y + s * 0.20, x + s * 0.88, y + s * 0.28);
            pb.line_to(x + s * 0.88, y + s * 0.72);
            pb.quad_to(x + s * 0.88, y + s * 0.80, x + s * 0.80, y + s * 0.80);
            pb.line_to(x + s * 0.38, y + s * 0.80);
            pb.quad_to(x + s * 0.30, y + s * 0.80, x + s * 0.30, y + s * 0.74);
            pb.line_to(x + s * 0.30, y + s * 0.64);
            // Incoming arrow
            pb.move_to(x + s * 0.10, y + m);
            pb.line_to(x + s * 0.62, y + m);
            pb.move_to(x + s * 0.48, y + s * 0.37);
            pb.line_to(x + s * 0.62, y + m);
            pb.line_to(x + s * 0.48, y + s * 0.63);
            if let Some(path) = pb.finish() {
                pm.stroke_path(&path, &sp, &st, Transform::identity(), None);
            }
        }
        Glyph::Remote => {
            if let Some(body) = round_rect_path(x + s * 0.28, y + s * 0.10, s * 0.44, s * 0.80, s * 0.14) {
                pm.stroke_path(&body, &sp, &st, Transform::identity(), None);
            }
            if let Some(ring) = circle_path(x + m, y + s * 0.42, s * 0.11) {
                pm.stroke_path(&ring, &sp, &st, Transform::identity(), None);
            }
            for (dx, dy) in [(0.50, 0.22), (0.41, 0.65), (0.59, 0.65), (0.41, 0.76), (0.59, 0.76)] {
                if let Some(dot) = circle_path(x + s * dx, y + s * dy, s * 0.036) {
                    pm.fill_path(&dot, &fp, FillRule::Winding, Transform::identity(), None);
                }
            }
        }
        Glyph::Gear => {
            let outer_r = s * 0.33;
            let inner_r = s * 0.23;
            let teeth = 8;
            let mut pb = PathBuilder::new();
            for i in 0..(teeth * 2) {
                let r = if i % 2 == 0 { outer_r } else { inner_r };
                let a = (i as f32) * PI / (teeth as f32);
                let px = x + m + r * a.cos();
                let py = y + m + r * a.sin();
                if i == 0 {
                    pb.move_to(px, py);
                } else {
                    pb.line_to(px, py);
                }
            }
            pb.close();
            if let Some(gear) = pb.finish() {
                pm.stroke_path(&gear, &sp, &st, Transform::identity(), None);
            }
            if let Some(hole) = circle_path(x + m, y + m, s * 0.11) {
                pm.stroke_path(&hole, &sp, &st, Transform::identity(), None);
            }
        }
        Glyph::Search => {
            if let Some(lens) = circle_path(x + s * 0.44, y + s * 0.44, s * 0.24) {
                pm.stroke_path(&lens, &sp, &st, Transform::identity(), None);
            }
            let mut pb = PathBuilder::new();
            pb.move_to(x + s * 0.62, y + s * 0.62);
            pb.line_to(x + s * 0.82, y + s * 0.82);
            if let Some(handle) = pb.finish() {
                pm.stroke_path(&handle, &sp, &st, Transform::identity(), None);
            }
        }
        Glyph::Cloud => {
            let mut pb = PathBuilder::new();
            pb.move_to(x + s * 0.26, y + s * 0.72);
            pb.line_to(x + s * 0.74, y + s * 0.72);
            pb.cubic_to(
                x + s * 0.88,
                y + s * 0.72,
                x + s * 0.88,
                y + s * 0.48,
                x + s * 0.72,
                y + s * 0.46,
            );
            pb.cubic_to(
                x + s * 0.68,
                y + s * 0.24,
                x + s * 0.36,
                y + s * 0.24,
                x + s * 0.32,
                y + s * 0.46,
            );
            pb.cubic_to(
                x + s * 0.14,
                y + s * 0.48,
                x + s * 0.14,
                y + s * 0.72,
                x + s * 0.26,
                y + s * 0.72,
            );
            pb.close();
            if let Some(cloud) = pb.finish() {
                pm.stroke_path(&cloud, &sp, &st, Transform::identity(), None);
            }
        }
        Glyph::Sliders => {
            let mut pb = PathBuilder::new();
            pb.move_to(x + s * 0.16, y + s * 0.32);
            pb.line_to(x + s * 0.52, y + s * 0.32);
            pb.move_to(x + s * 0.74, y + s * 0.32);
            pb.line_to(x + s * 0.84, y + s * 0.32);
            pb.move_to(x + s * 0.16, y + s * 0.68);
            pb.line_to(x + s * 0.26, y + s * 0.68);
            pb.move_to(x + s * 0.48, y + s * 0.68);
            pb.line_to(x + s * 0.84, y + s * 0.68);
            if let Some(tracks) = pb.finish() {
                pm.stroke_path(&tracks, &sp, &st, Transform::identity(), None);
            }
            if let Some(k1) = circle_path(x + s * 0.63, y + s * 0.32, s * 0.09) {
                pm.stroke_path(&k1, &sp, &st, Transform::identity(), None);
            }
            if let Some(k2) = circle_path(x + s * 0.37, y + s * 0.68, s * 0.09) {
                pm.stroke_path(&k2, &sp, &st, Transform::identity(), None);
            }
        }
        Glyph::Scene => {
            if let Some(frame) = round_rect_path(x + s * 0.12, y + s * 0.18, s * 0.76, s * 0.64, s * 0.10) {
                pm.stroke_path(&frame, &sp, &st, Transform::identity(), None);
            }
            if let Some(sun) = circle_path(x + s * 0.34, y + s * 0.38, s * 0.065) {
                pm.fill_path(&sun, &fp, FillRule::Winding, Transform::identity(), None);
            }
            let mut pb = PathBuilder::new();
            pb.move_to(x + s * 0.16, y + s * 0.78);
            pb.line_to(x + s * 0.42, y + s * 0.50);
            pb.line_to(x + s * 0.58, y + s * 0.66);
            pb.line_to(x + s * 0.72, y + s * 0.46);
            pb.line_to(x + s * 0.85, y + s * 0.78);
            if let Some(peaks) = pb.finish() {
                pm.stroke_path(&peaks, &sp, &st, Transform::identity(), None);
            }
        }
        Glyph::Apps => {
            for (rx, ry) in [(0.15, 0.15), (0.56, 0.15), (0.15, 0.56), (0.56, 0.56)] {
                if let Some(sq) = round_rect_path(x + s * rx, y + s * ry, s * 0.29, s * 0.29, s * 0.08) {
                    pm.stroke_path(&sq, &sp, &st, Transform::identity(), None);
                }
            }
        }
        Glyph::Person => {
            if let Some(head) = circle_path(x + m, y + s * 0.35, s * 0.16) {
                pm.stroke_path(&head, &sp, &st, Transform::identity(), None);
            }
            if let Some(sh) = arc_path(x + m, y + s * 0.86, s * 0.31, s * 0.26, 195.0, 150.0) {
                pm.stroke_path(&sh, &sp, &st, Transform::identity(), None);
            }
        }
        Glyph::Bolt => {
            let mut pb = PathBuilder::new();
            pb.move_to(x + s * 0.54, y + s * 0.12);
            pb.line_to(x + s * 0.23, y + s * 0.55);
            pb.line_to(x + s * 0.47, y + s * 0.55);
            pb.line_to(x + s * 0.44, y + s * 0.88);
            pb.line_to(x + s * 0.77, y + s * 0.44);
            pb.line_to(x + s * 0.52, y + s * 0.44);
            pb.close();
            if let Some(bolt) = pb.finish() {
                pm.stroke_path(&bolt, &sp, &st, Transform::identity(), None);
            }
        }
        Glyph::Globe => {
            if let Some(c) = circle_path(x + m, y + m, s * 0.36) {
                pm.stroke_path(&c, &sp, &st, Transform::identity(), None);
            }
            let mut pb = PathBuilder::new();
            pb.move_to(x + s * 0.14, y + m);
            pb.line_to(x + s * 0.86, y + m);
            if let Some(eq) = pb.finish() {
                pm.stroke_path(&eq, &sp, &st, Transform::identity(), None);
            }
            if let Some(meridian) = round_rect_path(x + s * 0.33, y + s * 0.14, s * 0.34, s * 0.72, s * 0.17) {
                pm.stroke_path(&meridian, &sp, &st, Transform::identity(), None);
            }
        }
        Glyph::Shield => {
            let mut pb = PathBuilder::new();
            pb.move_to(x + m, y + s * 0.12);
            pb.line_to(x + s * 0.80, y + s * 0.26);
            pb.cubic_to(
                x + s * 0.80,
                y + s * 0.58,
                x + s * 0.68,
                y + s * 0.78,
                x + m,
                y + s * 0.90,
            );
            pb.cubic_to(
                x + s * 0.32,
                y + s * 0.78,
                x + s * 0.20,
                y + s * 0.58,
                x + s * 0.20,
                y + s * 0.26,
            );
            pb.close();
            if let Some(sh) = pb.finish() {
                pm.stroke_path(&sh, &sp, &st, Transform::identity(), None);
            }
            let mut chk = PathBuilder::new();
            chk.move_to(x + s * 0.38, y + s * 0.50);
            chk.line_to(x + s * 0.47, y + s * 0.60);
            chk.line_to(x + s * 0.64, y + s * 0.40);
            if let Some(cp) = chk.finish() {
                pm.stroke_path(&cp, &sp, &st, Transform::identity(), None);
            }
        }
        Glyph::Update => {
            if let Some(c) = circle_path(x + m, y + m, s * 0.36) {
                pm.stroke_path(&c, &sp, &st, Transform::identity(), None);
            }
            let mut pb = PathBuilder::new();
            pb.move_to(x + m, y + s * 0.30);
            pb.line_to(x + m, y + s * 0.66);
            pb.move_to(x + s * 0.36, y + s * 0.52);
            pb.line_to(x + m, y + s * 0.68);
            pb.line_to(x + s * 0.64, y + s * 0.52);
            if let Some(arr) = pb.finish() {
                pm.stroke_path(&arr, &sp, &st, Transform::identity(), None);
            }
        }
        Glyph::Info => {
            if let Some(c) = circle_path(x + m, y + m, s * 0.36) {
                pm.stroke_path(&c, &sp, &st, Transform::identity(), None);
            }
            let mut pb = PathBuilder::new();
            pb.move_to(x + m, y + s * 0.46);
            pb.line_to(x + m, y + s * 0.69);
            if let Some(line) = pb.finish() {
                pm.stroke_path(&line, &sp, &st, Transform::identity(), None);
            }
            if let Some(dot) = circle_path(x + m, y + s * 0.32, s * 0.048) {
                pm.fill_path(&dot, &fp, FillRule::Winding, Transform::identity(), None);
            }
        }
    }
}

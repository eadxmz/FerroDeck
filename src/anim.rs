//! Snappy frame-rate-independent easing helpers.
//! Settles in ~6-10 frames (~120ms) so ALooper returns immediately to 0.0% idle CPU.

use tiny_skia::Color;

#[inline]
pub fn smooth_to(cur: &mut f32, target: f32, speed: f32, dt: f32, eps: f32) -> bool {
    let diff = target - *cur;
    if diff.abs() <= eps {
        *cur = target;
        false
    } else {
        let factor = (1.0 - (-speed * dt).exp()).clamp(0.16, 1.0);
        *cur += diff * factor;
        if (target - *cur).abs() <= eps {
            *cur = target;
            false
        } else {
            true
        }
    }
}

#[inline]
pub fn ease_out_cubic(t: f32) -> f32 {
    let x = t.clamp(0.0, 1.0);
    1.0 - (1.0 - x).powi(3)
}

#[inline]
pub fn with_alpha(c: Color, alpha: f32) -> Color {
    let f = alpha.clamp(0.0, 1.0);
    if f >= 0.999 {
        return c;
    }
    let u = c.to_color_u8();
    let a = ((u.alpha() as f32) * f).round().clamp(0.0, 255.0) as u8;
    Color::from_rgba8(u.red(), u.green(), u.blue(), a)
}

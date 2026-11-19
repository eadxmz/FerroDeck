//! Frame-rate-independent fluid animation helpers (exponential ease-out & lively tvOS springs).
//! All helpers snap cleanly to target within `eps` so `ALooper` returns to 0.0% idle CPU.

use tiny_skia::Color;

/// Exponential frame-rate-independent smoothing toward `target`.
/// Returns `true` while still moving, `false` once settled.
#[inline]
pub fn smooth_to(cur: &mut f32, target: f32, speed: f32, dt: f32, eps: f32) -> bool {
    let diff = target - *cur;
    if diff.abs() <= eps {
        *cur = target;
        false
    } else {
        let factor = 1.0 - (-speed * dt).exp();
        *cur += diff * factor;
        true
    }
}

/// Lightly-damped spring physics for crisp tvOS focus lift & pill pop.
/// Returns `true` while still moving, `false` once settled.
#[inline]
pub fn spring_to(
    cur: &mut f32,
    vel: &mut f32,
    target: f32,
    stiffness: f32,
    damping: f32,
    dt: f32,
    eps: f32,
) -> bool {
    let disp = target - *cur;
    if disp.abs() <= eps && vel.abs() <= eps * 8.0 {
        *cur = target;
        *vel = 0.0;
        return false;
    }
    let accel = disp * stiffness - (*vel) * damping;
    *vel += accel * dt;
    *cur += (*vel) * dt;
    true
}

/// Smooth cubic ease-out curve for modal entrance/exit offsets.
#[inline]
pub fn ease_out_cubic(t: f32) -> f32 {
    let x = t.clamp(0.0, 1.0);
    1.0 - (1.0 - x).powi(3)
}

/// Scales a `tiny_skia::Color`'s alpha channel by `alpha` (`0.0..=1.0`).
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

//! `meteordevelopment.meteorclient.utils.render.color` - `Color`, `SettingColor`
//! and the rainbow bookkeeping, ported literally.

use std::sync::atomic::{AtomicBool, Ordering};

/// RGBA colour with the same component order and the same arithmetic as the
/// Java `Color` class (`r`, `g`, `b`, `a` as unsigned bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const GRAY: Color = Color::rgb(128, 128, 128);
    pub const LIGHT_GRAY: Color = Color::rgb(192, 192, 192);
    pub const DARK_GRAY: Color = Color::rgb(64, 64, 64);
    pub const RED: Color = Color::rgb(255, 0, 0);
    pub const GREEN: Color = Color::rgb(0, 255, 0);
    pub const BLUE: Color = Color::rgb(0, 0, 255);
    pub const YELLOW: Color = Color::rgb(255, 255, 0);
    pub const ORANGE: Color = Color::rgb(255, 128, 0);
    pub const CYAN: Color = Color::rgb(0, 255, 255);
    pub const MAGENTA: Color = Color::rgb(255, 0, 255);

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// `new Color(r, g, b, a)` from ints, clamped like Java's byte casts.
    pub fn from_ints(r: i32, g: i32, b: i32, a: i32) -> Self {
        Self {
            r: r.clamp(0, 255) as u8,
            g: g.clamp(0, 255) as u8,
            b: b.clamp(0, 255) as u8,
            a: a.clamp(0, 255) as u8,
        }
    }

    /// The packed form Meteor's renderer hands the GPU (`0xAARRGGBB`).
    pub const fn argb(self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    pub const fn from_argb(v: u32) -> Self {
        Self {
            a: ((v >> 24) & 0xFF) as u8,
            r: ((v >> 16) & 0xFF) as u8,
            g: ((v >> 8) & 0xFF) as u8,
            b: (v & 0xFF) as u8,
        }
    }

    pub fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }

    /// `Color.fromHsv(hue, saturation, value)`; hue in degrees.
    pub fn from_hsv(hue: f64, saturation: f64, value: f64) -> Self {
        let h = ((hue % 360.0) + 360.0) % 360.0 / 60.0;
        let c = value * saturation;
        let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
        let m = value - c;
        let (r, g, b) = match h as i32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        Self::from_ints(
            ((r + m) * 255.0) as i32,
            ((g + m) * 255.0) as i32,
            ((b + m) * 255.0) as i32,
            255,
        )
    }

    /// Component-wise multiply, as `Color.set(Color)` does in the renderer.
    pub fn mul(self, o: Color) -> Self {
        Self {
            r: ((self.r as u32 * o.r as u32) / 255) as u8,
            g: ((self.g as u32 * o.g as u32) / 255) as u8,
            b: ((self.b as u32 * o.b as u32) / 255) as u8,
            a: ((self.a as u32 * o.a as u32) / 255) as u8,
        }
    }

    /// Linear interpolation, used by ESP fades and the HUD.
    pub fn lerp(self, o: Color, t: f64) -> Self {
        let t = t.clamp(0.0, 1.0);
        let f = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t).round() as i32;
        Self::from_ints(
            f(self.r, o.r),
            f(self.g, o.g),
            f(self.b, o.b),
            f(self.a, o.a),
        )
    }
}

/// `SettingColor` - a `Color` that can follow the rainbow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingColor {
    pub color: Color,
    pub rainbow: bool,
}

impl SettingColor {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            color: Color::new(r, g, b, a),
            rainbow: false,
        }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }

    pub fn from_color(color: Color) -> Self {
        Self {
            color,
            rainbow: false,
        }
    }
}

impl Default for SettingColor {
    fn default() -> Self {
        Self::rgb(255, 255, 255)
    }
}

impl From<SettingColor> for Color {
    fn from(s: SettingColor) -> Color {
        s.color
    }
}

/// `RainbowColors` - one global switch plus a shared phase, as in Java.
pub struct RainbowColors;

static RAINBOW_AMBIENT: AtomicBool = AtomicBool::new(false);

impl RainbowColors {
    /// `RainbowColors.get()`: the colour used by rainbow-enabled `SettingColor`s.
    pub fn get() -> Color {
        Color::from_hsv(Self::time(), 0.5, 1.0)
    }

    /// The shared hue phase (`RainbowColors.getTime`), in degrees.
    pub fn time() -> f64 {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as f64)
            .unwrap_or(0.0);
        (t / 20.0) % 360.0
    }

    pub fn set_ambient(on: bool) {
        RAINBOW_AMBIENT.store(on, Ordering::Relaxed);
    }

    pub fn ambient() -> bool {
        RAINBOW_AMBIENT.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argb_round_trip() {
        let c = Color::new(145, 61, 226, 200);
        assert_eq!(Color::from_argb(c.argb()), c);
        assert_eq!(c.argb(), 0xC891_3DE2);
    }

    #[test]
    fn hsv_matches_java_for_known_values() {
        // `Color.fromHsv(0, 0, 0)` is black, `(0, 0, 1)` white, `(0, 1, 1)` red.
        assert_eq!(Color::from_hsv(0.0, 0.0, 0.0), Color::BLACK);
        assert_eq!(Color::from_hsv(0.0, 0.0, 1.0), Color::WHITE);
        assert_eq!(Color::from_hsv(0.0, 1.0, 1.0), Color::RED);
        assert_eq!(Color::from_hsv(120.0, 1.0, 1.0), Color::GREEN);
    }

    #[test]
    fn lerp_endpoints() {
        let a = Color::BLACK;
        let b = Color::WHITE;
        assert_eq!(a.lerp(b, 0.0), a);
        assert_eq!(a.lerp(b, 1.0), b);
    }
}

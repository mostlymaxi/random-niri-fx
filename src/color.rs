#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgb(pub f32, pub f32, pub f32);

impl Rgb {
    pub const WHITE: Rgb = Rgb(1.0, 1.0, 1.0);

    pub const fn hex(v: u32) -> Rgb {
        Rgb(
            ((v >> 16) & 0xff) as f32 / 255.0,
            ((v >> 8) & 0xff) as f32 / 255.0,
            (v & 0xff) as f32 / 255.0,
        )
    }

    /// Parses `rrggbb` with an optional leading `#`.
    pub fn parse(s: &str) -> Option<Rgb> {
        let s = s.strip_prefix('#').unwrap_or(s);
        if s.len() != 6 {
            return None;
        }
        u32::from_str_radix(s, 16).ok().map(Rgb::hex)
    }

    pub fn lerp(self, other: Rgb, t: f32) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        Rgb(
            self.0 + (other.0 - self.0) * t,
            self.1 + (other.1 - self.1) * t,
            self.2 + (other.2 - self.2) * t,
        )
    }

    pub fn alpha(self, a: f32) -> tiny_skia::Color {
        tiny_skia::Color::from_rgba(
            self.0.clamp(0.0, 1.0),
            self.1.clamp(0.0, 1.0),
            self.2.clamp(0.0, 1.0),
            a.clamp(0.0, 1.0),
        )
        .expect("components are clamped")
    }
}

impl Default for Rgb {
    fn default() -> Self {
        Rgb::WHITE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex() {
        assert_eq!(Rgb::parse("#ff0000"), Some(Rgb(1.0, 0.0, 0.0)));
        assert_eq!(Rgb::parse("00ff00"), Some(Rgb(0.0, 1.0, 0.0)));
        assert_eq!(Rgb::parse("nope"), None);
    }
}

//! The time-of-day palette: colors pinned to times of day, blended in
//! between and wrapping around midnight.

use crate::color::Rgb;

#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    /// `(hour, color)`, sorted by hour, hours in `[0, 24)`, never empty.
    keys: Vec<(f32, Rgb)>,
}

impl Default for Palette {
    /// Soft at night, warm at the edges of the day, cool around noon.
    fn default() -> Self {
        Self {
            keys: vec![
                (0.0, Rgb::hex(0x6a5acd)),
                (5.0, Rgb::hex(0x7b6fd6)),
                (7.0, Rgb::hex(0xff8a7a)),
                (10.0, Rgb::hex(0xffd27a)),
                (13.0, Rgb::hex(0x9be7ff)),
                (17.0, Rgb::hex(0xffa25a)),
                (19.0, Rgb::hex(0xf2609e)),
                (21.5, Rgb::hex(0xaf7ad6)),
            ],
        }
    }
}

impl Palette {
    /// Parses `07:00 #ff8a7a, 13:00 #9be7ff, ...`. Times must be increasing.
    pub fn parse(s: &str) -> Option<Palette> {
        let keys = s
            .split(',')
            .map(|entry| {
                let (time, color) = entry.trim().split_once(char::is_whitespace)?;
                Some((parse_time(time)?, Rgb::parse(color.trim())?))
            })
            .collect::<Option<Vec<_>>>()?;
        let increasing = keys.windows(2).all(|w| w[0].0 < w[1].0);
        (!keys.is_empty() && increasing).then_some(Palette { keys })
    }

    /// Color at `day`, the fraction of the day elapsed.
    pub fn at(&self, day: f32) -> Rgb {
        let hour = day.rem_euclid(1.0) * 24.0;
        let next = self.keys.iter().position(|&(h, _)| h > hour);
        // Neighbours on either side, unrolling across midnight when needed.
        let (h0, c0) = match next {
            Some(0) | None => {
                let (h, c) = *self.keys.last().unwrap();
                (if next.is_some() { h - 24.0 } else { h }, c)
            }
            Some(i) => self.keys[i - 1],
        };
        let (h1, c1) = match next {
            Some(i) => self.keys[i],
            None => (self.keys[0].0 + 24.0, self.keys[0].1),
        };
        if h1 <= h0 {
            return c0;
        }
        c0.lerp(c1, (hour - h0) / (h1 - h0))
    }
}

/// `HH:MM` or `HH`.
fn parse_time(s: &str) -> Option<f32> {
    let (h, m) = s.split_once(':').unwrap_or((s, "0"));
    let (h, m): (f32, f32) = (h.parse().ok()?, m.parse().ok()?);
    let hour = h + m / 60.0;
    ((0.0..24.0).contains(&hour) && (0.0..60.0).contains(&m)).then_some(hour)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Rgb = Rgb(1.0, 0.0, 0.0);
    const BLUE: Rgb = Rgb(0.0, 0.0, 1.0);

    #[test]
    fn hits_keyframes() {
        let p = Palette::default();
        assert_eq!(p.at(13.0 / 24.0), Rgb::hex(0x9be7ff));
        assert_eq!(p.at(0.0), Rgb::hex(0x6a5acd));
    }

    #[test]
    fn wraps_around_midnight() {
        let p = Palette::parse("06:00 #ff0000, 18:00 #0000ff").unwrap();
        assert_eq!(p.at(0.25), RED);
        assert_eq!(p.at(0.75), BLUE);
        // Midnight is halfway from 18:00 back round to 06:00.
        assert_eq!(p.at(0.0), BLUE.lerp(RED, 0.5));
        assert_eq!(p.at(0.0), p.at(1.0));
    }

    #[test]
    fn single_color_is_constant() {
        let p = Palette::parse("12:00 #ff0000").unwrap();
        assert_eq!(p.at(0.1), RED);
        assert_eq!(p.at(0.9), RED);
    }

    #[test]
    fn rejects_bad_input() {
        assert_eq!(Palette::parse(""), None);
        assert_eq!(Palette::parse("25:00 #ff0000"), None);
        assert_eq!(Palette::parse("12:00 red"), None);
        assert_eq!(Palette::parse("12:00 #ff0000, 06:00 #0000ff"), None);
    }
}

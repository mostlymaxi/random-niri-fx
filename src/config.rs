//! Optional `key = value` config at `$XDG_CONFIG_HOME/random-fx/config`.

use std::path::PathBuf;

use crate::color::Rgb;
use crate::palette::Palette;
use crate::pill::Pill;

#[derive(Clone, Debug)]
pub struct Config {
    /// Range of seconds a state plays, rolled each time it is entered. The
    /// machine picks again at the first sweep boundary after that.
    pub dwell: (f32, f32),
    /// Seconds the line takes to cross the screen.
    pub flight: f32,
    /// Seconds of stillness between sweeps.
    pub rest: f32,
    /// Height of the overlay strip, in pixels.
    pub height: u32,
    /// Enabled states, by name. Empty means all.
    pub states: Vec<String>,
    /// Line color through the day.
    pub palette: Palette,
    /// Color the line runs toward under heavy CPU load.
    pub hot: Rgb,
    /// Default color for `zap` events that don't name one.
    pub zap: Rgb,
    /// Chance of a spontaneous zap, in the line's own color, at each decision
    /// point (once the current state's dwell is up).
    pub zap_chance: f32,
    /// Where the waybar clock pill is, for the half-hourly chime to wrap.
    pub clock: Option<Pill>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            dwell: (3.0, 5.0),
            flight: 2.2,
            rest: 1.4,
            height: 24,
            states: Vec::new(),
            palette: Palette::default(),
            hot: Rgb::hex(0xff3b30),
            zap: Rgb::hex(0xbfe9ff),
            zap_chance: 0.01,
            clock: None,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let Some(path) = path() else {
            return Self::default();
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => Self::parse(&text),
            Err(_) => Self::default(),
        }
    }

    pub fn parse(text: &str) -> Self {
        let mut cfg = Self::default();
        for (n, raw) in text.lines().enumerate() {
            let line = strip_comment(raw).trim();
            if line.is_empty() {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                eprintln!("random-fx: config line {}: expected `key = value`", n + 1);
                continue;
            };
            let (key, value) = (key.trim(), value.trim());
            let ok = match key {
                "dwell" => parse_range(value).map(|v| cfg.dwell = v).is_some(),
                "flight" => value.parse().map(|v| cfg.flight = v).is_ok(),
                "rest" => value.parse().map(|v| cfg.rest = v).is_ok(),
                "height" => value.parse().map(|v| cfg.height = v).is_ok(),
                "palette" => Palette::parse(value).map(|v| cfg.palette = v).is_some(),
                "hot" => Rgb::parse(value).map(|v| cfg.hot = v).is_some(),
                "zap" => Rgb::parse(value).map(|v| cfg.zap = v).is_some(),
                "zap_chance" => value
                    .parse()
                    .ok()
                    .filter(|p| (0.0..=1.0).contains(p))
                    .map(|v| cfg.zap_chance = v)
                    .is_some(),
                "clock" => Pill::parse(value).map(|v| cfg.clock = Some(v)).is_some(),
                "states" => {
                    cfg.states = value.split(',').map(|s| s.trim().to_owned()).collect();
                    true
                }
                _ => false,
            };
            if !ok {
                eprintln!("random-fx: config line {}: bad `{key}`", n + 1);
            }
        }
        cfg
    }

    pub fn enabled(&self, state: &str) -> bool {
        self.states.is_empty() || self.states.iter().any(|s| s == state)
    }
}

/// Drops a `#` comment. A `#` directly followed by a non-space character is
/// part of a value (a hex color), not a comment.
fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let end = (0..bytes.len())
        .find(|&i| bytes[i] == b'#' && bytes.get(i + 1).is_none_or(|b| b.is_ascii_whitespace()))
        .unwrap_or(line.len());
    &line[..end]
}

/// `3` or `3-5`.
fn parse_range(value: &str) -> Option<(f32, f32)> {
    let (lo, hi) = value.split_once('-').unwrap_or((value, value));
    let (lo, hi) = (lo.trim().parse().ok()?, hi.trim().parse().ok()?);
    (lo <= hi).then_some((lo, hi))
}

fn path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("random-fx/config"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_values_and_ignores_comments() {
        let cfg = Config::parse("dwell = 5 # longer\n\nstates = sweep, zap\nbogus\n");
        assert_eq!(cfg.dwell, (5.0, 5.0));
        assert!(cfg.enabled("zap"));
        assert!(!cfg.enabled("heat"));
    }

    #[test]
    fn parses_colors_next_to_comments() {
        let cfg = Config::parse(
            "# colors\nhot = #ff0000 # red\nzap=#00ff00\npalette = 06:00 #0000ff, 18:00 #ffffff\n",
        );
        assert_eq!(cfg.hot, Rgb(1.0, 0.0, 0.0));
        assert_eq!(cfg.zap, Rgb(0.0, 1.0, 0.0));
        assert_eq!(
            cfg.palette,
            Palette::parse("06:00 #0000ff, 18:00 #ffffff").unwrap()
        );
    }

    #[test]
    fn parses_dwell_range() {
        assert_eq!(Config::parse("dwell = 3-5").dwell, (3.0, 5.0));
        assert_eq!(Config::parse("dwell = 5-3").dwell, Config::default().dwell);
    }
}

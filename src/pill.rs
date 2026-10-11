//! A rounded "pill" on screen (the waybar clock), and lines drawn around
//! its outline.
//!
//! Positions on the outline are `u` in `-1..=1`: 0 is the rightmost point,
//! positive values run over the top and negative ones under the bottom, and
//! both ends meet at the leftmost point.

use std::f32::consts::PI;

use tiny_skia::{FillRule, LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, Stroke, Transform};

use crate::color::Rgb;

/// Room around the pill on its surface for the halo, in pixels.
pub const PAD: u32 = 12;

/// The pill's border box in screen pixels; its ends are fully rounded.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pill {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Something to draw on the pill.
#[derive(Clone, Debug)]
pub enum Mark {
    /// A streak from `tail` to `head` (outline positions), tapering to a
    /// point at its tail, with a rounded head.
    Trace {
        tail: f32,
        head: f32,
        color: Rgb,
        alpha: f32,
        width: f32,
    },
    /// The whole outline lit up.
    Ring { color: Rgb, alpha: f32, width: f32 },
}

impl Pill {
    /// Parses `x y w h`.
    pub fn parse(s: &str) -> Option<Pill> {
        let v: Vec<u32> = s
            .split_whitespace()
            .map(|n| n.parse().ok())
            .collect::<Option<_>>()?;
        match v[..] {
            [x, y, w, h] if w >= h && h >= 2 => Some(Pill { x, y, w, h }),
            _ => None,
        }
    }

    /// Size of the surface drawn over the pill.
    pub fn surface_size(&self) -> (u32, u32) {
        (self.w + 2 * PAD, self.h + 2 * PAD)
    }

    /// Screen position of the surface's top-left corner.
    pub fn surface_origin(&self) -> (i32, i32) {
        (self.x as i32 - PAD as i32, self.y as i32 - PAD as i32)
    }

    /// Radius and centers of the end caps, in surface coordinates, along the
    /// centerline of the pill's 1px border.
    fn shape(&self) -> (f32, f32, f32, f32) {
        let r = (self.h as f32 - 1.0) / 2.0;
        let cy = PAD as f32 + 0.5 + r;
        let left = PAD as f32 + 0.5 + r;
        let right = PAD as f32 + self.w as f32 - 0.5 - r;
        (r, cy, left, right)
    }

    /// Point and outward normal at outline position `u`.
    fn at(&self, u: f32) -> ((f32, f32), (f32, f32)) {
        let (r, cy, left, right) = self.shape();
        let quarter = PI * r / 2.0;
        let straight = right - left;
        let s = u.abs().min(1.0) * (2.0 * quarter + straight);
        // Walk over the top; mirror for the bottom half.
        let (center, angle) = if s < quarter {
            (right, -s / r)
        } else if s < quarter + straight {
            let x = right - (s - quarter);
            return mirror(u, cy, ((x, cy - r), (0.0, -1.0)));
        } else {
            (left, -PI / 2.0 - (s - quarter - straight) / r)
        };
        let n = (angle.cos(), angle.sin());
        mirror(u, cy, ((center + r * n.0, cy + r * n.1), n))
    }

    pub fn paint(&self, pixmap: &mut Pixmap, marks: &[Mark]) {
        for mark in marks {
            match *mark {
                Mark::Trace {
                    tail,
                    head,
                    color,
                    alpha,
                    width,
                } => {
                    let layers = [(3.0, 1.0, alpha * 0.15), (1.0, 0.0, alpha)];
                    for (scale, pad, a) in layers {
                        let hw = |s: f32| (width / 2.0 * s).max(0.2) * scale + pad;
                        if let Some(path) = self.trace(tail, head, hw) {
                            fill(pixmap, &path, color, a);
                        }
                        let ((x, y), _) = self.at(head);
                        if let Some(cap) = PathBuilder::from_circle(x, y, hw(1.0)) {
                            fill(pixmap, &cap, color, a);
                        }
                    }
                }
                Mark::Ring {
                    color,
                    alpha,
                    width,
                } => {
                    let ring = self.ring();
                    for (w, a) in [(width * 3.0 + 2.0, alpha * 0.15), (width, alpha)] {
                        let mut paint = Paint::default();
                        paint.set_color(color.alpha(a));
                        paint.anti_alias = true;
                        let stroke = Stroke {
                            width: w,
                            line_cap: LineCap::Round,
                            line_join: LineJoin::Round,
                            ..Default::default()
                        };
                        pixmap.stroke_path(&ring, &paint, &stroke, Transform::identity(), None);
                    }
                }
            }
        }
    }

    /// Ribbon along the outline from `tail` to `head`, half-thickness `hw(s)`
    /// for `s` from 0 at the tail to 1 at the head.
    fn trace(&self, tail: f32, head: f32, hw: impl Fn(f32) -> f32) -> Option<Path> {
        const SAMPLES: usize = 64;
        let side: Vec<_> = (0..=SAMPLES)
            .map(|i| {
                let s = i as f32 / SAMPLES as f32;
                let ((x, y), (nx, ny)) = self.at(tail + (head - tail) * s);
                (x, y, nx * hw(s), ny * hw(s))
            })
            .collect();
        let mut pb = PathBuilder::new();
        let (x, y, dx, dy) = side[0];
        pb.move_to(x + dx, y + dy);
        for &(x, y, dx, dy) in &side[1..] {
            pb.line_to(x + dx, y + dy);
        }
        for &(x, y, dx, dy) in side.iter().rev() {
            pb.line_to(x - dx, y - dy);
        }
        pb.close();
        pb.finish()
    }

    fn ring(&self) -> Path {
        const SAMPLES: usize = 128;
        let mut pb = PathBuilder::new();
        for i in 0..=SAMPLES {
            let u = -1.0 + 2.0 * i as f32 / SAMPLES as f32;
            let ((x, y), _) = self.at(u);
            if i == 0 {
                pb.move_to(x, y);
            } else {
                pb.line_to(x, y);
            }
        }
        pb.close();
        pb.finish().expect("ring has points")
    }
}

/// Reflects a top-half point and normal into the bottom half when `u < 0`.
fn mirror(
    u: f32,
    cy: f32,
    ((x, y), (nx, ny)): ((f32, f32), (f32, f32)),
) -> ((f32, f32), (f32, f32)) {
    if u < 0.0 {
        ((x, 2.0 * cy - y), (nx, -ny))
    } else {
        ((x, y), (nx, ny))
    }
}

fn fill(pixmap: &mut Pixmap, path: &Path, color: Rgb, alpha: f32) {
    let mut paint = Paint::default();
    paint.set_color(color.alpha(alpha));
    paint.anti_alias = true;
    pixmap.fill_path(path, &paint, FillRule::Winding, Transform::identity(), None);
}

#[cfg(test)]
mod tests {
    use super::*;

    const PILL: Pill = Pill {
        x: 0,
        y: 0,
        w: 101,
        h: 41,
    };

    fn close(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3
    }

    #[test]
    fn outline_landmarks() {
        let p = PAD as f32;
        // Rightmost, top middle, leftmost; the bottom mirrors the top.
        assert!(close(PILL.at(0.0).0, (p + 100.5, p + 20.5)));
        assert!(close(PILL.at(0.5).0, (p + 50.5, p + 0.5)));
        assert!(close(PILL.at(-0.5).0, (p + 50.5, p + 40.5)));
        assert!(close(PILL.at(1.0).0, (p + 0.5, p + 20.5)));
        assert!(close(PILL.at(1.0).0, PILL.at(-1.0).0));
    }

    #[test]
    fn parses_geometry() {
        assert_eq!(
            Pill::parse("2291 1395 261 41"),
            Some(Pill {
                x: 2291,
                y: 1395,
                w: 261,
                h: 41
            })
        );
        assert_eq!(Pill::parse("1 2 3"), None);
        assert_eq!(Pill::parse("0 0 10 40"), None);
    }
}

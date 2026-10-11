//! The line: what a state shapes each frame, and the code that paints it.
//!
//! A line lives on one screen edge, in that edge's own coordinates: `along`
//! runs the length of the edge, `inward` points into the screen (see `edge`).
//! It is drawn as a bar sitting flush on the edge, reaching inward.

use tiny_skia::{
    FillRule, GradientStop, LinearGradient, Paint, Path, PathBuilder, Pixmap, Point, SpreadMode,
    Transform,
};

use crate::color::Rgb;

/// Distance along the edge between displacement samples, in pixels.
pub const STEP: f32 = 4.0;

#[derive(Clone)]
pub struct Line {
    /// Visible span as fractions of the edge length. Hidden when `head <= tail`.
    pub tail: f32,
    pub head: f32,
    pub color: Rgb,
    pub alpha: f32,
    /// Depth at the head, in pixels from the screen edge.
    pub width: f32,
    /// How much the line narrows toward its tail: 0 = uniform, 1 = a point.
    pub taper: f32,
    /// Strength of the soft halo around the core, 0..1.
    pub glow: f32,
    /// How much the line fades out toward its tail, 0..1.
    pub tail_fade: f32,
    /// Which part of a longer streak this is, from 0 (its tail) to 1 (its
    /// head). A streak turning a corner is drawn as one piece per edge, and
    /// this keeps its taper and fade continuous. `(0, 1)` for a whole line.
    pub span: (f32, f32),
    /// Extra inward depth per `STEP` pixels along the full edge. Negative
    /// values are ignored: the line never leaves the edge.
    pub offsets: Vec<f32>,
}

impl Line {
    pub fn new(length: f32) -> Self {
        let mut line = Self {
            tail: 0.0,
            head: 0.0,
            color: Rgb::WHITE,
            alpha: 1.0,
            width: 5.0,
            taper: 1.0,
            glow: 0.6,
            tail_fade: 0.7,
            span: (0.0, 1.0),
            offsets: Vec::new(),
        };
        line.resize(length);
        line
    }

    pub fn resize(&mut self, length: f32) {
        let samples = (length / STEP).ceil() as usize + 2;
        self.offsets.resize(samples, 0.0);
    }

    /// Restores defaults for a new frame, keeping the allocation.
    pub fn reset(&mut self) {
        let mut offsets = std::mem::take(&mut self.offsets);
        offsets.fill(0.0);
        *self = Self {
            offsets,
            ..Self::new(0.0)
        };
    }

    pub fn is_visible(&self) -> bool {
        self.head > self.tail && self.alpha > 0.0
    }

    /// Displacement at `along`, linearly interpolated between samples.
    pub fn offset_at(&self, along: f32) -> f32 {
        let i = (along / STEP).max(0.0);
        let lo = (i.floor() as usize).min(self.offsets.len() - 1);
        let hi = (lo + 1).min(self.offsets.len() - 1);
        let f = i - i.floor();
        self.offsets[lo] * (1.0 - f) + self.offsets[hi] * f
    }

    /// Calls `f(along_pixels, along_fraction, &mut offset)` for every sample.
    pub fn displace(&mut self, mut f: impl FnMut(f32, f32, &mut f32)) {
        let length = (self.offsets.len() - 2) as f32 * STEP;
        for (i, o) in self.offsets.iter_mut().enumerate() {
            let x = i as f32 * STEP;
            f(x, (x / length).min(1.0), o);
        }
    }

    /// Paints the line as a bar on the edge (plus a soft halo of the same
    /// shape) with a rounded head, on an edge `length` pixels long.
    /// `transform` maps edge coordinates onto the pixmap.
    pub fn paint(&self, pixmap: &mut Pixmap, length: f32, transform: Transform) {
        if !self.is_visible() {
            return;
        }
        let (x0, x1) = (self.tail * length, self.head * length);
        if x1 - x0 < 0.5 {
            return;
        }

        let mut xs = vec![x0];
        let mut x = (x0 / STEP).floor() * STEP + STEP;
        while x < x1 {
            xs.push(x);
            x += STEP;
        }
        xs.push(x1);

        // Position within the whole streak, 0 at its tail and 1 at its head.
        let (s0, s1) = self.span;
        let streak = |x: f32| s0 + (s1 - s0) * (x - x0) / (x1 - x0);
        let depth = |x: f32| (self.width * (1.0 - self.taper * (1.0 - streak(x)))).max(0.4);
        let bump = |x: f32| self.offset_at(x).max(0.0);
        let fade = |s: f32| 1.0 - self.tail_fade * (1.0 - s);
        let is_head = s1 >= 1.0;

        // (thickness scale, extra padding, alpha): soft halo, then the body.
        let layers = [
            (3.0, 1.0, self.alpha * self.glow * 0.15),
            (1.0, 0.0, self.alpha),
        ];
        for (scale, pad, alpha) in layers {
            if alpha <= 0.0 {
                continue;
            }
            let inner = |x: f32| depth(x) * scale + pad + bump(x);
            let Some(body) = bar(&xs, inner) else {
                continue;
            };
            let paint = self.gradient(x0, x1, alpha * fade(s0), alpha * fade(s1));
            pixmap.fill_path(&body, &paint, FillRule::Winding, transform, None);

            if !is_head {
                continue;
            }
            // Centered on the edge, so only its inner half shows: a rounded nub.
            let r = depth(x1) * scale + pad;
            if let Some(cap) = PathBuilder::from_circle(x1, bump(x1), r) {
                let mut paint = Paint::default();
                paint.set_color(self.color.alpha(alpha));
                paint.anti_alias = true;
                pixmap.fill_path(&cap, &paint, FillRule::Winding, transform, None);
            }
        }
    }

    fn gradient(&self, x0: f32, x1: f32, a0: f32, a1: f32) -> Paint<'static> {
        let stops = vec![
            GradientStop::new(0.0, self.color.alpha(a0)),
            GradientStop::new(1.0, self.color.alpha(a1)),
        ];
        let shader = LinearGradient::new(
            Point::from_xy(x0, 0.0),
            Point::from_xy(x1, 0.0),
            stops,
            SpreadMode::Pad,
            Transform::identity(),
        )
        .expect("non-degenerate gradient");
        Paint {
            shader,
            anti_alias: true,
            ..Default::default()
        }
    }
}

/// Closed outline from the edge (`inward = 0`) to `inner(x)` for `x` in `xs`.
fn bar(xs: &[f32], inner: impl Fn(f32) -> f32) -> Option<Path> {
    let mut pb = PathBuilder::new();
    pb.move_to(xs[0], 0.0);
    for &x in xs {
        pb.line_to(x, inner(x));
    }
    pb.line_to(xs[xs.len() - 1], 0.0);
    pb.close();
    pb.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_keeps_sample_count() {
        let mut line = Line::new(100.0);
        let n = line.offsets.len();
        line.offsets[3] = 5.0;
        line.head = 1.0;
        line.reset();
        assert_eq!(line.offsets.len(), n);
        assert_eq!(line.offsets[3], 0.0);
        assert!(!line.is_visible());
    }

    #[test]
    fn offsets_interpolate() {
        let mut line = Line::new(100.0);
        line.offsets[0] = 0.0;
        line.offsets[1] = 4.0;
        assert!((line.offset_at(STEP / 2.0) - 2.0).abs() < 1e-5);
    }
}

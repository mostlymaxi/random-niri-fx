//! The four screen edges, and routing a streak along a path made of them.
//!
//! Every edge has its own coordinates: `along` runs left to right on the
//! horizontal edges and top to bottom on the vertical ones, and `inward`
//! points into the screen. A strip is `thickness` pixels deep.

use tiny_skia::Transform;

use crate::line::Line;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Top,
    Right,
    Bottom,
    Left,
}

impl Edge {
    pub const ALL: [Edge; 4] = [Edge::Top, Edge::Right, Edge::Bottom, Edge::Left];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn is_horizontal(self) -> bool {
        matches!(self, Edge::Top | Edge::Bottom)
    }

    /// Length in pixels on a `screen` of `(width, height)`.
    pub fn length(self, screen: (f32, f32)) -> f32 {
        if self.is_horizontal() {
            screen.0
        } else {
            screen.1
        }
    }

    /// Maps `(along, inward)` onto this edge's strip.
    pub fn transform(self, thickness: f32) -> Transform {
        // from_row(sx, ky, kx, sy, tx, ty): x' = sx·x + kx·y + tx, y' = ky·x + sy·y + ty
        match self {
            Edge::Top => Transform::identity(),
            Edge::Bottom => Transform::from_row(1.0, 0.0, 0.0, -1.0, 0.0, thickness),
            Edge::Left => Transform::from_row(0.0, 1.0, 1.0, 0.0, 0.0, 0.0),
            Edge::Right => Transform::from_row(0.0, 1.0, -1.0, 0.0, thickness, 0.0),
        }
    }
}

/// Splits the streak between `tail` and `head` (pixels along `path`) into
/// one `Line` per edge it touches, each a copy of `style` positioned on its
/// edge. The path's edges are walked in order, each in its `along` direction.
pub fn route(
    path: &[Edge],
    screen: (f32, f32),
    tail: f32,
    head: f32,
    style: &Line,
    out: &mut Vec<(Edge, Line)>,
) {
    if head <= tail {
        return;
    }
    let mut start = 0.0;
    for &edge in path {
        let len = edge.length(screen);
        let (lo, hi) = (tail.max(start), head.min(start + len));
        if lo < hi {
            let mut piece = style.clone();
            piece.resize(len);
            piece.tail = (lo - start) / len;
            piece.head = (hi - start) / len;
            piece.span = ((lo - tail) / (head - tail), (hi - tail) / (head - tail));
            out.push((edge, piece));
        }
        start += len;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_splits_at_corner() {
        let mut out = Vec::new();
        let path = [Edge::Top, Edge::Right];
        route(&path, (100.0, 50.0), 80.0, 120.0, &Line::new(0.0), &mut out);
        assert_eq!(out.len(), 2);
        let (top, right) = (&out[0].1, &out[1].1);
        assert_eq!((out[0].0, out[1].0), (Edge::Top, Edge::Right));
        assert!((top.tail - 0.8).abs() < 1e-5 && (top.head - 1.0).abs() < 1e-5);
        assert!((right.tail - 0.0).abs() < 1e-5 && (right.head - 0.4).abs() < 1e-5);
        assert!((top.span.1 - 0.5).abs() < 1e-5 && (right.span.0 - 0.5).abs() < 1e-5);
    }

    #[test]
    fn transforms_put_inward_into_the_screen() {
        let t = 20.0;
        let map = |e: Edge, along: f32, inward: f32| {
            let mut p = [tiny_skia::Point::from_xy(along, inward)];
            e.transform(t).map_points(&mut p);
            (p[0].x, p[0].y)
        };
        assert_eq!(map(Edge::Bottom, 5.0, 0.0), (5.0, 20.0));
        assert_eq!(map(Edge::Left, 5.0, 0.0), (0.0, 5.0));
        assert_eq!(map(Edge::Right, 5.0, 0.0), (20.0, 5.0));
    }
}

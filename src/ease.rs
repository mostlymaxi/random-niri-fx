//! CSS-style `cubic-bezier(x1, y1, x2, y2)` timing curves.

#[derive(Clone, Copy)]
pub struct CubicBezier {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

impl CubicBezier {
    pub const fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    /// Eased progress for linear progress `x` in `[0, 1]`.
    pub fn at(&self, x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);
        let t = self.solve_t(x);
        bezier(t, self.y1, self.y2)
    }

    /// Finds the curve parameter `t` whose x coordinate is `x`.
    /// Newton's method, falling back to bisection when the slope is flat.
    fn solve_t(&self, x: f32) -> f32 {
        let mut t = x;
        for _ in 0..8 {
            let err = bezier(t, self.x1, self.x2) - x;
            if err.abs() < 1e-5 {
                return t;
            }
            let slope = bezier_slope(t, self.x1, self.x2);
            if slope.abs() < 1e-6 {
                break;
            }
            t -= err / slope;
        }
        let (mut lo, mut hi) = (0.0, 1.0);
        t = x;
        for _ in 0..20 {
            if bezier(t, self.x1, self.x2) < x {
                lo = t;
            } else {
                hi = t;
            }
            t = (lo + hi) / 2.0;
        }
        t
    }
}

/// One axis of a cubic bezier with endpoints fixed at 0 and 1.
fn bezier(t: f32, p1: f32, p2: f32) -> f32 {
    let u = 1.0 - t;
    3.0 * u * u * t * p1 + 3.0 * u * t * t * p2 + t * t * t
}

fn bezier_slope(t: f32, p1: f32, p2: f32) -> f32 {
    let u = 1.0 - t;
    3.0 * u * u * p1 + 6.0 * u * t * (p2 - p1) + 3.0 * t * t * (1.0 - p2)
}

/// Progress through a move at time `t` in `[0, 1]` that starts at speed `v0`
/// and ends at speed `v1`, both relative to the move's average speed (cubic
/// Hermite). Speeds are scaled down together if needed so the move never
/// backtracks.
pub fn hermite(t: f32, v0: f32, v1: f32) -> f32 {
    // v0² + v1² ≤ 9 keeps a cubic Hermite segment monotonic.
    let scale = (3.0 / v0.hypot(v1)).min(1.0);
    let (v0, v1) = (v0 * scale, v1 * scale);
    let t = t.clamp(0.0, 1.0);
    let (t2, t3) = (t * t, t * t * t);
    (-2.0 * t3 + 3.0 * t2) + v0 * (t3 - 2.0 * t2 + t) + v1 * (t3 - t2)
}

/// Frame-rate independent exponential smoothing toward `target`.
pub fn approach(current: f32, target: f32, tau: f32, dt: f32) -> f32 {
    current + (target - current) * (1.0 - (-dt / tau).exp())
}

pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_are_fixed() {
        let c = CubicBezier::new(0.65, 0.0, 0.35, 1.0);
        assert!(c.at(0.0).abs() < 1e-4);
        assert!((c.at(1.0) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn linear_curve_is_identity() {
        let c = CubicBezier::new(1.0 / 3.0, 1.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0);
        for i in 0..=10 {
            let x = i as f32 / 10.0;
            assert!((c.at(x) - x).abs() < 1e-3);
        }
    }

    #[test]
    fn hermite_hits_its_speeds_and_never_backtracks() {
        let slope = |t: f32| (hermite(t + 1e-3, 0.3, 2.5) - hermite(t, 0.3, 2.5)) / 1e-3;
        assert!(hermite(0.0, 0.3, 2.5).abs() < 1e-6);
        assert!((hermite(1.0, 0.3, 2.5) - 1.0).abs() < 1e-6);
        assert!((slope(0.0) - 0.3).abs() < 0.01);
        assert!((slope(0.998) - 2.5).abs() < 0.02);
        let mut last = 0.0;
        for i in 0..=100 {
            let p = hermite(i as f32 / 100.0, 0.5, 4.0);
            assert!(p >= last - 1e-6);
            last = p;
        }
    }

    #[test]
    fn ease_in_out_is_symmetric() {
        let c = CubicBezier::new(0.65, 0.0, 0.35, 1.0);
        assert!((c.at(0.5) - 0.5).abs() < 1e-3);
        assert!(c.at(0.25) < 0.25);
    }
}

//! The race shared by `zap` and `chime`: two streaks leave the top-left
//! corner, one along the top and down the right edge, the other down the
//! left edge and along the bottom. Along the first edge each accelerates on
//! the first half of the main sweep's curve, reaching its top speed as it
//! turns the corner, then holds that speed down the second edge into the
//! collision. The turns come at different distances, so each route gets its
//! own top speed (the slower one under the main sweep's) such that both
//! arrive together. Each turn gives off a small lingering spark; the
//! collision sets off a flare splashing back along both edges.

use super::Ctx;
use crate::color::Rgb;
use crate::ease::CubicBezier;
use crate::edge::{route, Edge};
use crate::line::Line;
use crate::motion::{self, Comet};

/// The two routes, each walked in its edges' `along` direction.
const PATHS: [[Edge; 2]; 2] = [[Edge::Top, Edge::Right], [Edge::Left, Edge::Bottom]];
/// Tail chase while racing.
const CHASE: f32 = 0.12;
/// Seconds a corner spark lingers, and how far it reaches along each edge.
const SPARK: f32 = 0.8;
const SPARK_REACH: f32 = 70.0;
/// Seconds the flare takes to splash out and fade.
pub const FLARE: f32 = 0.35;
const FLARE_EASE: CubicBezier = CubicBezier::new(0.1, 0.9, 0.2, 1.0);
/// How far the flare splashes back along each edge, in pixels.
const FLARE_REACH: f32 = 260.0;

pub struct Collision {
    comets: [Comet; 2],
    /// When each streak turned its corner, in seconds since launch.
    turned: [Option<f32>; 2],
}

impl Collision {
    pub fn new() -> Self {
        Self {
            comets: [Comet::new(CHASE), Comet::new(CHASE)],
            turned: [None; 2],
        }
    }

    /// Starts a new race.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Seconds from launch to the collision: as long as the route that
    /// needs the most time at the main sweep's top speed.
    pub fn impact(ctx: &Ctx) -> f32 {
        let top = motion::peak_speed(ctx.screen.0, ctx.flight);
        PATHS
            .iter()
            .map(|path| effort(path.map(|e| e.length(ctx.screen))) / top)
            .fold(0.0, f32::max)
    }

    /// Draws the race at `ctx.age` in `color`: the streaks, their corner
    /// sparks, then the flare once they have met. The streaks keep drawing
    /// after impact so their tails get pulled into the corner while the
    /// flare splashes out.
    pub fn draw(&mut self, ctx: &Ctx, color: Rgb, out: &mut Vec<(Edge, Line)>) {
        let style = Line {
            color: color.lerp(Rgb::WHITE, 0.3),
            glow: 1.0,
            tail_fade: 0.9,
            ..Line::new(0.0)
        };
        let impact = Self::impact(ctx);
        for (i, path) in PATHS.iter().enumerate() {
            let legs = path.map(|e| e.length(ctx.screen));
            let length: f32 = legs.iter().sum();
            let head = head_along(legs, ctx.age, effort(legs) / impact);
            if head >= legs[0] && self.turned[i].is_none() {
                self.turned[i] = Some(ctx.age);
            }
            let (tail, head) = self.comets[i].follow(head / length, ctx.dt);
            route(path, ctx.screen, tail * length, head * length, &style, out);

            if let Some(at) = self.turned[i] {
                spark(ctx, (path[0], path[1]), ctx.age - at, &style, out);
            }
        }

        if ctx.age < impact {
            return;
        }
        let f = ((ctx.age - impact) / FLARE).clamp(0.0, 1.0);
        let flare = Line {
            color: color.lerp(Rgb::WHITE, 0.5 * (1.0 - f)),
            width: 7.0,
            taper: 0.8,
            alpha: (1.0 - f) * (1.0 - f),
            ..style
        };
        // Each splash ends ("heads") at the corner, so it is thickest there.
        for edge in [Edge::Right, Edge::Bottom] {
            let length = edge.length(ctx.screen);
            let reach = FLARE_REACH * FLARE_EASE.at(f) + 1.0;
            route(&[edge], ctx.screen, length - reach, length, &flare, out);
        }
    }
}

/// A spark `age` seconds old in the corner where `from` ends and `into`
/// begins: a short glow on both edges, brightest in the corner.
fn spark(
    ctx: &Ctx,
    (from, into): (Edge, Edge),
    age: f32,
    style: &Line,
    out: &mut Vec<(Edge, Line)>,
) {
    let f = age / SPARK;
    if f >= 1.0 {
        return;
    }
    let reach = SPARK_REACH * FLARE_EASE.at(f * 3.0) + 1.0;
    let glow = Line {
        color: style.color.lerp(Rgb::WHITE, 0.4 * (1.0 - f)),
        width: 5.0,
        taper: 0.7,
        alpha: (1.0 - f) * (1.0 - f),
        ..style.clone()
    };
    // The corner is the end of `from` and the start of `into`; both pieces
    // have their head there.
    let len = from.length(ctx.screen);
    out.push((
        from,
        Line {
            tail: (len - reach) / len,
            head: 1.0,
            ..glow.clone()
        },
    ));
    let len = into.length(ctx.screen);
    let mut start = Line {
        tail: reach / len,
        head: 0.0,
        ..glow
    };
    start.resize(len);
    out.push((into, start));
}

/// Seconds a route of `legs` takes at a top speed of one pixel per second:
/// the first leg on the accelerating curve, the second at top speed.
fn effort([first, second]: [f32; 2]) -> f32 {
    motion::peak_slope() * first + second
}

/// Head position in pixels, `age` seconds after launch, along a route of
/// `legs` run at a top speed of `top` pixels per second. It keeps going
/// past the end; the route clips it there.
fn head_along([first, _]: [f32; 2], age: f32, top: f32) -> f32 {
    let turn = motion::peak_slope() * first / top;
    if age < turn {
        first * motion::accelerate(age / turn)
    } else {
        first + top * (age - turn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_meet_at_the_end_at_full_speed() {
        let routes = [[2560.0, 1440.0], [1440.0, 2560.0]];
        let impact = 2.0;
        for legs in routes {
            let top = effort(legs) / impact;
            let at = |age: f32| head_along(legs, age, top);
            assert!((at(impact) - 4000.0).abs() < 0.5);
            // Full speed from the turn on.
            let turn = motion::peak_slope() * legs[0] / top;
            assert!((at(turn) - legs[0]).abs() < 0.5);
            assert!(((at(impact) - at(impact - 0.1)) / 0.1 - top).abs() < 1.0);
        }
    }
}

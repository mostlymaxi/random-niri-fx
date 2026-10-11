//! The race shared by `zap` and `chime`: two streaks leave the top-left
//! corner, one along the top and down the right edge, the other down the
//! left edge and along the bottom. Each leg is a sweep of its own: it eases
//! out of a corner and eases into the next, where the following leg picks
//! straight up. Legs are timed, not paced, so both streaks turn their first
//! corners together and meet in the bottom-right one, where a flare
//! splashes back along both edges.

use super::Ctx;
use crate::color::Rgb;
use crate::ease::CubicBezier;
use crate::edge::{route, Edge};
use crate::line::Line;
use crate::motion::Comet;

/// The two routes, each walked in its edges' `along` direction.
const PATHS: [[Edge; 2]; 2] = [[Edge::Top, Edge::Right], [Edge::Left, Edge::Bottom]];
/// Seconds per leg, whatever its length.
const LEG: f32 = 0.9;
/// The same in-out curve as the main sweep.
const LEG_EASE: CubicBezier = CubicBezier::new(0.65, 0.0, 0.35, 1.0);
/// Tail chase while racing.
const CHASE: f32 = 0.12;
/// Seconds from launch until the streaks reach the final corner.
const STRIKE: f32 = 2.0 * LEG;
/// When the flare goes off: a touch before the heads have crept all the way
/// into the corner, since the end of the ease is too slow to read as impact.
pub const IMPACT: f32 = STRIKE - 0.1;
/// Seconds the flare takes to splash out and fade.
pub const FLARE: f32 = 0.35;
const FLARE_EASE: CubicBezier = CubicBezier::new(0.1, 0.9, 0.2, 1.0);
/// How far the flare splashes back along each edge, in pixels.
const FLARE_REACH: f32 = 260.0;

pub struct Collision {
    comets: [Comet; 2],
}

impl Collision {
    pub fn new() -> Self {
        Self {
            comets: [Comet::new(CHASE), Comet::new(CHASE)],
        }
    }

    /// Starts a new race.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Draws the race at `ctx.age` in `color`: the streaks, then the flare
    /// once they have met. The streaks keep drawing after impact so their
    /// tails get pulled into the corner while the flare splashes out.
    pub fn draw(&mut self, ctx: &Ctx, color: Rgb, out: &mut Vec<(Edge, Line)>) {
        let style = Line {
            color: color.lerp(Rgb::WHITE, 0.3),
            glow: 1.0,
            tail_fade: 0.9,
            ..Line::new(0.0)
        };
        for (path, comet) in PATHS.iter().zip(&mut self.comets) {
            let legs = path.map(|e| e.length(ctx.screen));
            let length: f32 = legs.iter().sum();
            let head = head_along(ctx.age, &legs);
            let (tail, head) = comet.follow(head / length, ctx.dt);
            route(path, ctx.screen, tail * length, head * length, &style, out);
        }

        if ctx.age < IMPACT {
            return;
        }
        let f = ((ctx.age - IMPACT) / FLARE).clamp(0.0, 1.0);
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

/// Head position in pixels along a path of `legs`, `age` seconds in.
fn head_along(age: f32, legs: &[f32]) -> f32 {
    let mut done = 0.0;
    for (i, &leg) in legs.iter().enumerate() {
        let start = i as f32 * LEG;
        let p = ((age - start) / LEG).clamp(0.0, 1.0);
        if p < 1.0 || i == legs.len() - 1 {
            return done + LEG_EASE.at(p) * leg;
        }
        done += leg;
    }
    done
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_turns_the_corner_between_legs() {
        let legs = [100.0, 50.0];
        assert_eq!(head_along(0.0, &legs), 0.0);
        assert!((head_along(LEG, &legs) - 100.0).abs() < 1e-3);
        assert!(head_along(LEG * 1.5, &legs) > 100.0);
        assert!((head_along(STRIKE, &legs) - 150.0).abs() < 1e-3);
        assert!((head_along(STRIKE + 5.0, &legs) - 150.0).abs() < 1e-3);
    }
}

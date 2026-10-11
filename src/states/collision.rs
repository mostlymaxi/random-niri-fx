//! The race shared by `zap` and `chime`: two streaks leave the top-left
//! corner, one along the top and down the right edge, the other down the
//! left edge and along the bottom. They pick up speed exactly as the main
//! sweep does, but never ease off: once at its top speed they hold it all
//! the way into the collision. Both routes are the same length, so they meet
//! in the bottom-right corner together. Each turn gives off a small
//! lingering spark. The collision bursts in the corner and throws a streak
//! back out along each edge, which slows and fades as it goes.

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
/// Fast out, long glide: for things thrown out of an impact.
const BURST_EASE: CubicBezier = CubicBezier::new(0.1, 0.9, 0.2, 1.0);
/// Seconds the burst in the corner lasts, and how far it reaches.
const BURST: f32 = 0.3;
const BURST_REACH: f32 = 150.0;
/// Seconds the rebounding streaks travel and fade, and how far they get.
const REBOUND: f32 = 0.7;
const REBOUND_REACH: f32 = 360.0;
/// The tail runs this many times slower than the head, so as the streak
/// glides to a halt it stays drawn out instead of shrinking to a dot.
const REBOUND_TAIL: f32 = 0.25;
/// Seconds from impact until the explosion is over.
pub const FLARE: f32 = REBOUND;

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

    /// Seconds from launch to the collision.
    pub fn impact(ctx: &Ctx) -> f32 {
        let route: f32 = PATHS[0].iter().map(|e| e.length(ctx.screen)).sum();
        motion::capped_time(route, ctx.screen.0, ctx.flight)
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
        // Moving like the main sweep across the screen, minus the easing off.
        let head = motion::capped(ctx.age, ctx.screen.0, ctx.flight);
        for (i, path) in PATHS.iter().enumerate() {
            let legs = path.map(|e| e.length(ctx.screen));
            let length: f32 = legs.iter().sum();
            if head >= legs[0] && self.turned[i].is_none() {
                self.turned[i] = Some(ctx.age);
            }
            let (tail, head) = self.comets[i].follow(head / length, ctx.dt);
            route(path, ctx.screen, tail * length, head * length, &style, out);

            if let Some(at) = self.turned[i] {
                spark(ctx, (path[0], path[1]), ctx.age - at, &style, out);
            }
        }

        let impact = Self::impact(ctx);
        if ctx.age < impact {
            return;
        }
        let since = ctx.age - impact;
        // Both edges end in the impact corner.
        for edge in [Edge::Right, Edge::Bottom] {
            let length = edge.length(ctx.screen);

            // The burst: thickest in the corner, flashing out and gone.
            let f = (since / BURST).min(1.0);
            if f < 1.0 {
                let burst = Line {
                    color: color.lerp(Rgb::WHITE, 0.6 * (1.0 - f)),
                    width: 9.0,
                    taper: 0.8,
                    alpha: (1.0 - f) * (1.0 - f),
                    ..style.clone()
                };
                let reach = BURST_REACH * BURST_EASE.at(f) + 1.0;
                route(&[edge], ctx.screen, length - reach, length, &burst, out);
            }

            // The rebound: running backwards, away from the corner.
            let f = (since / REBOUND).min(1.0);
            if f < 1.0 {
                let travel = |t: f32| REBOUND_REACH * BURST_EASE.at(t / REBOUND);
                let mut rebound = Line {
                    tail: (length - travel(since * REBOUND_TAIL)) / length,
                    head: (length - travel(since)) / length,
                    color: color.lerp(Rgb::WHITE, 0.3 * (1.0 - f)),
                    width: 4.0,
                    tail_fade: 1.0,
                    alpha: (1.0 - f).powf(1.5),
                    ..style.clone()
                };
                rebound.resize(length);
                out.push((edge, rebound));
            }
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
    let reach = SPARK_REACH * BURST_EASE.at(f * 3.0) + 1.0;
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

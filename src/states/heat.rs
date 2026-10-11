//! When the CPU is pegged the line runs hot: red, shimmering, and sweeping
//! faster the harder the machine works.

use super::{Ctx, State};
use crate::color::Rgb;
use crate::ease::smoothstep;
use crate::line::Line;
use crate::rng::Rng;
use crate::signals::Signals;

/// Peak shimmer in pixels.
const SHIMMER: f32 = 1.2;
/// Sweep speed-up at full heat.
const HURRY: f32 = 0.7;

pub struct Heat {
    rng: Rng,
    hot: Rgb,
}

impl Heat {
    pub fn new(hot: Rgb) -> Self {
        Self {
            rng: Rng::seeded(),
            hot,
        }
    }
}

fn heat(s: &Signals) -> f32 {
    smoothstep(0.6, 0.9, s.cpu)
}

impl State for Heat {
    fn name(&self) -> &'static str {
        "heat"
    }

    fn want(&self, s: &Signals) -> f32 {
        heat(s)
    }

    fn tempo(&self, s: &Signals) -> f32 {
        1.0 + HURRY * heat(s)
    }

    fn update(&mut self, ctx: &Ctx, line: &mut Line) {
        let h = heat(ctx.signals);
        let rng = &mut self.rng;
        line.displace(|_, _, offset| *offset = rng.range(0.0, SHIMMER) * h);
        line.color = ctx.signals.tint.lerp(self.hot, 0.4 + 0.6 * h);
        line.glow = 1.0;
    }
}

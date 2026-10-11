//! The default: the plain sweep in the time-of-day color.

use super::{Ctx, State, AMBIENT};
use crate::line::Line;
use crate::signals::Signals;

pub struct Sweep;

impl State for Sweep {
    fn name(&self) -> &'static str {
        "sweep"
    }

    fn want(&self, _: &Signals) -> f32 {
        AMBIENT
    }

    fn update(&mut self, ctx: &Ctx, line: &mut Line) {
        line.color = ctx.signals.tint;
    }
}

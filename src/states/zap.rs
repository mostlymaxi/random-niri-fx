//! `random-fx zap [#color]`: instead of the next sweep, the collision race
//! (see `collision`). Following sweeps carry the zap's color, cooling back to
//! the time-of-day tint. It also fires on its own now and then, in the
//! line's own color.

use super::collision::{Collision, FLARE};
use super::{Ctx, Overlay, State};
use crate::color::Rgb;
use crate::ipc::Event;
use crate::line::Line;
use crate::rng::Rng;
use crate::signals::Signals;

/// Seconds for later sweeps to cool from the zap color back to the tint.
const COOL: f32 = 2.4;

/// What a zap is colored by.
#[derive(Clone, Copy)]
enum ZapColor {
    Fixed(Rgb),
    /// The line's own time-of-day color.
    Tint,
}

pub struct Zap {
    rng: Rng,
    /// Chance of a spontaneous zap at each decision point.
    chance: f32,
    default_color: Rgb,
    pending: Option<ZapColor>,
    color: ZapColor,
    race: Collision,
}

impl Zap {
    pub fn new(default_color: Rgb, chance: f32) -> Self {
        Self {
            rng: Rng::seeded(),
            chance,
            default_color,
            pending: None,
            color: ZapColor::Fixed(default_color),
            race: Collision::new(),
        }
    }

    fn color(&self, ctx: &Ctx) -> Rgb {
        match self.color {
            ZapColor::Fixed(c) => c,
            ZapColor::Tint => ctx.signals.tint,
        }
    }
}

impl State for Zap {
    fn name(&self) -> &'static str {
        "zap"
    }

    fn want(&self, _: &Signals) -> f32 {
        if self.pending.is_some() {
            1.0
        } else {
            0.0
        }
    }

    fn urge(&mut self, _: &Signals) -> bool {
        let fire = self.rng.f32() < self.chance;
        if fire {
            self.pending = Some(ZapColor::Tint);
        }
        fire
    }

    fn event(&mut self, event: &Event) -> bool {
        if event.verb != "zap" {
            return false;
        }
        let color = event.args.first().and_then(|a| Rgb::parse(a));
        self.pending = Some(ZapColor::Fixed(color.unwrap_or(self.default_color)));
        true
    }

    fn enter(&mut self) {
        self.color = self
            .pending
            .take()
            .unwrap_or(ZapColor::Fixed(self.default_color));
        self.race.reset();
    }

    fn update(&mut self, ctx: &Ctx, line: &mut Line) {
        if ctx.cycles == 0 {
            // The streaks replace this sweep; they draw themselves in `overlay`.
            line.alpha = 0.0;
            return;
        }
        let cooled = (ctx.age - Collision::impact(ctx) - FLARE) / COOL;
        line.color = self.color(ctx).lerp(ctx.signals.tint, cooled);
    }

    fn overlay(&mut self, ctx: &Ctx, out: &mut Overlay) {
        if ctx.cycles > 0 {
            return;
        }
        let color = self.color(ctx);
        self.race.draw(ctx, color, &mut out.edges);
    }
}

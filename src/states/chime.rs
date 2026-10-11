//! On the hour and half hour (or `random-fx chime`): the collision race,
//! then the energy runs on into the waybar clock pill, around its top and
//! bottom at once, and lights up the whole outline when the two meet.
//! Without a configured `clock`, it is just the race.

use super::collision::Collision;
use super::{Ctx, Overlay, State};
use crate::color::Rgb;
use crate::ease::CubicBezier;
use crate::ipc::Event;
use crate::line::Line;
use crate::motion::Comet;
use crate::pill::Mark;
use crate::signals::Signals;

/// Chimes per day.
const SLOTS: f32 = 48.0;
/// Seconds after impact before the traces set off into the pill.
const HANDOFF: f32 = 0.08;
/// Seconds for the traces to run from the right end to the left end.
const TRACE: f32 = 0.7;
const TRACE_EASE: CubicBezier = CubicBezier::new(0.65, 0.0, 0.35, 1.0);
const CHASE: f32 = 0.1;
/// Seconds the lit outline holds, then fades.
const HOLD: f32 = 0.4;
const FADE: f32 = 1.2;

pub struct Chime {
    race: Collision,
    /// Tails of the traces over the top and under the bottom.
    traces: [Comet; 2],
    pending: bool,
    playing: bool,
    /// The half hour we last saw, so we chime when it changes.
    slot: Option<u32>,
}

impl Chime {
    pub fn new() -> Self {
        Self {
            race: Collision::new(),
            traces: [Comet::new(CHASE), Comet::new(CHASE)],
            pending: false,
            playing: false,
            slot: None,
        }
    }

    fn color(ctx: &Ctx) -> Rgb {
        ctx.signals.tint.lerp(Rgb::WHITE, 0.2)
    }
}

impl State for Chime {
    fn name(&self) -> &'static str {
        "chime"
    }

    /// Holds on until the outline has faded, so the machine doesn't cut it off.
    fn want(&self, _: &Signals) -> f32 {
        if self.pending || self.playing {
            1.0
        } else {
            0.0
        }
    }

    fn urge(&mut self, signals: &Signals) -> bool {
        let slot = (signals.day * SLOTS) as u32;
        let changed = self.slot.is_some_and(|last| last != slot);
        self.slot = Some(slot);
        if changed {
            self.pending = true;
        }
        changed
    }

    fn event(&mut self, event: &Event) -> bool {
        self.pending |= event.verb == "chime";
        event.verb == "chime"
    }

    fn enter(&mut self) {
        self.pending = false;
        self.playing = true;
        self.race.reset();
        self.traces = [Comet::new(CHASE), Comet::new(CHASE)];
    }

    fn update(&mut self, ctx: &Ctx, line: &mut Line) {
        self.playing = ctx.age < Collision::impact(ctx) + HANDOFF + TRACE + HOLD + FADE;
        if ctx.cycles == 0 {
            // The race replaces this sweep.
            line.alpha = 0.0;
        } else {
            line.color = ctx.signals.tint;
        }
    }

    fn overlay(&mut self, ctx: &Ctx, out: &mut Overlay) {
        if !self.playing {
            return;
        }
        let color = Self::color(ctx);
        self.race.draw(ctx, color, &mut out.edges);
        if ctx.pill.is_none() {
            return;
        }

        let t = ctx.age - Collision::impact(ctx) - HANDOFF;
        if t < 0.0 {
            return;
        }
        let head = TRACE_EASE.at(t / TRACE);
        for (sign, comet) in [1.0, -1.0].into_iter().zip(&mut self.traces) {
            let (tail, head) = comet.follow(head, ctx.dt);
            if head - tail > 1e-3 {
                out.pill.push(Mark::Trace {
                    tail: sign * tail,
                    head: sign * head,
                    color,
                    alpha: 1.0,
                    width: 3.0,
                });
            }
        }

        let lit = t - TRACE;
        if lit > 0.0 {
            let fade = ((lit - HOLD) / FADE).clamp(0.0, 1.0);
            out.pill.push(Mark::Ring {
                color,
                alpha: (lit / 0.12).min(1.0) * (1.0 - fade * fade),
                width: 2.0,
            });
        }
    }
}

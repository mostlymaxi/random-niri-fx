//! States are the moods the machine chooses between. The line sweeps across
//! the screen once per cycle no matter what (see `motion`); a state decides
//! how each sweep looks. Exactly one state is active, and it only changes at a cycle
//! boundary, so transitions happen between sweeps instead of mid-flight.
//!
//! Adding one: write a file here implementing `State`, then list it in `all`.

mod chime;
mod collision;
mod heat;
mod sweep;
mod zap;

use crate::config::Config;
use crate::edge::Edge;
use crate::ipc::Event;
use crate::line::Line;
use crate::pill::{Mark, Pill};
use crate::signals::Signals;

pub struct Ctx<'a> {
    /// Seconds since the previous frame.
    pub dt: f32,
    /// Screen size in pixels.
    pub screen: (f32, f32),
    /// The clock pill, if configured.
    pub pill: Option<Pill>,
    /// The main sweep's top speed, in pixels per second.
    pub sweep_speed: f32,
    /// Seconds since this state was entered.
    pub age: f32,
    /// Full cycles completed since this state was entered.
    pub cycles: u32,
    pub signals: &'a Signals,
}

pub trait State {
    fn name(&self) -> &'static str;

    /// How much this state wants to play right now; the highest wins at each
    /// decision point. Zero means "not eligible". Ambient states return a small
    /// constant so they fill in when nothing else is going on.
    fn want(&self, signals: &Signals) -> f32;

    /// Speed multiplier for the next sweep (flight and rest alike).
    fn tempo(&self, _signals: &Signals) -> f32 {
        1.0
    }

    /// Called at the cycle boundary where this state becomes active.
    fn enter(&mut self) {}

    /// Shapes the line for this frame. The line arrives with default styling
    /// and its position for this point in the sweep already set.
    fn update(&mut self, ctx: &Ctx, line: &mut Line);

    /// Extra lines on any edge, drawn on top of the main one. Most states
    /// leave this alone; it is for the occasional bigger moment.
    fn overlay(&mut self, _ctx: &Ctx, _out: &mut Overlay) {}

    /// Checked at each decision point. Returning true makes this state play
    /// now, exactly as an event would (restarting it if already active).
    fn urge(&mut self, _signals: &Signals) -> bool {
        false
    }

    /// Handles an outside event. Returning true asks the machine to switch to
    /// this state at the next cycle boundary, even if the current state has
    /// not finished its dwell.
    fn event(&mut self, _event: &Event) -> bool {
        false
    }
}

/// What a state draws beyond the main line.
#[derive(Default)]
pub struct Overlay {
    /// Lines on any screen edge.
    pub edges: Vec<(Edge, Line)>,
    /// Marks on the clock pill.
    pub pill: Vec<Mark>,
}

impl Overlay {
    pub fn clear(&mut self) {
        self.edges.clear();
        self.pill.clear();
    }
}

/// Want for always-available background states.
pub const AMBIENT: f32 = 0.1;

pub fn all(config: &Config) -> Vec<Box<dyn State>> {
    vec![
        Box::new(sweep::Sweep),
        Box::new(heat::Heat::new(config.hot)),
        Box::new(zap::Zap::new(config.zap, config.zap_chance)),
        Box::new(chime::Chime::new()),
    ]
}

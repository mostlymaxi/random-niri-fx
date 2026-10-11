//! The state machine. Keeps the sweep rhythm and decides, at cycle
//! boundaries only, which state shapes the line.

use std::sync::mpsc::Receiver;

use crate::config::Config;
use crate::edge::Edge;
use crate::ipc::Event;
use crate::line::Line;
use crate::motion::{Comet, CHASE};
use crate::pill::Mark;
use crate::rng::Rng;
use crate::signals::{Signals, Sources};
use crate::states::{Ctx, Overlay, State};

pub struct Machine {
    states: Vec<Box<dyn State>>,
    config: Config,
    sources: Sources,
    signals: Signals,
    events: Receiver<Event>,
    line: Line,
    overlay: Overlay,
    comet: Comet,

    rng: Rng,
    current: usize,
    entered_at: f32,
    /// How long the current state plays, rolled from the config range on entry.
    dwell: f32,
    cycles: u32,
    cycle_start: f32,
    /// Lengths of the current sweep's flight and whole cycle, in seconds.
    flight_len: f32,
    cycle_len: f32,
    /// A state that asked, via an event, to play at the next boundary.
    interrupt: Option<usize>,
    last_t: f32,
}

impl Machine {
    pub fn new(
        states: Vec<Box<dyn State>>,
        config: Config,
        sources: Sources,
        events: Receiver<Event>,
    ) -> Self {
        assert!(!states.is_empty(), "no states enabled");
        let (flight_len, cycle_len) = (config.flight, config.flight + config.rest);
        let dwell = config.dwell.0;
        Self {
            states,
            config,
            sources,
            signals: Signals::default(),
            events,
            line: Line::new(0.0),
            overlay: Overlay::default(),
            comet: Comet::new(CHASE),
            rng: Rng::seeded(),
            current: 0,
            entered_at: 0.0,
            dwell,
            cycles: 0,
            cycle_start: 0.0,
            flight_len,
            cycle_len,
            interrupt: None,
            last_t: 0.0,
        }
    }

    /// Advances to time `t` (seconds since startup) on a screen of `screen`
    /// pixels, and returns what to draw.
    pub fn frame(&mut self, t: f32, screen: (f32, f32)) -> Frame<'_> {
        let dt = (t - self.last_t).clamp(0.0, 0.1);
        self.last_t = t;
        self.sources.sample(&mut self.signals, dt);

        while let Ok(event) = self.events.try_recv() {
            for (i, state) in self.states.iter_mut().enumerate() {
                if state.event(&event) {
                    self.interrupt = Some(i);
                }
            }
        }

        if t - self.cycle_start >= self.cycle_len {
            self.next_cycle(t);
        }

        let ctx = Ctx {
            dt,
            screen,
            pill: self.config.clock,
            age: t - self.entered_at,
            cycles: self.cycles,
            signals: &self.signals,
        };
        self.line.reset();
        self.line.resize(screen.0);
        (self.line.tail, self.line.head) =
            self.comet
                .advance(t - self.cycle_start, self.flight_len, dt);
        let state = &mut self.states[self.current];
        state.update(&ctx, &mut self.line);
        self.overlay.clear();
        state.overlay(&ctx, &mut self.overlay);

        Frame {
            main: &self.line,
            edges: &self.overlay.edges,
            pill: &self.overlay.pill,
        }
    }

    fn next_cycle(&mut self, t: f32) {
        // After a stall (suspend, hidden output) restart the rhythm from now
        // instead of fast-forwarding through missed cycles.
        self.cycle_start = if t - self.cycle_start < 2.0 * self.cycle_len {
            self.cycle_start + self.cycle_len
        } else {
            t
        };
        self.cycles += 1;
        self.comet.restart();

        if let Some(i) = self.interrupt.take() {
            // Always (re)enter: a second zap should strike again.
            self.enter(i);
        } else if self.cycle_start - self.entered_at >= self.dwell - 1e-3 {
            let signals = &self.signals;
            if let Some(i) = self.states.iter_mut().position(|s| s.urge(signals)) {
                self.enter(i);
            } else {
                let best = self.pick();
                if best != self.current {
                    self.enter(best);
                }
            }
        }
        let tempo = self.states[self.current].tempo(&self.signals);
        self.flight_len = self.config.flight / tempo;
        self.cycle_len = (self.config.flight + self.config.rest) / tempo;
    }

    fn enter(&mut self, i: usize) {
        if i != self.current {
            eprintln!(
                "random-fx: {} -> {}",
                self.states[self.current].name(),
                self.states[i].name()
            );
        }
        self.current = i;
        self.entered_at = self.cycle_start;
        self.dwell = self.rng.range(self.config.dwell.0, self.config.dwell.1);
        self.cycles = 0;
        self.states[i].enter();
    }

    /// The most wanted state. Ties go to the current state, so an ambient
    /// state isn't needlessly restarted.
    fn pick(&self) -> usize {
        let wants: Vec<f32> = self.states.iter().map(|s| s.want(&self.signals)).collect();
        (0..wants.len()).fold(
            self.current,
            |best, i| {
                if wants[i] > wants[best] {
                    i
                } else {
                    best
                }
            },
        )
    }
}

/// Everything to draw this frame.
pub struct Frame<'a> {
    /// The sweep, along the top edge.
    pub main: &'a Line,
    /// Extra lines from the active state, on any edge.
    pub edges: &'a [(Edge, Line)],
    /// Marks on the clock pill.
    pub pill: &'a [Mark],
}

impl Frame<'_> {
    pub fn lines(&self) -> impl Iterator<Item = (Edge, &Line)> {
        std::iter::once((Edge::Top, self.main)).chain(self.edges.iter().map(|(e, l)| (*e, l)))
    }
}

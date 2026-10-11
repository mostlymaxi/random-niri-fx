//! Background data sources. Each runs on its own thread and publishes into
//! shared state; `Sources::sample` turns that into a smoothed per-frame
//! snapshot that states read.

mod clock;
mod cpu;

use crate::color::Rgb;
use crate::ease::approach;
use crate::palette::Palette;

/// What the outside world looks like this frame.
#[derive(Clone, Debug, Default)]
pub struct Signals {
    /// Total CPU usage, 0..1.
    pub cpu: f32,
    /// Fraction of the local day elapsed, 0..1.
    pub day: f32,
    /// Base color for the current time of day.
    pub tint: Rgb,
}

pub struct Sources {
    cpu: cpu::Cpu,
    clock: clock::Clock,
    palette: Palette,
}

impl Sources {
    pub fn spawn(palette: Palette) -> Self {
        Self {
            cpu: cpu::Cpu::spawn(),
            clock: clock::Clock::spawn(),
            palette,
        }
    }

    pub fn sample(&self, s: &mut Signals, dt: f32) {
        s.cpu = approach(s.cpu, self.cpu.usage(), 0.8, dt);
        s.day = self.clock.day();
        s.tint = self.palette.at(s.day);
    }
}

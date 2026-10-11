//! How the line moves through one cycle, in the spirit of kitty's
//! `cursor_trail`: the head flies across on a bezier curve and the tail
//! chases it with exponential decay, so the line stretches out while the
//! head is fast and snaps shut as it slows down.

use std::sync::OnceLock;

use crate::ease::{approach, CubicBezier};

const HEAD: CubicBezier = CubicBezier::new(0.65, 0.0, 0.35, 1.0);
/// Time constant of the tail's chase, in seconds. Higher = longer trail.
pub const CHASE: f32 = 0.2;
/// Where the head starts, just off the left edge.
const ENTRY: f32 = -0.03;
/// Where the head stops, far enough past the right edge that it is still
/// moving fast as it leaves the screen instead of easing to a halt on it.
const EXIT: f32 = 1.3;

/// Head position as a fraction of the edge, `elapsed` seconds into a flight
/// lasting `flight` seconds.
pub fn head(elapsed: f32, flight: f32) -> f32 {
    ENTRY + HEAD.at(elapsed / flight) * (EXIT - ENTRY)
}

/// Distance in pixels the head covers `elapsed` seconds into a flight of
/// `flight` seconds on an edge `length` pixels long, except that it never
/// eases off: past the curve's midpoint it holds its top speed.
pub fn capped(elapsed: f32, length: f32, flight: f32) -> f32 {
    let (half_time, half_way) = (flight / 2.0, (EXIT - ENTRY) * length / 2.0);
    if elapsed < half_time {
        (EXIT - ENTRY) * length * HEAD.at(elapsed / flight)
    } else {
        half_way + peak_speed(length, flight) * (elapsed - half_time)
    }
}

/// Seconds until `capped` has covered `distance` pixels.
pub fn capped_time(distance: f32, length: f32, flight: f32) -> f32 {
    let (mut lo, mut hi) = (0.0, flight / 2.0);
    let half_way = (EXIT - ENTRY) * length / 2.0;
    if distance > half_way {
        return hi + (distance - half_way) / peak_speed(length, flight);
    }
    // Still on the curve: solve for it.
    for _ in 0..30 {
        let mid = (lo + hi) / 2.0;
        if capped(mid, length, flight) < distance {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

/// The head curve's steepest slope, at its middle: its top speed relative
/// to its average speed.
fn peak_slope() -> f32 {
    static PEAK: OnceLock<f32> = OnceLock::new();
    *PEAK.get_or_init(|| {
        const N: usize = 400;
        (0..N)
            .map(|i| (HEAD.at((i + 1) as f32 / N as f32) - HEAD.at(i as f32 / N as f32)) * N as f32)
            .fold(0.0, f32::max)
    })
}

/// The head's top speed in pixels per second, on an edge `length` pixels
/// long with a flight of `flight` seconds.
pub fn peak_speed(length: f32, flight: f32) -> f32 {
    peak_slope() * (EXIT - ENTRY) * length / flight
}

pub struct Comet {
    tail: f32,
    chase: f32,
}

impl Comet {
    /// A comet whose tail chases its head with time constant `chase` seconds.
    pub fn new(chase: f32) -> Self {
        Self { tail: ENTRY, chase }
    }

    /// Starts a new flight from the left edge.
    pub fn restart(&mut self) {
        self.tail = ENTRY;
    }

    /// Moves the head to `head` and lets the tail chase it.
    pub fn follow(&mut self, head: f32, dt: f32) -> (f32, f32) {
        self.tail = approach(self.tail, head, self.chase, dt);
        (self.tail, head)
    }

    /// `(tail, head)` as edge fractions, `elapsed` seconds into a flight
    /// lasting `flight` seconds. Past the flight the head waits off-screen
    /// while the tail catches up.
    pub fn advance(&mut self, elapsed: f32, flight: f32, dt: f32) -> (f32, f32) {
        self.follow(head(elapsed, flight), dt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trail_stretches_then_leaves() {
        let mut comet = Comet::new(CHASE);
        let dt = 1.0 / 60.0;
        let (flight, rest) = (2.2, 1.4);
        let mut longest: f32 = 0.0;
        let mut last = (0.0, 0.0);
        for i in 0..=((flight + rest) / dt) as usize {
            last = comet.advance(i as f32 * dt, flight, dt);
            longest = longest.max(last.1 - last.0);
        }
        assert!(longest > 0.1, "trail should stretch, got {longest}");
        assert!(
            last.0 > 1.05,
            "tail should leave the screen, got {}",
            last.0
        );
    }

    #[test]
    fn capped_follows_the_curve_then_holds_top_speed() {
        let (length, flight) = (2560.0, 2.2);
        let at = |t: f32| capped(t, length, flight);
        let speed = |t: f32| (at(t + 0.001) - at(t)) / 0.001;
        // Same as the real head while accelerating.
        assert!((at(0.5) - (head(0.5, flight) - ENTRY) * length).abs() < 0.5);
        // Then flat out at the top speed instead of easing off.
        let top = peak_speed(length, flight);
        assert!((speed(1.5) - top).abs() < 5.0 && (speed(2.5) - top).abs() < 5.0);
        for d in [500.0, 4000.0] {
            assert!((at(capped_time(d, length, flight)) - d).abs() < 0.5);
        }
    }
}

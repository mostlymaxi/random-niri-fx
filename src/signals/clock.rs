//! Local time of day, read from `date` once a minute so we don't need a
//! timezone database.

use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct Clock(Arc<Mutex<(f32, Instant)>>);

impl Clock {
    pub fn spawn() -> Self {
        let anchor = Arc::new(Mutex::new((read_day().unwrap_or(0.5), Instant::now())));
        let out = anchor.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(60));
            if let Some(day) = read_day() {
                *out.lock().unwrap() = (day, Instant::now());
            }
        });
        Self(anchor)
    }

    /// Fraction of the local day elapsed, 0..1.
    pub fn day(&self) -> f32 {
        let (day, at) = *self.0.lock().unwrap();
        (day + at.elapsed().as_secs_f32() / 86_400.0).fract()
    }
}

fn read_day() -> Option<f32> {
    let out = Command::new("date").arg("+%H %M %S").output().ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    let mut parts = text.split_whitespace().map(|p| p.parse::<f32>().ok());
    let (h, m, s) = (parts.next()??, parts.next()??, parts.next()??);
    Some((h * 3600.0 + m * 60.0 + s) / 86_400.0)
}

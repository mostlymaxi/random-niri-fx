use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

const INTERVAL: Duration = Duration::from_millis(750);

pub struct Cpu(Arc<AtomicU32>);

impl Cpu {
    pub fn spawn() -> Self {
        let usage = Arc::new(AtomicU32::new(0));
        let out = usage.clone();
        std::thread::spawn(move || {
            let mut prev = read_stat();
            loop {
                std::thread::sleep(INTERVAL);
                let next = read_stat();
                if let (Some((busy0, total0)), Some((busy1, total1))) = (prev, next) {
                    let total = total1.saturating_sub(total0).max(1);
                    let frac = busy1.saturating_sub(busy0) as f32 / total as f32;
                    out.store(frac.to_bits(), Ordering::Relaxed);
                }
                prev = next;
            }
        });
        Self(usage)
    }

    pub fn usage(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }
}

/// `(busy, total)` jiffies from the aggregate line of `/proc/stat`.
fn read_stat() -> Option<(u64, u64)> {
    let text = std::fs::read_to_string("/proc/stat").ok()?;
    parse_stat(text.lines().next()?)
}

fn parse_stat(line: &str) -> Option<(u64, u64)> {
    let fields: Vec<u64> = line
        .strip_prefix("cpu ")?
        .split_whitespace()
        .filter_map(|f| f.parse().ok())
        .collect();
    let total = fields.iter().take(8).sum();
    let idle = fields.get(3)? + fields.get(4).unwrap_or(&0);
    Some((total - idle, total))
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_aggregate_line() {
        let (busy, total) = super::parse_stat("cpu  10 0 10 70 10 0 0 0 0 0").unwrap();
        assert_eq!((busy, total), (20, 100));
    }
}

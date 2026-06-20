use crate::platform::clock::Clock;
use std::sync::OnceLock;

pub struct StdClock;

fn monotonic_base() -> &'static std::time::Instant {
    static BASE: OnceLock<std::time::Instant> = OnceLock::new();
    BASE.get_or_init(std::time::Instant::now)
}

impl Clock for StdClock {
    fn now_ms(&self) -> u32 {
        std::time::Instant::now()
            .duration_since(*monotonic_base())
            .as_millis() as u32
    }
}

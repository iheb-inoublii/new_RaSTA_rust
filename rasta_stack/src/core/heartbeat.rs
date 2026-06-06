// Heartbeat logic
// This helps us know when to send a "Ping" to the other side.

use crate::platform::timer::Timer;

pub struct HeartbeatHandler<T: Timer> {
    pub timer: T,
    pub interval_ms: u32,
}

impl<T: Timer> HeartbeatHandler<T> {
    pub fn new(timer: T, interval_ms: u32) -> Self {
        HeartbeatHandler { timer, interval_ms }
    }

    pub fn reset(&mut self) {
        self.timer.stop();
        self.timer.start(self.interval_ms);
    }

    pub fn is_due(&self) -> bool {
        self.timer.expired()
    }
}

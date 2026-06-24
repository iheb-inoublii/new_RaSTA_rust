use rasta_core::time::{DurationMs, MonotonicInstant};

pub struct HeartbeatHandler {
    deadline: Option<MonotonicInstant>,
    interval: DurationMs,
}

impl HeartbeatHandler {
    pub fn new(interval: DurationMs) -> Self {
        Self {
            deadline: None,
            interval,
        }
    }

    pub fn restart(&mut self, now: MonotonicInstant) {
        self.deadline = Some(now.deadline_after(self.interval));
    }

    pub fn stop(&mut self) {
        self.deadline = None;
    }

    pub fn is_due(&self, now: MonotonicInstant) -> bool {
        self.deadline
            .is_some_and(|deadline| now.has_reached(deadline))
    }
}

#[cfg(test)]
mod tests {
    use super::HeartbeatHandler;
    use rasta_core::time::{DurationMs, MonotonicInstant};

    #[test]
    fn restart_stop_and_wraparound_are_deterministic() {
        let mut heartbeat = HeartbeatHandler::new(DurationMs::from_millis(10));
        assert!(!heartbeat.is_due(MonotonicInstant::from_wrapping_millis(0)));
        heartbeat.restart(MonotonicInstant::from_wrapping_millis(u32::MAX - 5));
        assert!(!heartbeat.is_due(MonotonicInstant::from_wrapping_millis(3)));
        assert!(heartbeat.is_due(MonotonicInstant::from_wrapping_millis(4)));
        heartbeat.stop();
        assert!(!heartbeat.is_due(MonotonicInstant::from_wrapping_millis(4)));
    }
}

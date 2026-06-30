use std::time::{Instant, SystemTime, UNIX_EPOCH};

use rasta_core::time::{
    MonotonicClock, MonotonicInstant, ProtocolTimestamp, ProtocolTimestampSource,
};

pub struct StdClock {
    origin: Instant,
    protocol_origin_millis: u32,
}

impl StdClock {
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
            protocol_origin_millis: current_epoch_wrapping_millis(),
        }
    }

    fn elapsed_wrapping_millis(&self) -> u32 {
        self.origin.elapsed().as_millis() as u32
    }

    fn protocol_wrapping_millis(&self) -> u32 {
        self.protocol_origin_millis
            .wrapping_add(self.elapsed_wrapping_millis())
    }

    #[cfg(test)]
    fn with_protocol_origin_millis(protocol_origin_millis: u32) -> Self {
        Self {
            origin: Instant::now(),
            protocol_origin_millis,
        }
    }
}

fn current_epoch_wrapping_millis() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u32)
        .unwrap_or(0)
}

impl Default for StdClock {
    fn default() -> Self {
        Self::new()
    }
}

impl MonotonicClock for StdClock {
    fn now(&self) -> MonotonicInstant {
        MonotonicInstant::from_wrapping_millis(self.elapsed_wrapping_millis())
    }
}

impl ProtocolTimestampSource for StdClock {
    fn protocol_timestamp(&self) -> ProtocolTimestamp {
        ProtocolTimestamp::from_wire_millis(self.protocol_wrapping_millis())
    }
}

#[cfg(test)]
mod tests {
    use super::StdClock;
    use rasta_core::time::{MonotonicClock, ProtocolTimestampSource};

    #[test]
    fn immediate_monotonic_reads_do_not_move_backwards() {
        let clock = StdClock::new();
        let first = clock.now();
        let second = clock.now();
        assert!(second.elapsed_since(first).as_millis() < 0x8000_0000);
    }

    #[test]
    fn protocol_timestamp_uses_shared_epoch_not_instance_zero() {
        let first = StdClock::with_protocol_origin_millis(1_000);
        let second = StdClock::with_protocol_origin_millis(1_000);
        let first_timestamp = first.protocol_timestamp();
        let second_timestamp = second.protocol_timestamp();
        assert!(
            first_timestamp
                .wrapping_elapsed_since(second_timestamp)
                .as_millis()
                < 0x8000_0000
                || second_timestamp
                    .wrapping_elapsed_since(first_timestamp)
                    .as_millis()
                    < 0x8000_0000
        );
        assert!(first_timestamp.wire_millis() >= 1_000);
        assert!(second_timestamp.wire_millis() >= 1_000);
    }

    #[test]
    fn monotonic_and_protocol_time_have_separate_origins() {
        let clock = StdClock::with_protocol_origin_millis(123_456);
        assert!(clock.now().wrapping_millis() < 1_000);
        assert!(clock.protocol_timestamp().wire_millis() >= 123_456);
    }
}

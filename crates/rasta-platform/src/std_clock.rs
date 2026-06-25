use std::time::Instant;

use rasta_core::time::{
    MonotonicClock, MonotonicInstant, ProtocolTimestamp, ProtocolTimestampSource,
};

pub struct StdClock {
    origin: Instant,
}

impl StdClock {
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }

    fn elapsed_wrapping_millis(&self) -> u32 {
        self.origin.elapsed().as_millis() as u32
    }
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
        ProtocolTimestamp::from_wire_millis(self.elapsed_wrapping_millis())
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
    fn protocol_timestamp_is_explicit_and_instances_are_independent() {
        let first = StdClock::new();
        let second = StdClock::new();
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
    }
}

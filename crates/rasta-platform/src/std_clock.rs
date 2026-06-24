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

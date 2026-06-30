// Time / timestamp supervision for platform-independent RaSTA core logic.

use crate::time::{DurationMs, ProtocolTimestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeSupervisionError {
    TimestampTooOld,
    TimestampTooFarInFuture,
    ConfirmedTimestampMovedBackwards,
    ConfirmedTimestampTooFarInFuture,
}

#[derive(Debug, Clone, Copy)]
pub struct TimeSupervisor {
    pub t_max: DurationMs,
    pub future_tolerance: DurationMs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmedTimestampDecision {
    pub confirmed_timestamp: ProtocolTimestamp,
    pub round_trip: DurationMs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteTimestampDecision {
    pub normalized_timestamp: ProtocolTimestamp,
}

impl TimeSupervisor {
    pub const DEFAULT_FUTURE_TOLERANCE_MS: u32 = 100;

    pub fn new(t_max_ms: u32) -> Self {
        Self {
            t_max: DurationMs::from_millis(t_max_ms),
            future_tolerance: DurationMs::from_millis(Self::DEFAULT_FUTURE_TOLERANCE_MS),
        }
    }

    pub fn validate(
        &self,
        local_timestamp: ProtocolTimestamp,
        remote_timestamp: ProtocolTimestamp,
    ) -> Result<(), TimeSupervisionError> {
        let age = local_timestamp.wrapping_elapsed_since(remote_timestamp);

        if age.as_millis() < 0x8000_0000 {
            if age.as_millis() > self.t_max.as_millis() {
                return Err(TimeSupervisionError::TimestampTooOld);
            }
        } else {
            let future_offset = remote_timestamp.wrapping_elapsed_since(local_timestamp);
            if future_offset.as_millis() > self.future_tolerance.as_millis() {
                return Err(TimeSupervisionError::TimestampTooFarInFuture);
            }
        }

        Ok(())
    }

    pub fn validate_peer_relative(
        &self,
        local_timestamp: ProtocolTimestamp,
        remote_timestamp: ProtocolTimestamp,
        peer_to_local_offset: DurationMs,
    ) -> Result<RemoteTimestampDecision, TimeSupervisionError> {
        let normalized_timestamp = ProtocolTimestamp::from_wire_millis(
            remote_timestamp
                .wire_millis()
                .wrapping_add(peer_to_local_offset.as_millis()),
        );
        self.validate(local_timestamp, normalized_timestamp)?;
        Ok(RemoteTimestampDecision {
            normalized_timestamp,
        })
    }

    pub fn validate_confirmed_timestamp(
        &self,
        local_timestamp: ProtocolTimestamp,
        reference: ProtocolTimestamp,
        confirmed_timestamp: ProtocolTimestamp,
    ) -> Result<ConfirmedTimestampDecision, TimeSupervisionError> {
        if confirmed_timestamp.is_after(local_timestamp) {
            return Err(TimeSupervisionError::ConfirmedTimestampTooFarInFuture);
        }

        if confirmed_timestamp != reference && !confirmed_timestamp.is_after(reference) {
            return Err(TimeSupervisionError::ConfirmedTimestampMovedBackwards);
        }

        let confirmed_distance = confirmed_timestamp.wrapping_elapsed_since(reference);
        if confirmed_distance.as_millis() >= self.t_max.as_millis() {
            return Err(TimeSupervisionError::ConfirmedTimestampTooFarInFuture);
        }

        let round_trip = local_timestamp.wrapping_elapsed_since(confirmed_timestamp);
        if round_trip.as_millis() > self.t_max.as_millis() {
            return Err(TimeSupervisionError::TimestampTooOld);
        }

        Ok(ConfirmedTimestampDecision {
            confirmed_timestamp,
            round_trip,
        })
    }
}

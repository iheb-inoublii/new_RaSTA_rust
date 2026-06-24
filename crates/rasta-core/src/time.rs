//! Wrapping monotonic time for internal timeout supervision.
//!
//! `MonotonicInstant` has no wall-clock or wire-format meaning. Timeout
//! decisions are unambiguous only when compared durations are below
//! `u32::MAX / 2`; the exact half-range is deliberately treated as not reached.

const HALF_RANGE: u32 = 0x8000_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DurationMs(u32);

impl DurationMs {
    pub const fn from_millis(value: u32) -> Self {
        Self(value)
    }

    pub const fn as_millis(self) -> u32 {
        self.0
    }

    pub const fn is_unambiguous_timeout(self) -> bool {
        self.0 < HALF_RANGE
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonotonicInstant(u32);

impl MonotonicInstant {
    pub const fn from_wrapping_millis(value: u32) -> Self {
        Self(value)
    }

    pub const fn wrapping_millis(self) -> u32 {
        self.0
    }

    pub fn elapsed_since(self, earlier: Self) -> DurationMs {
        DurationMs(self.0.wrapping_sub(earlier.0))
    }

    pub fn deadline_after(self, duration: DurationMs) -> Self {
        Self(self.0.wrapping_add(duration.0))
    }

    pub fn has_reached(self, deadline: Self) -> bool {
        self.0 == deadline.0 || self.0.wrapping_sub(deadline.0) < HALF_RANGE
    }
}

/// Internal monotonic-time source. It is intentionally separate from RaSTA
/// protocol timestamps.
pub trait MonotonicClock {
    fn now(&self) -> MonotonicInstant;
}

/// A timestamp value carried by a RaSTA packet, distinct from an internal
/// monotonic instant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProtocolTimestamp(u32);

impl ProtocolTimestamp {
    pub const fn from_wire_millis(value: u32) -> Self {
        Self(value)
    }

    pub const fn wire_millis(self) -> u32 {
        self.0
    }

    pub fn wrapping_elapsed_since(self, earlier: Self) -> DurationMs {
        DurationMs(self.0.wrapping_sub(earlier.0))
    }

    pub fn is_after(self, reference: Self) -> bool {
        self != reference && self.0.wrapping_sub(reference.0) < HALF_RANGE
    }
}

/// Source of RaSTA packet timestamp values, deliberately separate from the
/// internal monotonic instant used for deadlines.
pub trait ProtocolTimestampSource {
    fn protocol_timestamp(&self) -> ProtocolTimestamp;
}

#[cfg(test)]
mod tests {
    use super::{DurationMs, MonotonicInstant, ProtocolTimestamp};

    #[test]
    fn calculates_normal_elapsed_time() {
        let earlier = MonotonicInstant::from_wrapping_millis(100);
        let later = MonotonicInstant::from_wrapping_millis(250);
        assert_eq!(later.elapsed_since(earlier), DurationMs::from_millis(150));
    }

    #[test]
    fn calculates_elapsed_time_across_wrap() {
        let earlier = MonotonicInstant::from_wrapping_millis(u32::MAX - 4);
        let later = MonotonicInstant::from_wrapping_millis(3);
        assert_eq!(later.elapsed_since(earlier), DurationMs::from_millis(8));
    }

    #[test]
    fn calculates_deadlines_before_and_across_wrap() {
        assert_eq!(
            MonotonicInstant::from_wrapping_millis(10)
                .deadline_after(DurationMs::from_millis(5))
                .wrapping_millis(),
            15
        );
        assert_eq!(
            MonotonicInstant::from_wrapping_millis(u32::MAX - 2)
                .deadline_after(DurationMs::from_millis(5))
                .wrapping_millis(),
            2
        );
    }

    #[test]
    fn distinguishes_reached_and_not_reached_deadlines() {
        let deadline = MonotonicInstant::from_wrapping_millis(100);
        assert!(!MonotonicInstant::from_wrapping_millis(99).has_reached(deadline));
        assert!(MonotonicInstant::from_wrapping_millis(100).has_reached(deadline));
        assert!(MonotonicInstant::from_wrapping_millis(101).has_reached(deadline));
        assert!(
            MonotonicInstant::from_wrapping_millis(1)
                .has_reached(MonotonicInstant::from_wrapping_millis(u32::MAX))
        );
    }

    #[test]
    fn treats_the_half_range_boundary_as_ambiguous() {
        assert!(
            !MonotonicInstant::from_wrapping_millis(0)
                .has_reached(MonotonicInstant::from_wrapping_millis(0x8000_0000))
        );
    }

    #[test]
    fn protocol_timestamps_compare_across_wrap() {
        let before_wrap = ProtocolTimestamp::from_wire_millis(u32::MAX);
        let after_wrap = ProtocolTimestamp::from_wire_millis(1);
        assert_eq!(
            after_wrap.wrapping_elapsed_since(before_wrap),
            DurationMs::from_millis(2)
        );
        assert!(after_wrap.is_after(before_wrap));
    }
}

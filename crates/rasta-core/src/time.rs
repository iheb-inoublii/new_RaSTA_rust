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

#[cfg(test)]
mod tests {
    use super::{DurationMs, MonotonicInstant};

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
}

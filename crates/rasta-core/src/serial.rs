//! RFC-style 32-bit serial arithmetic used by DIN RaSTA sequence fields.

pub const HALF_RANGE: u32 = 0x8000_0000;

pub fn is_after(candidate: u32, reference: u32) -> bool {
    candidate != reference && candidate.wrapping_sub(reference) < HALF_RANGE
}

pub fn is_before(candidate: u32, reference: u32) -> bool {
    is_after(reference, candidate)
}

pub fn forward_distance(from: u32, to: u32) -> u32 {
    to.wrapping_sub(from)
}

pub fn is_in_forward_window(value: u32, start: u32, width: u32) -> bool {
    forward_distance(start, value) <= width
}

#[cfg(test)]
mod tests {
    use super::{HALF_RANGE, forward_distance, is_after, is_before, is_in_forward_window};

    #[test]
    fn handles_equal_and_ordinary_values() {
        assert!(!is_after(10, 10));
        assert!(!is_before(10, 10));
        assert!(is_after(11, 10));
        assert!(is_before(10, 11));
    }

    #[test]
    fn handles_wraparound_boundaries() {
        assert!(is_after(0, u32::MAX));
        assert!(is_before(u32::MAX, 0));
        assert!(is_after(u32::MAX, u32::MAX - 1));
        assert!(is_before(0, 1));
        assert_eq!(forward_distance(u32::MAX, 1), 2);
        assert!(is_in_forward_window(1, u32::MAX, 2));
    }

    #[test]
    fn treats_half_range_as_ambiguous() {
        assert!(!is_after(HALF_RANGE, 0));
        assert!(!is_after(0, HALF_RANGE));
    }
}

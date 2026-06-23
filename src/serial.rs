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

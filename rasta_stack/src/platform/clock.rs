// Clock trait for getting monotonic time without depending on std::time::Instant
pub trait Clock {
    fn now_ms(&self) -> u32;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RandomError {
    Unavailable,
    Failed,
}

/// Source of initial SRL sequence numbers. Production adapters must provide a
/// suitable source; deterministic implementations are for tests only.
pub trait RandomSource {
    fn next_u32(&mut self) -> Result<u32, RandomError>;
}

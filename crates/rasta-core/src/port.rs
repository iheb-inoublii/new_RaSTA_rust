#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportError {
    SendFailed,
    ReceiveFailed,
    BufferTooSmall,
    InvalidFrame,
    SequenceViolation,
}

/// Platform-neutral byte transport used by the protocol core.
pub trait Transport {
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError>;

    /// Receives into `buffer`, returning the number of bytes read or zero when
    /// no frame is currently available.
    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RandomError {
    Unavailable,
    Failed,
}

/// Source of initial SRL sequence numbers.
pub trait RandomSource {
    fn next_u32(&mut self) -> Result<u32, RandomError>;
}

// This is the Transport trait from the architecture document.
// It lets us abstract away the OS specific socket stuff so the core is portable.

#[derive(Debug, PartialEq)]
pub enum TransportError {
    SendFailed,
    ReceiveFailed,
    BufferTooSmall,
}

pub trait Transport {
    // Sends data using the transport.
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError>;

    // Receives data into the provided buffer. Returns how many bytes were read.
    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError>;
}

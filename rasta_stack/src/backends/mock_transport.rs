use crate::platform::transport::{Transport, TransportError};

pub struct MockTransport {
    pub last_sent: [u8; 512],
    pub last_sent_len: usize,
}

impl Default for MockTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl MockTransport {
    pub fn new() -> Self {
        MockTransport {
            last_sent: [0; 512],
            last_sent_len: 0,
        }
    }
}

impl Transport for MockTransport {
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        if data.len() > 512 {
            return Err(TransportError::BufferTooSmall);
        }
        if let Some(dest) = self.last_sent.get_mut(0..data.len()) {
            dest.copy_from_slice(data);
            self.last_sent_len = data.len();
        }
        Ok(())
    }

    fn receive(&mut self, _buffer: &mut [u8]) -> Result<usize, TransportError> {
        // Mocking no data received
        Err(TransportError::ReceiveFailed)
    }
}

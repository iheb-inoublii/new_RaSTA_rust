//! Platform-neutral extension points used by the RaSTA protocol core.
//!
//! `rasta-core` owns only the transport contract. OS UDP sockets, raw sockets,
//! embedded Ethernet drivers, and test doubles live outside the protocol core
//! and implement these traits without requiring heap allocation or `std`.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportError {
    SendFailed,
    ReceiveFailed,
    BufferTooSmall,
    InvalidFrame,
    SequenceViolation,
}

/// Public RaSTA frame transport contract.
///
/// A transport sends and receives one complete redundancy-layer frame at a
/// time. Implementations must copy received frame bytes into the caller-owned
/// `buffer` and return `Ok(0)` when no frame is currently available. Returning
/// `Ok(0)` is the nonblocking/no-data path and is not treated as a fatal
/// transport failure by the protocol core.
///
/// The trait intentionally has no `std`, heap allocation, socket type, or async
/// requirement. A platform crate can implement it for OS UDP, raw sockets, an
/// embedded Ethernet driver, or a deterministic mock transport.
pub trait RastaTransport {
    /// Sends one complete redundancy-layer frame.
    fn send(&mut self, frame: &[u8]) -> Result<(), TransportError>;

    /// Receives one complete redundancy-layer frame into `buffer`.
    ///
    /// Returns the frame length or `0` when no frame is available.
    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError>;
}

/// Backwards-compatible name for the platform-neutral RaSTA transport.
///
/// New code may use `RastaTransport` in public APIs. Existing code that imports
/// `Transport` remains valid because every `Transport` implementer also
/// implements `RastaTransport`.
pub trait Transport {
    fn send(&mut self, frame: &[u8]) -> Result<(), TransportError>;

    /// Receives into `buffer`, returning the number of bytes read or zero when no frame is available.
    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError>;
}

impl<T: Transport + ?Sized> RastaTransport for T {
    fn send(&mut self, frame: &[u8]) -> Result<(), TransportError> {
        Transport::send(self, frame)
    }

    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError> {
        Transport::receive(self, buffer)
    }
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

#[cfg(test)]
mod tests {
    use super::{RastaTransport, Transport, TransportError};

    struct MockTransport {
        received: [u8; 8],
        received_len: usize,
        sent: [u8; 8],
        sent_len: usize,
    }

    impl MockTransport {
        fn with_receive(data: &[u8]) -> Self {
            let mut received = [0u8; 8];
            received[..data.len()].copy_from_slice(data);
            Self {
                received,
                received_len: data.len(),
                sent: [0; 8],
                sent_len: 0,
            }
        }
    }

    impl Transport for MockTransport {
        fn send(&mut self, frame: &[u8]) -> Result<(), TransportError> {
            if frame.len() > self.sent.len() {
                return Err(TransportError::BufferTooSmall);
            }
            self.sent[..frame.len()].copy_from_slice(frame);
            self.sent_len = frame.len();
            Ok(())
        }

        fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError> {
            if self.received_len == 0 {
                return Ok(0);
            }
            if buffer.len() < self.received_len {
                return Err(TransportError::BufferTooSmall);
            }
            buffer[..self.received_len].copy_from_slice(&self.received[..self.received_len]);
            let length = self.received_len;
            self.received_len = 0;
            Ok(length)
        }
    }

    fn assert_public_transport<T: RastaTransport>(transport: &mut T) {
        transport.send(b"frame").unwrap();
        let mut buffer = [0u8; 8];
        assert_eq!(transport.receive(&mut buffer).unwrap(), 4);
        assert_eq!(&buffer[..4], b"data");
        assert_eq!(transport.receive(&mut buffer).unwrap(), 0);
    }

    #[test]
    fn mock_transport_implements_public_rasta_transport_trait() {
        let mut transport = MockTransport::with_receive(b"data");

        assert_public_transport(&mut transport);
        assert_eq!(&transport.sent[..transport.sent_len], b"frame");
    }
}

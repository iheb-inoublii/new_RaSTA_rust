use crate::port::{Transport, TransportError};

use super::defer_queue::DeferQueue;
use super::frame::{HEADER_SIZE, MAX_FRAME_SIZE, parse_header, payload_range, write_header};
use super::sequence::{ReceiveSequence, RedundancySequence};
use super::{RedundancyConfig, calculate, check_code_len};

pub struct RedundancyLayer<T1: Transport, T2: Transport> {
    transport_a: T1,
    transport_b: T2,
    config: RedundancyConfig,
    sequence: RedundancySequence,
    deferred: DeferQueue,
}

impl<T1: Transport, T2: Transport> RedundancyLayer<T1, T2> {
    pub const HEADER_SIZE: usize = HEADER_SIZE;

    pub fn new(transport_a: T1, transport_b: T2) -> Self {
        Self::with_config(transport_a, transport_b, RedundancyConfig::default())
    }

    pub fn with_config(transport_a: T1, transport_b: T2, config: RedundancyConfig) -> Self {
        Self {
            transport_a,
            transport_b,
            config,
            sequence: RedundancySequence::new(),
            deferred: DeferQueue::new(),
        }
    }

    pub fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        let mut buffer = [0u8; MAX_FRAME_SIZE];
        let total_len = data
            .len()
            .checked_add(HEADER_SIZE)
            .and_then(|length| length.checked_add(self.config.check_code_len()))
            .ok_or(TransportError::BufferTooSmall)?;
        if total_len > buffer.len() {
            return Err(TransportError::BufferTooSmall);
        }

        write_header(&mut buffer, total_len, self.sequence.transmit())?;
        buffer
            .get_mut(HEADER_SIZE..HEADER_SIZE + data.len())
            .ok_or(TransportError::BufferTooSmall)?
            .copy_from_slice(data);
        self.write_check_code(&mut buffer, HEADER_SIZE + data.len(), total_len)?;

        let frame = buffer
            .get(..total_len)
            .ok_or(TransportError::BufferTooSmall)?;
        let result_a = self.transport_a.send(frame);
        let result_b = self.transport_b.send(frame);
        if result_a.is_err() && result_b.is_err() {
            return Err(TransportError::SendFailed);
        }
        self.sequence.advance_transmit();
        Ok(())
    }

    pub fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError> {
        self.receive_at(buffer, 0)
    }

    /// Temporary raw-millisecond boundary for the unmigrated root connection.
    pub fn receive_at(&mut self, buffer: &mut [u8], now_ms: u32) -> Result<usize, TransportError> {
        if let Some(length) = self.deliver_ready_deferred(buffer, now_ms)? {
            return Ok(length);
        }
        let mut temporary = [0u8; MAX_FRAME_SIZE];
        let mut saw_receive_error = false;
        for channel in 0..2 {
            let result = if channel == 0 {
                self.transport_a.receive(&mut temporary)
            } else {
                self.transport_b.receive(&mut temporary)
            };
            match result {
                Ok(0) => {}
                Ok(bytes_read) => {
                    if let Some(length) =
                        self.accept_frame(&temporary, bytes_read, buffer, now_ms)?
                    {
                        return Ok(length);
                    }
                }
                Err(TransportError::ReceiveFailed) => saw_receive_error = true,
                Err(error) => return Err(error),
            }
        }
        if saw_receive_error {
            Err(TransportError::ReceiveFailed)
        } else {
            Ok(0)
        }
    }

    fn accept_frame(
        &mut self,
        frame: &[u8],
        bytes_read: usize,
        output: &mut [u8],
        now_ms: u32,
    ) -> Result<Option<usize>, TransportError> {
        let check_len = self.config.check_code_len();
        if bytes_read < HEADER_SIZE + check_len {
            return Ok(None);
        }
        let header = parse_header(frame)?;
        if header.declared_len != bytes_read {
            return Err(TransportError::InvalidFrame);
        }
        if !self.check_code_matches(frame, header.declared_len)? {
            return Err(TransportError::InvalidFrame);
        }
        match self.sequence.classify_receive(header.sequence) {
            ReceiveSequence::StaleOrDuplicate => return Ok(None),
            ReceiveSequence::Violation => return Err(TransportError::SequenceViolation),
            ReceiveSequence::Ahead => {
                self.deferred
                    .insert(frame, bytes_read, header.sequence, now_ms)?;
                return Ok(None);
            }
            ReceiveSequence::Expected => {}
        }
        self.sequence.accept(header.sequence);
        self.copy_payload(frame, header.declared_len, output)
            .map(Some)
    }

    fn deliver_ready_deferred(
        &mut self,
        output: &mut [u8],
        now_ms: u32,
    ) -> Result<Option<usize>, TransportError> {
        let Some(frame) =
            self.deferred
                .take_ready(self.sequence.expected(), now_ms, self.config.t_seq_ms)
        else {
            return Ok(None);
        };
        self.sequence.accept(frame.sequence);
        self.copy_payload(&frame.bytes, frame.len, output).map(Some)
    }

    fn copy_payload(
        &self,
        frame: &[u8],
        total_len: usize,
        output: &mut [u8],
    ) -> Result<usize, TransportError> {
        let range = payload_range(total_len, self.config.check_code_len())?;
        let payload = frame.get(range).ok_or(TransportError::InvalidFrame)?;
        if payload.len() > output.len() {
            return Err(TransportError::BufferTooSmall);
        }
        output
            .get_mut(..payload.len())
            .ok_or(TransportError::BufferTooSmall)?
            .copy_from_slice(payload);
        Ok(payload.len())
    }

    fn write_check_code(
        &self,
        frame: &mut [u8],
        check_start: usize,
        total_len: usize,
    ) -> Result<(), TransportError> {
        let Some(option) = self.config.crc_option() else {
            return Ok(());
        };
        let crc = calculate(
            option,
            frame
                .get(..check_start)
                .ok_or(TransportError::BufferTooSmall)?,
        );
        let destination = frame
            .get_mut(check_start..total_len)
            .ok_or(TransportError::BufferTooSmall)?;
        if check_code_len(option) == 2 {
            destination.copy_from_slice(&(crc as u16).to_le_bytes());
        } else {
            destination.copy_from_slice(&crc.to_le_bytes());
        }
        Ok(())
    }

    fn check_code_matches(&self, frame: &[u8], total_len: usize) -> Result<bool, TransportError> {
        let Some(option) = self.config.crc_option() else {
            return Ok(true);
        };
        let check_len = self.config.check_code_len();
        let check_start = total_len
            .checked_sub(check_len)
            .ok_or(TransportError::InvalidFrame)?;
        let expected = calculate(
            option,
            frame
                .get(..check_start)
                .ok_or(TransportError::BufferTooSmall)?,
        );
        let bytes = frame
            .get(check_start..total_len)
            .ok_or(TransportError::BufferTooSmall)?;
        if check_code_len(option) == 2 {
            Ok(u16::from_le_bytes([
                *bytes.first().ok_or(TransportError::BufferTooSmall)?,
                *bytes.get(1).ok_or(TransportError::BufferTooSmall)?,
            ]) == expected as u16)
        } else {
            Ok(u32::from_le_bytes([
                *bytes.first().ok_or(TransportError::BufferTooSmall)?,
                *bytes.get(1).ok_or(TransportError::BufferTooSmall)?,
                *bytes.get(2).ok_or(TransportError::BufferTooSmall)?,
                *bytes.get(3).ok_or(TransportError::BufferTooSmall)?,
            ]) == expected)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RedundancyLayer;
    use crate::port::{Transport, TransportError};
    use crate::redundancy::{RedundancyCheckCode, RedundancyConfig, calculate};
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Clone, Copy)]
    struct MockTransport {
        received: [u8; 520],
        received_len: usize,
        fail_send: bool,
    }

    impl MockTransport {
        fn empty() -> Self {
            Self {
                received: [0; 520],
                received_len: 0,
                fail_send: false,
            }
        }

        fn with_frame(frame: &[u8]) -> Self {
            let mut transport = Self::empty();
            transport.received[..frame.len()].copy_from_slice(frame);
            transport.received_len = frame.len();
            transport
        }
    }

    impl Transport for MockTransport {
        fn send(&mut self, _data: &[u8]) -> Result<(), TransportError> {
            if self.fail_send {
                Err(TransportError::SendFailed)
            } else {
                Ok(())
            }
        }

        fn receive(&mut self, output: &mut [u8]) -> Result<usize, TransportError> {
            if self.received_len == 0 {
                return Ok(0);
            }
            if output.len() < self.received_len {
                return Err(TransportError::BufferTooSmall);
            }
            output[..self.received_len].copy_from_slice(&self.received[..self.received_len]);
            let len = self.received_len;
            self.received_len = 0;
            Ok(len)
        }
    }

    fn frame(sequence: u32, payload: &[u8]) -> [u8; 520] {
        let mut bytes = [0u8; 520];
        let len = RedundancyLayer::<MockTransport, MockTransport>::HEADER_SIZE + payload.len();
        bytes[..2].copy_from_slice(&(len as u16).to_le_bytes());
        bytes[4..8].copy_from_slice(&sequence.to_le_bytes());
        bytes[8..len].copy_from_slice(payload);
        bytes
    }

    fn no_crc() -> RedundancyConfig {
        RedundancyConfig {
            check_code: RedundancyCheckCode::None,
            t_seq_ms: 100,
        }
    }

    #[test]
    fn accepts_non_zero_reserve_and_discards_duplicate_channel_copy() {
        let payload = b"safe-pdu";
        let mut bytes = frame(0, payload);
        bytes[2..4].copy_from_slice(&0xbeefu16.to_le_bytes());
        let len = 8 + payload.len();
        let mut layer = RedundancyLayer::with_config(
            MockTransport::with_frame(&bytes[..len]),
            MockTransport::with_frame(&bytes[..len]),
            no_crc(),
        );
        let mut output = [0u8; 32];
        let received = layer.receive(&mut output).unwrap();
        assert_eq!(&output[..received], payload);
        assert_eq!(layer.receive(&mut output), Ok(0));
    }

    #[test]
    fn defers_ahead_frame_then_releases_it_on_t_seq_expiry() {
        let ahead = frame(1, b"one");
        let ahead_len = 11;
        let mut layer = RedundancyLayer::with_config(
            MockTransport::with_frame(&ahead[..ahead_len]),
            MockTransport::empty(),
            no_crc(),
        );
        let mut output = [0u8; 32];
        assert_eq!(layer.receive_at(&mut output, 0), Ok(0));
        let received = layer.receive_at(&mut output, 100).unwrap();
        assert_eq!(&output[..received], b"one");
    }

    #[test]
    fn releases_a_deferred_frame_after_the_missing_frame_arrives() {
        let ahead = frame(1, b"one");
        let expected = frame(0, b"zero");
        let mut layer = RedundancyLayer::with_config(
            MockTransport::with_frame(&ahead[..11]),
            MockTransport::with_frame(&expected[..12]),
            no_crc(),
        );
        let mut output = [0u8; 32];
        let received = layer.receive_at(&mut output, 0).unwrap();
        assert_eq!(&output[..received], b"zero");
        let received = layer.receive_at(&mut output, 1).unwrap();
        assert_eq!(&output[..received], b"one");
    }

    #[test]
    fn rejects_a_malformed_check_code() {
        let payload = b"x";
        let mut bytes = frame(0, payload);
        let total_len = 8 + payload.len() + 4;
        bytes[..2].copy_from_slice(&(total_len as u16).to_le_bytes());
        let crc = calculate(crate::redundancy::RedundancyCrc::OptionB, &bytes[..9]);
        bytes[9..13].copy_from_slice(&crc.to_le_bytes());
        bytes[12] ^= 0xff;
        let mut layer = RedundancyLayer::new(
            MockTransport::with_frame(&bytes[..total_len]),
            MockTransport::empty(),
        );
        assert_eq!(
            layer.receive(&mut [0u8; 16]),
            Err(TransportError::InvalidFrame)
        );
    }

    #[test]
    fn reports_send_failure_only_when_both_channels_fail() {
        let mut failed = RedundancyLayer::new(
            MockTransport {
                fail_send: true,
                ..MockTransport::empty()
            },
            MockTransport {
                fail_send: true,
                ..MockTransport::empty()
            },
        );
        assert_eq!(failed.send(b"x"), Err(TransportError::SendFailed));

        let calls = Rc::new(Cell::new(0));
        struct CountingTransport(Rc<Cell<u8>>);
        impl Transport for CountingTransport {
            fn send(&mut self, _data: &[u8]) -> Result<(), TransportError> {
                self.0.set(self.0.get() + 1);
                Ok(())
            }
            fn receive(&mut self, _data: &mut [u8]) -> Result<usize, TransportError> {
                Ok(0)
            }
        }
        let mut layer = RedundancyLayer::new(
            CountingTransport(calls.clone()),
            CountingTransport(calls.clone()),
        );
        assert_eq!(layer.send(b"x"), Ok(()));
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn writes_check_codes_in_little_endian_wire_order() {
        struct RecordingTransport(Rc<RefCell<[u8; 520]>>, Rc<Cell<usize>>);
        impl Transport for RecordingTransport {
            fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
                self.0.borrow_mut()[..data.len()].copy_from_slice(data);
                self.1.set(data.len());
                Ok(())
            }
            fn receive(&mut self, _data: &mut [u8]) -> Result<usize, TransportError> {
                Ok(0)
            }
        }

        let captured = Rc::new(RefCell::new([0u8; 520]));
        let captured_len = Rc::new(Cell::new(0));
        let mut layer = RedundancyLayer::with_config(
            RecordingTransport(captured.clone(), captured_len.clone()),
            RecordingTransport(captured.clone(), captured_len.clone()),
            RedundancyConfig {
                check_code: RedundancyCheckCode::OptionB,
                t_seq_ms: 100,
            },
        );
        layer.send(b"x").unwrap();
        let bytes = captured.borrow();
        let len = captured_len.get();
        let expected = calculate(crate::redundancy::RedundancyCrc::OptionB, &bytes[..9]);
        assert_eq!(len, 13);
        assert_eq!(&bytes[9..13], &expected.to_le_bytes());
    }
}

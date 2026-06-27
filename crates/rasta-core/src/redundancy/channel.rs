use crate::port::{Transport, TransportError};
use crate::time::{DurationMs, MonotonicInstant};

use super::defer_queue::DeferQueue;
use super::frame::{HEADER_SIZE, MAX_FRAME_SIZE, parse_header, payload_range, write_header};
use super::sequence::{ReceiveSequence, RedundancySequence};
use super::{RedundancyConfig, calculate, check_code_len};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelId {
    Channel0,
    Channel1,
}

impl ChannelId {
    const fn index(self) -> usize {
        match self {
            Self::Channel0 => 0,
            Self::Channel1 => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelStatus {
    Unknown,
    Healthy,
    Degraded,
    Failed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChannelCounters {
    pub valid_frame_count: u32,
    pub duplicate_frame_count: u32,
    pub send_failure_count: u32,
    pub receive_failure_count: u32,
    pub validation_failure_count: u32,
    pub sequence_error_count: u32,
    pub status_transition_count: u32,
}

#[derive(Clone, Copy, Debug)]
struct ChannelState {
    status: ChannelStatus,
    last_valid_receive: Option<MonotonicInstant>,
    last_send_success: Option<MonotonicInstant>,
    consecutive_send_failures: u32,
    consecutive_receive_failures: u32,
    counters: ChannelCounters,
}

impl ChannelState {
    const fn new() -> Self {
        Self {
            status: ChannelStatus::Unknown,
            last_valid_receive: None,
            last_send_success: None,
            consecutive_send_failures: 0,
            consecutive_receive_failures: 0,
            counters: ChannelCounters {
                valid_frame_count: 0,
                duplicate_frame_count: 0,
                send_failure_count: 0,
                receive_failure_count: 0,
                validation_failure_count: 0,
                sequence_error_count: 0,
                status_transition_count: 0,
            },
        }
    }
}

pub struct RedundancyLayer<T1: Transport, T2: Transport> {
    transport_a: T1,
    transport_b: T2,
    config: RedundancyConfig,
    sequence: RedundancySequence,
    deferred: DeferQueue,
    channels: [ChannelState; 2],
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
            channels: [ChannelState::new(), ChannelState::new()],
        }
    }

    pub fn channel_status(&self, channel: ChannelId) -> ChannelStatus {
        self.channels[channel.index()].status
    }

    pub fn channel_statuses(&self) -> [ChannelStatus; 2] {
        [self.channels[0].status, self.channels[1].status]
    }

    pub fn channel_counters(&self, channel: ChannelId) -> ChannelCounters {
        self.channels[channel.index()].counters
    }

    pub fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.send_at(data, MonotonicInstant::from_wrapping_millis(0))
    }

    pub fn send_at(&mut self, data: &[u8], now: MonotonicInstant) -> Result<(), TransportError> {
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
        self.record_send_result(ChannelId::Channel0, result_a.is_ok(), now);
        self.record_send_result(ChannelId::Channel1, result_b.is_ok(), now);
        if result_a.is_err() && result_b.is_err() {
            return Err(TransportError::SendFailed);
        }
        self.sequence.advance_transmit();
        Ok(())
    }

    pub fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError> {
        self.receive_at(buffer, MonotonicInstant::from_wrapping_millis(0))
    }

    pub fn receive_at(
        &mut self,
        buffer: &mut [u8],
        now: MonotonicInstant,
    ) -> Result<usize, TransportError> {
        self.update_timeouts(now);
        if let Some(length) = self.deliver_ready_deferred(buffer, now)? {
            return Ok(length);
        }
        let mut temporary = [0u8; MAX_FRAME_SIZE];
        let mut receive_failures = 0usize;
        let mut validation_failures = 0usize;
        let mut last_validation_error = TransportError::InvalidFrame;
        for channel in [ChannelId::Channel0, ChannelId::Channel1] {
            let result = if channel == ChannelId::Channel0 {
                self.transport_a.receive(&mut temporary)
            } else {
                self.transport_b.receive(&mut temporary)
            };
            match result {
                Ok(0) => {}
                Ok(bytes_read) => {
                    match self.accept_frame(channel, &temporary, bytes_read, buffer, now) {
                        Ok(Some(length)) => return Ok(length),
                        Ok(None) => {}
                        Err(TransportError::BufferTooSmall) => {
                            return Err(TransportError::BufferTooSmall);
                        }
                        Err(error) => {
                            validation_failures += 1;
                            last_validation_error = error;
                        }
                    }
                }
                Err(TransportError::ReceiveFailed) => {
                    receive_failures += 1;
                    self.record_receive_failure(channel);
                }
                Err(error) => return Err(error),
            }
        }
        if receive_failures == 2 {
            Err(TransportError::ReceiveFailed)
        } else if validation_failures > 0 {
            Err(last_validation_error)
        } else {
            Ok(0)
        }
    }

    fn accept_frame(
        &mut self,
        channel: ChannelId,
        frame: &[u8],
        bytes_read: usize,
        output: &mut [u8],
        now: MonotonicInstant,
    ) -> Result<Option<usize>, TransportError> {
        let check_len = self.config.check_code_len();
        if bytes_read < HEADER_SIZE + check_len {
            self.record_validation_failure(channel, false);
            return Ok(None);
        }
        let header = parse_header(frame)?;
        if header.declared_len != bytes_read {
            self.record_validation_failure(channel, false);
            return Err(TransportError::InvalidFrame);
        }
        if !self.check_code_matches(frame, header.declared_len)? {
            self.record_validation_failure(channel, false);
            return Err(TransportError::InvalidFrame);
        }
        match self.sequence.classify_receive(header.sequence) {
            ReceiveSequence::StaleOrDuplicate => {
                self.record_valid_frame(channel, now, true);
                return Ok(None);
            }
            ReceiveSequence::Violation => {
                self.record_validation_failure(channel, true);
                return Err(TransportError::SequenceViolation);
            }
            ReceiveSequence::Ahead => {
                self.record_valid_frame(channel, now, false);
                self.deferred
                    .insert(frame, bytes_read, header.sequence, now)?;
                return Ok(None);
            }
            ReceiveSequence::Expected => {}
        }
        self.record_valid_frame(channel, now, false);
        self.sequence.accept(header.sequence);
        self.copy_payload(frame, header.declared_len, output)
            .map(Some)
    }

    fn deliver_ready_deferred(
        &mut self,
        output: &mut [u8],
        now: MonotonicInstant,
    ) -> Result<Option<usize>, TransportError> {
        let Some(frame) = self.deferred.take_ready(
            self.sequence.expected(),
            now,
            DurationMs::from_millis(self.config.t_seq_ms),
        ) else {
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

    fn record_send_result(&mut self, channel: ChannelId, success: bool, now: MonotonicInstant) {
        let state = &mut self.channels[channel.index()];
        if success {
            state.last_send_success = Some(now);
            state.consecutive_send_failures = 0;
            Self::set_status(state, ChannelStatus::Healthy);
        } else {
            state.counters.send_failure_count = state.counters.send_failure_count.saturating_add(1);
            state.consecutive_send_failures = state.consecutive_send_failures.saturating_add(1);
            Self::set_status(state, ChannelStatus::Degraded);
        }
    }

    fn record_receive_failure(&mut self, channel: ChannelId) {
        let state = &mut self.channels[channel.index()];
        state.counters.receive_failure_count =
            state.counters.receive_failure_count.saturating_add(1);
        state.consecutive_receive_failures = state.consecutive_receive_failures.saturating_add(1);
        Self::set_status(state, ChannelStatus::Degraded);
    }

    fn record_validation_failure(&mut self, channel: ChannelId, sequence_error: bool) {
        let state = &mut self.channels[channel.index()];
        state.counters.validation_failure_count =
            state.counters.validation_failure_count.saturating_add(1);
        if sequence_error {
            state.counters.sequence_error_count =
                state.counters.sequence_error_count.saturating_add(1);
        }
        Self::set_status(state, ChannelStatus::Degraded);
    }

    fn record_valid_frame(&mut self, channel: ChannelId, now: MonotonicInstant, duplicate: bool) {
        let state = &mut self.channels[channel.index()];
        state.last_valid_receive = Some(now);
        state.consecutive_receive_failures = 0;
        state.counters.valid_frame_count = state.counters.valid_frame_count.saturating_add(1);
        if duplicate {
            state.counters.duplicate_frame_count =
                state.counters.duplicate_frame_count.saturating_add(1);
        }
        Self::set_status(state, ChannelStatus::Healthy);
    }

    fn update_timeouts(&mut self, now: MonotonicInstant) {
        let timeout = DurationMs::from_millis(self.config.t_seq_ms);
        for state in &mut self.channels {
            if let Some(last_valid_receive) = state.last_valid_receive
                && now.elapsed_since(last_valid_receive).as_millis() >= timeout.as_millis()
            {
                Self::set_status(state, ChannelStatus::Failed);
            }
        }
    }

    fn set_status(state: &mut ChannelState, status: ChannelStatus) {
        if state.status != status {
            state.status = status;
            state.counters.status_transition_count =
                state.counters.status_transition_count.saturating_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ChannelId, ChannelStatus, RedundancyLayer};
    use crate::port::{Transport, TransportError};
    use crate::redundancy::{RedundancyCheckCode, RedundancyConfig, calculate};
    use crate::time::MonotonicInstant;
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Clone, Copy)]
    struct MockTransport {
        received: [u8; 520],
        received_len: usize,
        fail_send: bool,
        fail_receive: bool,
    }

    impl MockTransport {
        fn empty() -> Self {
            Self {
                received: [0; 520],
                received_len: 0,
                fail_send: false,
                fail_receive: false,
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
            if self.fail_receive {
                return Err(TransportError::ReceiveFailed);
            }
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

    fn frame_with_option_b(sequence: u32, payload: &[u8]) -> ([u8; 520], usize) {
        let mut bytes = [0u8; 520];
        let payload_end =
            RedundancyLayer::<MockTransport, MockTransport>::HEADER_SIZE + payload.len();
        let total_len = payload_end + 4;
        bytes[..2].copy_from_slice(&(total_len as u16).to_le_bytes());
        bytes[4..8].copy_from_slice(&sequence.to_le_bytes());
        bytes[8..payload_end].copy_from_slice(payload);
        let crc = calculate(
            crate::redundancy::RedundancyCrc::OptionB,
            &bytes[..payload_end],
        );
        bytes[payload_end..total_len].copy_from_slice(&crc.to_le_bytes());
        (bytes, total_len)
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
        assert_eq!(
            layer.channel_statuses(),
            [ChannelStatus::Healthy, ChannelStatus::Healthy]
        );
        assert_eq!(
            layer
                .channel_counters(ChannelId::Channel1)
                .duplicate_frame_count,
            1
        );
    }

    #[test]
    fn initial_channel_status_is_unknown() {
        let layer =
            RedundancyLayer::with_config(MockTransport::empty(), MockTransport::empty(), no_crc());
        assert_eq!(
            layer.channel_statuses(),
            [ChannelStatus::Unknown, ChannelStatus::Unknown]
        );
    }

    #[test]
    fn valid_activity_marks_each_channel_healthy() {
        let payload = b"ok";
        let bytes = frame(0, payload);
        let mut layer = RedundancyLayer::with_config(
            MockTransport::empty(),
            MockTransport::with_frame(&bytes[..10]),
            no_crc(),
        );
        let mut output = [0u8; 8];
        let received = layer
            .receive_at(&mut output, MonotonicInstant::from_wrapping_millis(5))
            .unwrap();
        assert_eq!(&output[..received], payload);
        assert_eq!(
            layer.channel_status(ChannelId::Channel1),
            ChannelStatus::Healthy
        );
        assert_eq!(
            layer
                .channel_counters(ChannelId::Channel1)
                .valid_frame_count,
            1
        );
        assert_eq!(
            layer.channel_status(ChannelId::Channel0),
            ChannelStatus::Unknown
        );
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
        assert_eq!(
            layer.receive_at(&mut output, MonotonicInstant::from_wrapping_millis(0)),
            Ok(0)
        );
        let received = layer
            .receive_at(&mut output, MonotonicInstant::from_wrapping_millis(100))
            .unwrap();
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
        let received = layer
            .receive_at(&mut output, MonotonicInstant::from_wrapping_millis(0))
            .unwrap();
        assert_eq!(&output[..received], b"zero");
        let received = layer
            .receive_at(&mut output, MonotonicInstant::from_wrapping_millis(1))
            .unwrap();
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
        assert_eq!(
            layer.channel_status(ChannelId::Channel0),
            ChannelStatus::Degraded
        );
        assert_eq!(
            layer
                .channel_counters(ChannelId::Channel0)
                .validation_failure_count,
            1
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
        assert_eq!(
            failed.channel_statuses(),
            [ChannelStatus::Degraded, ChannelStatus::Degraded]
        );
        assert_eq!(
            failed
                .channel_counters(ChannelId::Channel0)
                .send_failure_count,
            1
        );

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
        assert_eq!(
            layer.channel_statuses(),
            [ChannelStatus::Healthy, ChannelStatus::Healthy]
        );
    }

    #[test]
    fn single_channel_send_failure_degrades_only_that_channel() {
        let mut layer = RedundancyLayer::with_config(
            MockTransport {
                fail_send: true,
                ..MockTransport::empty()
            },
            MockTransport::empty(),
            no_crc(),
        );
        assert_eq!(layer.send(b"x"), Ok(()));
        assert_eq!(
            layer.channel_statuses(),
            [ChannelStatus::Degraded, ChannelStatus::Healthy]
        );
        assert_eq!(
            layer
                .channel_counters(ChannelId::Channel0)
                .send_failure_count,
            1
        );
        assert_eq!(
            layer
                .channel_counters(ChannelId::Channel1)
                .send_failure_count,
            0
        );
    }

    #[test]
    fn crc_failure_on_one_channel_still_accepts_valid_other_channel() {
        let (mut bad, bad_len) = frame_with_option_b(0, b"bad");
        bad[bad_len - 1] ^= 0xff;
        let (good, good_len) = frame_with_option_b(0, b"good");
        let mut layer = RedundancyLayer::new(
            MockTransport::with_frame(&bad[..bad_len]),
            MockTransport::with_frame(&good[..good_len]),
        );
        let mut output = [0u8; 8];
        let received = layer.receive(&mut output).unwrap();
        assert_eq!(&output[..received], b"good");
        assert_eq!(
            layer.channel_status(ChannelId::Channel0),
            ChannelStatus::Degraded
        );
        assert_eq!(
            layer.channel_status(ChannelId::Channel1),
            ChannelStatus::Healthy
        );
    }

    #[test]
    fn malformed_frame_on_one_channel_still_accepts_valid_other_channel() {
        let mut malformed = frame(0, b"bad");
        malformed[..2].copy_from_slice(&99u16.to_le_bytes());
        let valid = frame(0, b"ok");
        let mut layer = RedundancyLayer::with_config(
            MockTransport::with_frame(&malformed[..11]),
            MockTransport::with_frame(&valid[..10]),
            no_crc(),
        );
        let mut output = [0u8; 8];
        let received = layer.receive(&mut output).unwrap();
        assert_eq!(&output[..received], b"ok");
        assert_eq!(
            layer.channel_status(ChannelId::Channel0),
            ChannelStatus::Degraded
        );
        assert_eq!(
            layer.channel_status(ChannelId::Channel1),
            ChannelStatus::Healthy
        );
    }

    #[test]
    fn receive_failure_on_one_channel_does_not_fail_remaining_channel() {
        let valid = frame(0, b"ok");
        let mut layer = RedundancyLayer::with_config(
            MockTransport {
                fail_receive: true,
                ..MockTransport::empty()
            },
            MockTransport::with_frame(&valid[..10]),
            no_crc(),
        );
        let mut output = [0u8; 8];
        let received = layer.receive(&mut output).unwrap();
        assert_eq!(&output[..received], b"ok");
        assert_eq!(
            layer.channel_status(ChannelId::Channel0),
            ChannelStatus::Degraded
        );
        assert_eq!(
            layer.channel_status(ChannelId::Channel1),
            ChannelStatus::Healthy
        );
        assert_eq!(
            layer
                .channel_counters(ChannelId::Channel0)
                .receive_failure_count,
            1
        );
    }

    #[test]
    fn both_channel_receive_failures_report_failure() {
        let mut layer = RedundancyLayer::with_config(
            MockTransport {
                fail_receive: true,
                ..MockTransport::empty()
            },
            MockTransport {
                fail_receive: true,
                ..MockTransport::empty()
            },
            no_crc(),
        );
        assert_eq!(
            layer.receive(&mut [0u8; 8]),
            Err(TransportError::ReceiveFailed)
        );
        assert_eq!(
            layer.channel_statuses(),
            [ChannelStatus::Degraded, ChannelStatus::Degraded]
        );
    }

    #[test]
    fn channel_timeout_and_recovery_are_deterministic() {
        let valid = frame(0, b"ok");
        let mut layer = RedundancyLayer::with_config(
            MockTransport::with_frame(&valid[..10]),
            MockTransport::empty(),
            no_crc(),
        );
        let mut output = [0u8; 8];
        layer
            .receive_at(&mut output, MonotonicInstant::from_wrapping_millis(0))
            .unwrap();
        assert_eq!(
            layer.channel_status(ChannelId::Channel0),
            ChannelStatus::Healthy
        );

        assert_eq!(
            layer.receive_at(&mut output, MonotonicInstant::from_wrapping_millis(100)),
            Ok(0)
        );
        assert_eq!(
            layer.channel_status(ChannelId::Channel0),
            ChannelStatus::Failed
        );

        let recovery = frame(1, b"up");
        layer.transport_a = MockTransport::with_frame(&recovery[..10]);
        let received = layer
            .receive_at(&mut output, MonotonicInstant::from_wrapping_millis(101))
            .unwrap();
        assert_eq!(&output[..received], b"up");
        assert_eq!(
            layer.channel_status(ChannelId::Channel0),
            ChannelStatus::Healthy
        );
    }

    #[test]
    fn out_of_order_frame_counts_as_valid_activity_and_missing_frame_can_arrive_elsewhere() {
        let ahead = frame(1, b"one");
        let expected = frame(0, b"zero");
        let mut layer = RedundancyLayer::with_config(
            MockTransport::with_frame(&ahead[..11]),
            MockTransport::with_frame(&expected[..12]),
            no_crc(),
        );
        let mut output = [0u8; 32];
        let received = layer.receive(&mut output).unwrap();
        assert_eq!(&output[..received], b"zero");
        assert_eq!(
            layer.channel_status(ChannelId::Channel0),
            ChannelStatus::Healthy
        );
        assert_eq!(
            layer.channel_status(ChannelId::Channel1),
            ChannelStatus::Healthy
        );
    }

    #[test]
    fn sequence_wraparound_preserves_channel_health() {
        let first = frame(u32::MAX - 1, b"a");
        let second = frame(u32::MAX, b"b");
        let third = frame(0, b"c");
        let mut layer = RedundancyLayer::with_config(
            MockTransport::with_frame(&first[..9]),
            MockTransport::empty(),
            no_crc(),
        );
        layer.sequence.accept(u32::MAX - 2);
        let mut output = [0u8; 8];
        assert_eq!(layer.receive(&mut output).unwrap(), 1);
        layer.transport_b = MockTransport::with_frame(&second[..9]);
        assert_eq!(layer.receive(&mut output).unwrap(), 1);
        layer.transport_a = MockTransport::with_frame(&third[..9]);
        assert_eq!(layer.receive(&mut output).unwrap(), 1);
        assert_eq!(
            layer.channel_statuses(),
            [ChannelStatus::Healthy, ChannelStatus::Healthy]
        );
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

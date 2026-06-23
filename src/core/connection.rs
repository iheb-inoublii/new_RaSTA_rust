use crate::core::connection_state_machine::{RastaState, StateMachine};
use crate::core::heartbeat::HeartbeatHandler;
use crate::core::pdu::{Packet, PacketError, PacketType};
use crate::core::redundancy_management::{RedundancyConfig, RedundancyLayer};
use crate::core::retransmission::RetransmissionBuffer;
use crate::core::safety_code::SafetyCodeConfig;
use crate::core::sequencing::{SequenceHandler, SequenceResult};
use crate::platform::clock::Clock;
use crate::platform::random::{RandomError, RandomSource};
use crate::platform::timer::Timer;
use crate::platform::transport::{Transport, TransportError};
use crate::serial;
use crate::srl::DisconnectReason;
use crate::{
    fixed_queue::{FixedQueue, FixedQueueError},
    srl::{DiagnosticEvent, DiagnosticKind, SrlErrorCounters},
};

#[derive(Debug)]
pub enum ConnectionError {
    Transport(TransportError),
    Packet(PacketError),
    UnexpectedPacket,
    BufferFull,
    ProtocolViolation,
    SafetyTimeout,
    StateTransitionInvalid,
    InvalidPayloadSize,
    ReceiveQueueEmpty,
    ReceiveQueueFull,
    TransmitQueueFull,
    InvalidConfiguration,
    RetransmissionUnavailable,
    Random(RandomError),
}

#[derive(Clone, Copy)]
pub struct RastaConfig {
    pub sender_id: u32,
    pub remote_id: u32,
    pub safety_code: SafetyCodeConfig,
    pub redundancy: RedundancyConfig,
    pub t_max: u32,
    pub initial_seq: u32,
    pub heartbeat_interval_ms: u32,
    pub n_send_max: u16,
    pub mwa: u16,
}

pub struct RastaConnection<T1: Transport, T2: Transport, TimerCtx: Timer, C: Clock> {
    pub state_machine: StateMachine,
    pub redundancy: RedundancyLayer<T1, T2>,
    pub clock: C,
    pub heartbeat: HeartbeatHandler<TimerCtx>,
    pub sequence: SequenceHandler,
    pub retransmission: RetransmissionBuffer,
    pub sender_id: u32,
    pub remote_id: u32,
    is_client: bool,
    pub safety_code: SafetyCodeConfig,
    pub t_max: u32,
    pub n_send_max: u16,
    pub mwa: u16,
    remote_n_send_max: u16,
    last_tx_sequence: Option<u32>,
    last_confirmed_by_peer: Option<u32>,
    received_since_ack: u16,
    last_received_timestamp: u32,
    confirmed_timestamp_reference: Option<u32>,
    timeliness_deadline_ms: Option<u32>,
    rx_buffer: [u8; 512],
    tx_buffer: [u8; 512],
    app_rx_buffer: [[u8; 256]; 20],
    app_rx_len: [usize; 20],
    app_rx_head: usize,
    app_rx_tail: usize,
    app_rx_count: usize,
    app_tx_buffer: [[u8; 256]; 20],
    app_tx_len: [usize; 20],
    app_tx_head: usize,
    app_tx_tail: usize,
    app_tx_count: usize,
    diagnostics: FixedQueue<DiagnosticEvent, 16>,
    diagnostic_overflow_count: u32,
    error_counters: SrlErrorCounters,
}

impl<T1: Transport, T2: Transport, TimerCtx: Timer, C: Clock> RastaConnection<T1, T2, TimerCtx, C> {
    pub fn try_new(
        transport_a: T1,
        transport_b: T2,
        timer: TimerCtx,
        clock: C,
        config: RastaConfig,
    ) -> Result<Self, ConnectionError> {
        if config.sender_id == 0
            || config.remote_id == 0
            || config.sender_id == config.remote_id
            || config.t_max == 0
            || config.heartbeat_interval_ms == 0
            || config.n_send_max == 0
            || config.n_send_max > 20
            || config.mwa == 0
            || config.mwa >= config.n_send_max
            || config.redundancy.t_seq_ms == 0
            || matches!(
                config.safety_code.mode,
                crate::core::safety_code::SafetyCodeMode::None
            )
            || matches!(
                config.redundancy.check_code,
                crate::core::redundancy_management::RedundancyCheckCode::None
            )
        {
            return Err(ConnectionError::InvalidConfiguration);
        }
        Ok(RastaConnection {
            state_machine: StateMachine::new(),
            redundancy: RedundancyLayer::with_config(transport_a, transport_b, config.redundancy),
            clock,
            heartbeat: HeartbeatHandler::new(timer, config.heartbeat_interval_ms),
            sequence: SequenceHandler::with_initial_tx(config.initial_seq),
            retransmission: RetransmissionBuffer::with_capacity(config.n_send_max as usize),
            sender_id: config.sender_id,
            remote_id: config.remote_id,
            is_client: config.sender_id < config.remote_id,
            safety_code: config.safety_code,
            t_max: config.t_max,
            n_send_max: config.n_send_max,
            mwa: config.mwa,
            remote_n_send_max: config.n_send_max,
            last_tx_sequence: None,
            last_confirmed_by_peer: None,
            received_since_ack: 0,
            last_received_timestamp: 0,
            confirmed_timestamp_reference: None,
            timeliness_deadline_ms: None,
            rx_buffer: [0; 512],
            tx_buffer: [0; 512],
            app_rx_buffer: [[0; 256]; 20],
            app_rx_len: [0; 20],
            app_rx_head: 0,
            app_rx_tail: 0,
            app_rx_count: 0,
            app_tx_buffer: [[0; 256]; 20],
            app_tx_len: [0; 20],
            app_tx_head: 0,
            app_tx_tail: 0,
            app_tx_count: 0,
            diagnostics: FixedQueue::new(),
            diagnostic_overflow_count: 0,
            error_counters: SrlErrorCounters::default(),
        })
    }

    pub fn try_new_with_random<R: RandomSource>(
        transport_a: T1,
        transport_b: T2,
        timer: TimerCtx,
        clock: C,
        mut config: RastaConfig,
        random: &mut R,
    ) -> Result<Self, ConnectionError> {
        config.initial_seq = random.next_u32().map_err(ConnectionError::Random)?;
        Self::try_new(transport_a, transport_b, timer, clock, config)
    }

    pub fn transition(&mut self, new_state: RastaState) -> Result<(), ConnectionError> {
        if self.state_machine.transition(new_state) {
            Ok(())
        } else {
            Err(ConnectionError::StateTransitionInvalid)
        }
    }

    pub fn connect(&mut self) -> Result<(), ConnectionError> {
        if self.state_machine.current_state != RastaState::Closed {
            return Err(ConnectionError::ProtocolViolation);
        }
        self.transition(RastaState::Down)?;
        self.heartbeat.reset();
        // DIN 5.5.12: the lower identification is the client.
        if self.sender_id > self.remote_id {
            return Ok(());
        }
        self.transition(RastaState::Start)?;
        self.start_timeliness_monitor(self.clock.now_ms());
        let payload = self.connection_payload();
        self.send_packet(
            PacketType::ConnectionRequest,
            payload
                .get(..14)
                .ok_or(ConnectionError::InvalidPayloadSize)?,
        )?;
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<(), ConnectionError> {
        if self.state_machine.current_state != RastaState::Closed {
            self.send_disconnect(DisconnectReason::UserRequest)?;
            self.transition(RastaState::Closed)?;
            self.stop_timeliness_monitor();
        }
        Ok(())
    }

    pub fn disconnect_with_error(&mut self) -> Result<(), ConnectionError> {
        self.disconnect_with_reason(DisconnectReason::ProtocolSequenceError)
    }

    fn disconnect_with_reason(&mut self, reason: DisconnectReason) -> Result<(), ConnectionError> {
        let send_result = self.send_disconnect(reason);
        if self.state_machine.current_state != RastaState::Closed {
            self.transition(RastaState::Closed)?;
            self.stop_timeliness_monitor();
        }
        send_result
    }

    pub fn process(&mut self) -> Result<(), ConnectionError> {
        let local_now = self.clock.now_ms();
        if self
            .timeliness_deadline_ms
            .is_some_and(|deadline| local_now == deadline || serial::is_after(local_now, deadline))
        {
            self.record_diagnostic(DiagnosticKind::ConnectionTimeout, local_now);
            self.disconnect_with_error()?;
            return Err(ConnectionError::SafetyTimeout);
        }
        if self.heartbeat.is_due() {
            if !matches!(
                self.state_machine.current_state,
                RastaState::Closed | RastaState::Down
            ) {
                self.send_packet(PacketType::Heartbeat, &[])?;
            }
            self.heartbeat.reset();
        }

        for _ in 0..32 {
            let bytes_read = self
                .redundancy
                .receive_at(&mut self.rx_buffer, local_now)
                .map_err(ConnectionError::Transport)?;
            if bytes_read == 0 {
                break;
            }
            let rx_slice = self
                .rx_buffer
                .get(..bytes_read)
                .ok_or(ConnectionError::BufferFull)?;
            let parse_res = Packet::parse(rx_slice, &self.safety_code);
            match parse_res {
                Ok(packet) => self.handle_packet(packet)?,
                Err(PacketError::ChecksumMismatch) => {
                    self.record_diagnostic(DiagnosticKind::SafetyCodeError, 1);
                    continue;
                }
                Err(PacketError::UnsupportedProtocolVersion) => {
                    self.record_diagnostic(DiagnosticKind::ProtocolVersionError, 1);
                    self.disconnect_with_reason(DisconnectReason::ProtocolVersionError)?;
                    return Err(ConnectionError::Packet(
                        PacketError::UnsupportedProtocolVersion,
                    ));
                }
                Err(PacketError::InvalidType) => {
                    self.error_counters.message_type =
                        self.error_counters.message_type.saturating_add(1);
                    self.record_diagnostic(DiagnosticKind::MalformedMessage, 1);
                    continue;
                }
                Err(e) => {
                    self.record_diagnostic(DiagnosticKind::MalformedMessage, 1);
                    let _ = e;
                    continue;
                }
            }
        }

        self.flush_application_tx()?;
        Ok(())
    }

    fn handle_packet(&mut self, packet: Packet) -> Result<(), ConnectionError> {
        // 1. Check IDs
        if self.state_machine.current_state != RastaState::Down {
            if packet.receiver_id != self.sender_id || packet.sender_id != self.remote_id {
                return Err(ConnectionError::ProtocolViolation);
            }
        } else {
            // In Down state, we only accept ConnectionRequest
            if packet.packet_type != PacketType::ConnectionRequest {
                return Err(ConnectionError::UnexpectedPacket);
            }
            if (packet.receiver_id != 0 && packet.receiver_id != self.sender_id)
                || packet.sender_id != self.remote_id
            {
                // Receiver ID in ConnectionRequest can be 0 or local sender_id
                return Err(ConnectionError::ProtocolViolation);
            }
        }

        let local_now = self.clock.now_ms();

        match packet.packet_type {
            PacketType::ConnectionRequest
            | PacketType::ConnectionResponse
            | PacketType::RetransmissionResponse
            | PacketType::DisconnectionRequest => {}
            _ => {
                if !self
                    .sequence
                    .validate_range(packet.sequence_number, self.remote_n_send_max)
                {
                    self.record_diagnostic(DiagnosticKind::SequenceError, packet.sequence_number);
                    return Err(ConnectionError::ProtocolViolation);
                }
                match self.sequence.validate_rx(packet.sequence_number) {
                    SequenceResult::Ok => {}
                    SequenceResult::Gap(expected) => {
                        self.transition(RastaState::RetransmissionRequested)?;
                        let _ = expected;
                        self.send_packet(PacketType::RetransmissionRequest, &[])?;
                        return Ok(());
                    }
                    SequenceResult::Duplicate => return Ok(()),
                }
            }
        }

        // DIN 5.5.6.1: copy the timestamp of every formally correct received
        // message into the next outbound PDU; only time-out related PDUs are
        // analysed by the adaptive monitor.
        self.last_received_timestamp = packet.timestamp;
        self.apply_timeliness(&packet, local_now)?;
        self.heartbeat.reset();
        self.apply_confirmation(packet.confirmed_sequence_number)?;

        match self.state_machine.current_state {
            RastaState::Down if packet.packet_type == PacketType::ConnectionRequest => {
                self.start_timeliness_monitor(local_now);
                self.sequence.accept_initial_rx(packet.sequence_number);
                self.apply_connection_payload(&packet)?;
                self.transition(RastaState::Start)?;
                self.heartbeat.reset();
                let payload = self.connection_payload();
                self.send_packet(
                    PacketType::ConnectionResponse,
                    payload
                        .get(..14)
                        .ok_or(ConnectionError::InvalidPayloadSize)?,
                )?;
            }
            RastaState::Start => match packet.packet_type {
                PacketType::ConnectionResponse => {
                    if !self.is_client {
                        return self.reject_unexpected_packet();
                    }
                    self.sequence.accept_initial_rx(packet.sequence_number);
                    self.apply_connection_payload(&packet)?;
                    self.transition(RastaState::Up)?;
                    self.error_counters.reset();
                    self.send_packet(PacketType::Heartbeat, &[])?;
                }
                PacketType::Heartbeat => {
                    if self.is_client {
                        return self.reject_unexpected_packet();
                    }
                    self.transition(RastaState::Up)?;
                    self.error_counters.reset();
                }
                PacketType::ConnectionRequest => return self.reject_unexpected_packet(),
                _ => {
                    return self.reject_unexpected_packet();
                }
            },
            RastaState::Up
            | RastaState::RetransmissionRequested
            | RastaState::RetransmissionRunning => {
                match packet.packet_type {
                    PacketType::RetransmissionRequest => {
                        self.send_packet(PacketType::RetransmissionResponse, &[])?;
                        self.retransmit_from(packet.confirmed_sequence_number.wrapping_add(1))?;
                        // DIN 5.5.11: a regular message terminates retransmission.
                        self.send_packet(PacketType::Heartbeat, &[])?;
                    }
                    PacketType::RetransmissionResponse => {
                        if self.state_machine.current_state == RastaState::RetransmissionRequested {
                            self.sequence.accept_initial_rx(packet.sequence_number);
                            self.transition(RastaState::RetransmissionRunning)?;
                        } else {
                            return self.reject_unexpected_packet();
                        }
                    }
                    PacketType::DisconnectionRequest => {
                        self.transition(RastaState::Closed)?;
                        self.stop_timeliness_monitor();
                    }
                    PacketType::Data | PacketType::RetransmissionData => {
                        self.enqueue_application_data(&packet)?;
                        self.received_since_ack = self.received_since_ack.saturating_add(1);
                        if self.received_since_ack >= self.mwa {
                            self.send_packet(PacketType::Heartbeat, &[])?;
                            self.received_since_ack = 0;
                        }
                        if self.state_machine.current_state == RastaState::RetransmissionRunning
                            && packet.packet_type == PacketType::Data
                        {
                            self.transition(RastaState::Up)?;
                        }
                    }
                    PacketType::Heartbeat => {
                        if matches!(
                            self.state_machine.current_state,
                            RastaState::RetransmissionRequested | RastaState::RetransmissionRunning
                        ) {
                            self.transition(RastaState::Up)?;
                        }
                    }
                    _ => {
                        return self.reject_unexpected_packet();
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn retransmit_from(&mut self, start_seq: u32) -> Result<(), ConnectionError> {
        let mut count = self.retransmission.count();
        let mut current_seq: u32 = start_seq;

        // If start_seq is 0, we find the oldest packet
        if start_seq == 0 {
            let mut found_start = false;
            for p in self.retransmission.packets.iter().flatten() {
                if !found_start || current_seq.wrapping_sub(p.sequence_number) < 0x80000000 {
                    current_seq = p.sequence_number;
                    found_start = true;
                }
            }
        }

        let mut iterations = 0;
        while count > 0 && iterations < self.n_send_max as usize {
            if let Some(p) = self.retransmission.get_packet(current_seq) {
                let len = p.payload_len;
                if len > 256 {
                    return Err(ConnectionError::InvalidPayloadSize);
                }

                let mut temp_payload = [0u8; 256];
                {
                    let dst = temp_payload
                        .get_mut(..len)
                        .ok_or(ConnectionError::InvalidPayloadSize)?;
                    let src = p
                        .payload
                        .get(..len)
                        .ok_or(ConnectionError::InvalidPayloadSize)?;
                    dst.copy_from_slice(src);
                }

                self.send_packet(
                    PacketType::RetransmissionData,
                    temp_payload
                        .get(..len)
                        .ok_or(ConnectionError::InvalidPayloadSize)?,
                )?;
                count -= 1;
            } else if iterations == 0 {
                return Err(ConnectionError::RetransmissionUnavailable);
            }
            current_seq = current_seq.wrapping_add(1);
            iterations += 1;
        }
        Ok(())
    }

    pub fn send_packet(
        &mut self,
        p_type: PacketType,
        payload: &[u8],
    ) -> Result<(), ConnectionError> {
        if payload.len() > Packet::MAX_PAYLOAD_SIZE {
            return Err(ConnectionError::InvalidPayloadSize);
        }

        let mut packet = Packet {
            packet_type: p_type,
            receiver_id: self.remote_id,
            sender_id: self.sender_id,
            sequence_number: self.sequence.next_tx(),
            confirmed_sequence_number: self.sequence.last_received_seq().unwrap_or_default(),
            timestamp: self.clock.now_ms(),
            confirmed_timestamp: self.last_received_timestamp,
            payload: [0; 256],
            payload_len: payload.len(),
        };

        if !payload.is_empty() {
            let dst = packet
                .payload
                .get_mut(..payload.len())
                .ok_or(ConnectionError::InvalidPayloadSize)?;
            dst.copy_from_slice(payload);
        }

        if p_type == PacketType::Data
            && self.retransmission.count()
                >= usize::from(self.n_send_max.min(self.remote_n_send_max))
        {
            return Err(ConnectionError::BufferFull);
        }

        let size = packet
            .serialize(&mut self.tx_buffer, &self.safety_code)
            .map_err(ConnectionError::Packet)?;
        let tx_slice = self
            .tx_buffer
            .get(..size)
            .ok_or(ConnectionError::BufferFull)?;
        self.redundancy
            .send(tx_slice)
            .map_err(ConnectionError::Transport)?;
        self.last_tx_sequence = Some(packet.sequence_number);

        if p_type == PacketType::Data && !self.retransmission.store(packet) {
            return Err(ConnectionError::BufferFull);
        }
        Ok(())
    }

    pub fn send_application_data(&mut self, data: &[u8]) -> Result<(), ConnectionError> {
        if data.len() > Packet::MAX_PAYLOAD_SIZE - 2 {
            return Err(ConnectionError::InvalidPayloadSize);
        }
        let mut payload = [0u8; Packet::MAX_PAYLOAD_SIZE];
        payload[..2].copy_from_slice(&(data.len() as u16).to_le_bytes());
        payload[2..2 + data.len()].copy_from_slice(data);
        let payload_len = data.len() + 2;
        if self.can_send_data() {
            self.send_packet(PacketType::Data, &payload[..payload_len])
        } else {
            self.enqueue_application_tx(&payload[..payload_len])
        }
    }

    fn send_disconnect(&mut self, reason: DisconnectReason) -> Result<(), ConnectionError> {
        let mut payload = [0u8; 4];
        payload[2..4].copy_from_slice(&reason.code().to_le_bytes());
        self.send_packet(PacketType::DisconnectionRequest, &payload)
    }

    fn reject_unexpected_packet(&mut self) -> Result<(), ConnectionError> {
        self.record_diagnostic(DiagnosticKind::UnexpectedMessage, 1);
        self.disconnect_with_reason(DisconnectReason::UnexpectedMessageForState)?;
        Err(ConnectionError::UnexpectedPacket)
    }

    fn apply_confirmation(&mut self, confirmed: u32) -> Result<(), ConnectionError> {
        if let Some(previous) = self.last_confirmed_by_peer
            && serial::is_before(confirmed, previous)
        {
            self.record_diagnostic(DiagnosticKind::ConfirmedSequenceError, confirmed);
            self.disconnect_with_error()?;
            return Err(ConnectionError::ProtocolViolation);
        }
        if let Some(last_sent) = self.last_tx_sequence
            && serial::is_after(confirmed, last_sent)
        {
            self.record_diagnostic(DiagnosticKind::ConfirmedSequenceError, confirmed);
            self.disconnect_with_error()?;
            return Err(ConnectionError::ProtocolViolation);
        }
        self.last_confirmed_by_peer = Some(confirmed);
        self.retransmission.clear_up_to(confirmed);
        Ok(())
    }

    fn start_timeliness_monitor(&mut self, now_ms: u32) {
        self.confirmed_timestamp_reference = Some(now_ms);
        self.timeliness_deadline_ms = Some(now_ms.wrapping_add(self.t_max));
    }

    fn stop_timeliness_monitor(&mut self) {
        self.confirmed_timestamp_reference = None;
        self.timeliness_deadline_ms = None;
    }

    fn apply_timeliness(&mut self, packet: &Packet, now_ms: u32) -> Result<(), ConnectionError> {
        if !matches!(
            packet.packet_type,
            PacketType::Heartbeat | PacketType::Data | PacketType::RetransmissionData
        ) {
            return Ok(());
        }

        let reference = self.confirmed_timestamp_reference.unwrap_or(now_ms);
        let confirmed_distance = packet.confirmed_timestamp.wrapping_sub(reference);
        if confirmed_distance >= self.t_max {
            self.record_diagnostic(
                DiagnosticKind::ConfirmedTimestampError,
                packet.confirmed_timestamp,
            );
            self.disconnect_with_error()?;
            return Err(ConnectionError::SafetyTimeout);
        }

        let round_trip_ms = now_ms.wrapping_sub(packet.confirmed_timestamp);
        if round_trip_ms > self.t_max {
            self.record_diagnostic(DiagnosticKind::ConnectionTimeout, round_trip_ms);
            self.disconnect_with_error()?;
            return Err(ConnectionError::SafetyTimeout);
        }

        if packet.confirmed_timestamp == reference
            || serial::is_after(packet.confirmed_timestamp, reference)
        {
            self.confirmed_timestamp_reference = Some(packet.confirmed_timestamp);
            self.timeliness_deadline_ms = Some(now_ms.wrapping_add(self.t_max - round_trip_ms));
        }
        Ok(())
    }

    pub fn receive_data(&mut self, output: &mut [u8]) -> Result<usize, ConnectionError> {
        if self.app_rx_count == 0 {
            return Err(ConnectionError::ReceiveQueueEmpty);
        }
        let len = *self
            .app_rx_len
            .get(self.app_rx_head)
            .ok_or(ConnectionError::InvalidPayloadSize)?;
        if output.len() < len {
            return Err(ConnectionError::BufferFull);
        }
        let src = self
            .app_rx_buffer
            .get(self.app_rx_head)
            .and_then(|b| b.get(..len))
            .ok_or(ConnectionError::InvalidPayloadSize)?;
        let dst = output
            .get_mut(..len)
            .ok_or(ConnectionError::InvalidPayloadSize)?;
        dst.copy_from_slice(src);
        self.app_rx_head = (self.app_rx_head + 1) % self.app_rx_buffer.len();
        self.app_rx_count -= 1;
        Ok(len)
    }

    pub fn has_received_data(&self) -> bool {
        self.app_rx_count > 0
    }

    pub fn take_diagnostic(&mut self) -> Option<DiagnosticEvent> {
        self.diagnostics.pop()
    }

    pub fn diagnostic_overflow_count(&self) -> u32 {
        self.diagnostic_overflow_count
    }

    pub fn error_counters(&self) -> SrlErrorCounters {
        self.error_counters
    }

    fn record_diagnostic(&mut self, kind: DiagnosticKind, value: u32) {
        if kind == DiagnosticKind::SafetyCodeError {
            self.error_counters.safety = self.error_counters.safety.saturating_add(1);
        }
        if kind == DiagnosticKind::ConfirmedSequenceError {
            self.error_counters.confirmed_sequence_number = self
                .error_counters
                .confirmed_sequence_number
                .saturating_add(1);
        }
        if self.diagnostics.push(DiagnosticEvent { kind, value }) == Err(FixedQueueError::Full) {
            self.diagnostic_overflow_count = self.diagnostic_overflow_count.saturating_add(1);
        }
    }

    fn enqueue_application_data(&mut self, packet: &Packet) -> Result<(), ConnectionError> {
        if self.app_rx_count == self.app_rx_buffer.len() {
            return Err(ConnectionError::ReceiveQueueFull);
        }
        if packet.payload_len < 2 {
            return Err(ConnectionError::InvalidPayloadSize);
        }
        let len_bytes = packet
            .payload
            .get(0..2)
            .ok_or(ConnectionError::InvalidPayloadSize)?;
        let len = u16::from_le_bytes([
            *len_bytes
                .first()
                .ok_or(ConnectionError::InvalidPayloadSize)?,
            *len_bytes
                .get(1)
                .ok_or(ConnectionError::InvalidPayloadSize)?,
        ]) as usize;
        if len.checked_add(2) != Some(packet.payload_len) {
            return Err(ConnectionError::InvalidPayloadSize);
        }
        let dst = self
            .app_rx_buffer
            .get_mut(self.app_rx_tail)
            .and_then(|b| b.get_mut(..len))
            .ok_or(ConnectionError::InvalidPayloadSize)?;
        let src = packet
            .payload
            .get(2..2 + len)
            .ok_or(ConnectionError::InvalidPayloadSize)?;
        dst.copy_from_slice(src);
        if let Some(slot_len) = self.app_rx_len.get_mut(self.app_rx_tail) {
            *slot_len = len;
        }
        self.app_rx_tail = (self.app_rx_tail + 1) % self.app_rx_buffer.len();
        self.app_rx_count += 1;
        Ok(())
    }

    fn can_send_data(&self) -> bool {
        self.retransmission.count() < usize::from(self.n_send_max.min(self.remote_n_send_max))
    }

    fn enqueue_application_tx(&mut self, payload: &[u8]) -> Result<(), ConnectionError> {
        if self.app_tx_count == self.app_tx_buffer.len() {
            return Err(ConnectionError::TransmitQueueFull);
        }
        let destination = self
            .app_tx_buffer
            .get_mut(self.app_tx_tail)
            .and_then(|slot| slot.get_mut(..payload.len()))
            .ok_or(ConnectionError::InvalidPayloadSize)?;
        destination.copy_from_slice(payload);
        *self
            .app_tx_len
            .get_mut(self.app_tx_tail)
            .ok_or(ConnectionError::InvalidPayloadSize)? = payload.len();
        self.app_tx_tail = (self.app_tx_tail + 1) % self.app_tx_buffer.len();
        self.app_tx_count += 1;
        Ok(())
    }

    fn flush_application_tx(&mut self) -> Result<(), ConnectionError> {
        while self.app_tx_count > 0 && self.can_send_data() {
            let length = *self
                .app_tx_len
                .get(self.app_tx_head)
                .ok_or(ConnectionError::InvalidPayloadSize)?;
            let mut payload = [0u8; Packet::MAX_PAYLOAD_SIZE];
            let source = self
                .app_tx_buffer
                .get(self.app_tx_head)
                .and_then(|slot| slot.get(..length))
                .ok_or(ConnectionError::InvalidPayloadSize)?;
            payload
                .get_mut(..length)
                .ok_or(ConnectionError::InvalidPayloadSize)?
                .copy_from_slice(source);
            self.send_packet(PacketType::Data, &payload[..length])?;
            self.app_tx_head = (self.app_tx_head + 1) % self.app_tx_buffer.len();
            self.app_tx_count -= 1;
        }
        Ok(())
    }

    fn connection_payload(&self) -> [u8; 14] {
        let mut payload = [0u8; 14];
        payload[0] = b'0';
        payload[1] = b'3';
        payload[2] = b'0';
        payload[3] = b'3';
        payload[4..6].copy_from_slice(&self.n_send_max.to_le_bytes());
        payload
    }

    fn apply_connection_payload(&mut self, packet: &Packet) -> Result<(), ConnectionError> {
        if packet.payload_len != 14
            || packet.payload.get(0..4) != Some(b"0303")
            || packet.payload.get(6..14) != Some(&[0; 8])
        {
            return Err(ConnectionError::ProtocolViolation);
        }
        let nsend = u16::from_le_bytes([
            *packet
                .payload
                .get(4)
                .ok_or(ConnectionError::InvalidPayloadSize)?,
            *packet
                .payload
                .get(5)
                .ok_or(ConnectionError::InvalidPayloadSize)?,
        ]);
        if nsend == 0 || nsend > 20 {
            return Err(ConnectionError::ProtocolViolation);
        }
        self.remote_n_send_max = nsend;
        Ok(())
    }
}

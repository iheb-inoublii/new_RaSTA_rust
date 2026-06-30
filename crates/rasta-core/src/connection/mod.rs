pub mod heartbeat;
pub mod pdu;
pub mod retransmission;
pub mod safety_code;
pub mod sequencing;
pub mod state_machine;
pub mod time_supervision;

pub use crate::config::RastaConfig;

use crate::connection::heartbeat::HeartbeatHandler;
use crate::connection::pdu::{Packet, PacketError, PacketType};
use crate::connection::retransmission::RetransmissionBuffer;
use crate::connection::safety_code::SafetyCodeConfig;
use crate::connection::sequencing::{SequenceHandler, SequenceResult};
use crate::connection::state_machine::{RastaState, StateMachine};
use crate::connection::time_supervision::{
    ConfirmedTimestampDecision, TimeSupervisionError, TimeSupervisor,
};
use crate::port::{RandomError, RandomSource, Transport, TransportError};
use crate::redundancy::{ChannelStatus, RedundancyLayer};
use crate::serial;
use crate::srl::DisconnectReason;
use crate::time::{
    DurationMs, MonotonicClock, MonotonicInstant, ProtocolTimestamp, ProtocolTimestampSource,
};
use crate::{
    queue::{FixedQueue, FixedQueueError},
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

pub struct RastaConnection<
    T1: Transport,
    T2: Transport,
    C: MonotonicClock + ProtocolTimestampSource,
> {
    pub state_machine: StateMachine,
    pub redundancy: RedundancyLayer<T1, T2>,
    pub clock: C,
    pub heartbeat: HeartbeatHandler,
    pub sequence: SequenceHandler,
    pub retransmission: RetransmissionBuffer,
    pub sender_id: u32,
    pub remote_id: u32,
    is_client: bool,
    pub safety_code: SafetyCodeConfig,
    pub t_max: u32,
    t_max_duration: DurationMs,
    pub n_send_max: u16,
    pub mwa: u16,
    remote_n_send_max: u16,
    initial_tx_sequence: u32,
    last_tx_sequence: Option<u32>,
    last_confirmed_by_peer: Option<u32>,
    received_since_ack: u16,
    last_received_timestamp: ProtocolTimestamp,
    confirmed_timestamp_reference: Option<ProtocolTimestamp>,
    timeliness_deadline: Option<MonotonicInstant>,
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
    last_channel_statuses: [ChannelStatus; 2],
}

impl<T1: Transport, T2: Transport, C: MonotonicClock + ProtocolTimestampSource>
    RastaConnection<T1, T2, C>
{
    pub fn try_new(
        transport_a: T1,
        transport_b: T2,
        clock: C,
        config: RastaConfig,
    ) -> Result<Self, ConnectionError> {
        if config.sender_id == 0
            || config.remote_id == 0
            || config.sender_id == config.remote_id
            || config.t_max == 0
            || config.heartbeat_interval_ms == 0
            || !DurationMs::from_millis(config.t_max).is_unambiguous_timeout()
            || !DurationMs::from_millis(config.heartbeat_interval_ms).is_unambiguous_timeout()
            || !DurationMs::from_millis(config.redundancy.t_seq_ms).is_unambiguous_timeout()
            || config.n_send_max == 0
            || config.n_send_max > 20
            || config.mwa == 0
            || config.mwa >= config.n_send_max
            || config.redundancy.t_seq_ms == 0
            || matches!(
                config.safety_code.mode,
                crate::connection::safety_code::SafetyCodeMode::None
            )
            || matches!(
                config.redundancy.check_code,
                crate::redundancy::RedundancyCheckCode::None
            )
        {
            return Err(ConnectionError::InvalidConfiguration);
        }
        Ok(RastaConnection {
            state_machine: StateMachine::new(),
            redundancy: RedundancyLayer::with_config(transport_a, transport_b, config.redundancy),
            clock,
            heartbeat: HeartbeatHandler::new(DurationMs::from_millis(config.heartbeat_interval_ms)),
            sequence: SequenceHandler::with_initial_tx(config.initial_seq),
            retransmission: RetransmissionBuffer::with_capacity(config.n_send_max as usize),
            sender_id: config.sender_id,
            remote_id: config.remote_id,
            is_client: config.sender_id < config.remote_id,
            safety_code: config.safety_code,
            t_max: config.t_max,
            t_max_duration: DurationMs::from_millis(config.t_max),
            n_send_max: config.n_send_max,
            mwa: config.mwa,
            remote_n_send_max: config.n_send_max,
            initial_tx_sequence: config.initial_seq,
            last_tx_sequence: None,
            last_confirmed_by_peer: None,
            received_since_ack: 0,
            last_received_timestamp: ProtocolTimestamp::from_wire_millis(0),
            confirmed_timestamp_reference: None,
            timeliness_deadline: None,
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
            last_channel_statuses: [ChannelStatus::Unknown; 2],
        })
    }

    pub fn try_new_with_random<R: RandomSource>(
        transport_a: T1,
        transport_b: T2,
        clock: C,
        mut config: RastaConfig,
        random: &mut R,
    ) -> Result<Self, ConnectionError> {
        config.initial_seq = random.next_u32().map_err(ConnectionError::Random)?;
        Self::try_new(transport_a, transport_b, clock, config)
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
        // DIN 5.5.12: the lower identification is the client.
        if self.sender_id > self.remote_id {
            return Ok(());
        }
        self.transition(RastaState::Start)?;
        self.start_timeliness_monitor(self.clock.now());
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
            self.heartbeat.stop();
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
            self.heartbeat.stop();
        }
        send_result
    }

    pub fn process(&mut self) -> Result<(), ConnectionError> {
        let local_now = self.clock.now();
        let local_timestamp = self.clock.protocol_timestamp();
        let heartbeat_due_at_poll_start = self.heartbeat.is_due(local_now);
        for _ in 0..32 {
            let receive_result = self.redundancy.receive_at(&mut self.rx_buffer, local_now);
            self.record_channel_status_transitions();
            let bytes_read = receive_result.map_err(ConnectionError::Transport)?;
            if bytes_read == 0 {
                break;
            }
            let rx_slice = self
                .rx_buffer
                .get(..bytes_read)
                .ok_or(ConnectionError::BufferFull)?;
            let parse_res = Packet::parse(rx_slice, &self.safety_code);
            match parse_res {
                Ok(packet) => self.handle_packet(packet, local_now, local_timestamp)?,
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

        if self
            .timeliness_deadline
            .is_some_and(|deadline| local_now.has_reached(deadline))
        {
            self.record_diagnostic(
                DiagnosticKind::ConnectionTimeout,
                local_now.wrapping_millis(),
            );
            self.disconnect_with_reason(DisconnectReason::IncomingMessageTimeout)?;
            return Err(ConnectionError::SafetyTimeout);
        }
        if heartbeat_due_at_poll_start && self.heartbeat_send_active() {
            self.send_packet(PacketType::Heartbeat, &[])?;
        }

        self.flush_application_tx()?;
        Ok(())
    }

    fn handle_packet(
        &mut self,
        packet: Packet,
        local_now: MonotonicInstant,
        local_timestamp: ProtocolTimestamp,
    ) -> Result<(), ConnectionError> {
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

        let timeliness_decision = self.validate_timeliness(&packet, local_timestamp)?;
        let peer_confirmation = self.validate_peer_confirmation(&packet)?;

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
                        self.record_diagnostic(
                            DiagnosticKind::SequenceError,
                            packet.sequence_number,
                        );
                        if self.state_machine.current_state == RastaState::Up {
                            self.transition(RastaState::RetransmissionRequested)?;
                            self.send_retransmission_request(expected)?;
                        }
                        return Ok(());
                    }
                    SequenceResult::Duplicate => return Ok(()),
                }
            }
        }

        self.apply_valid_confirmation(peer_confirmation);

        // DIN 5.5.6.1: copy the timestamp of every formally correct received
        // message into the next outbound PDU; only time-out related PDUs are
        // analysed by the adaptive monitor.
        self.last_received_timestamp = ProtocolTimestamp::from_wire_millis(packet.timestamp);
        self.refresh_receive_supervision(timeliness_decision, local_now);

        match self.state_machine.current_state {
            RastaState::Down if packet.packet_type == PacketType::ConnectionRequest => {
                self.start_timeliness_monitor(local_now);
                self.sequence.accept_initial_rx(packet.sequence_number);
                self.apply_connection_payload(&packet)?;
                self.transition(RastaState::Start)?;
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
                    self.enter_up()?;
                }
                PacketType::Heartbeat => {
                    if self.is_client {
                        return self.reject_unexpected_packet();
                    }
                    self.enter_up()?;
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
                        let requested_sequence = packet.confirmed_sequence_number.wrapping_add(1);
                        if !self.retransmission.contains(requested_sequence) {
                            self.record_diagnostic(
                                DiagnosticKind::RetransmissionFailure,
                                requested_sequence,
                            );
                            self.disconnect_with_reason(
                                DisconnectReason::RetransmissionUnavailable,
                            )?;
                            return Err(ConnectionError::RetransmissionUnavailable);
                        }
                        self.send_retransmission_response(packet.confirmed_sequence_number)?;
                        self.retransmit_from(requested_sequence)?;
                        // DIN 5.5.11: a regular message terminates retransmission.
                        self.send_packet(PacketType::Heartbeat, &[])?;
                    }
                    PacketType::RetransmissionResponse => {
                        if self.state_machine.current_state == RastaState::RetransmissionRequested {
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
        let mut current_seq: u32 = start_seq;
        let mut sent = 0usize;

        if self.retransmission.count() == 0 || !self.retransmission.contains(start_seq) {
            return Err(ConnectionError::RetransmissionUnavailable);
        }

        let mut iterations = 0;
        while iterations < self.n_send_max as usize {
            if let Some(p) = self.retransmission.get_packet(current_seq) {
                let packet = p.clone();
                self.send_retransmission_data(packet)?;
                sent += 1;
            } else if sent > 0 {
                break;
            } else {
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
            timestamp: self.clock.protocol_timestamp().wire_millis(),
            confirmed_timestamp: self.last_received_timestamp.wire_millis(),
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
        let send_result = self.redundancy.send_at(tx_slice, self.clock.now());
        self.record_channel_status_transitions();
        send_result.map_err(ConnectionError::Transport)?;
        if self.heartbeat_send_active() {
            self.restart_send_heartbeat_timer(self.clock.now());
        }
        self.last_tx_sequence = Some(packet.sequence_number);

        if p_type == PacketType::Data && !self.retransmission.store(packet) {
            return Err(ConnectionError::BufferFull);
        }
        Ok(())
    }

    fn send_retransmission_request(
        &mut self,
        expected_sequence: u32,
    ) -> Result<(), ConnectionError> {
        let sequence_number = self.sequence.next_tx();
        self.send_packet_with_fields(
            PacketType::RetransmissionRequest,
            &[],
            sequence_number,
            expected_sequence.wrapping_sub(1),
            true,
        )
    }

    fn send_retransmission_response(
        &mut self,
        confirmed_sequence: u32,
    ) -> Result<(), ConnectionError> {
        self.send_packet_with_fields(
            PacketType::RetransmissionResponse,
            &[],
            confirmed_sequence,
            self.sequence.last_received_seq().unwrap_or_default(),
            false,
        )
    }

    fn send_retransmission_data(&mut self, mut packet: Packet) -> Result<(), ConnectionError> {
        packet.packet_type = PacketType::RetransmissionData;
        packet.confirmed_sequence_number = self.sequence.last_received_seq().unwrap_or_default();
        packet.timestamp = self.clock.protocol_timestamp().wire_millis();
        packet.confirmed_timestamp = self.last_received_timestamp.wire_millis();
        let size = packet
            .serialize(&mut self.tx_buffer, &self.safety_code)
            .map_err(ConnectionError::Packet)?;
        let tx_slice = self
            .tx_buffer
            .get(..size)
            .ok_or(ConnectionError::BufferFull)?;
        let send_result = self.redundancy.send_at(tx_slice, self.clock.now());
        self.record_channel_status_transitions();
        send_result.map_err(ConnectionError::Transport)?;
        if self.heartbeat_send_active() {
            self.restart_send_heartbeat_timer(self.clock.now());
        }
        Ok(())
    }

    fn send_packet_with_fields(
        &mut self,
        p_type: PacketType,
        payload: &[u8],
        sequence_number: u32,
        confirmed_sequence_number: u32,
        update_last_tx: bool,
    ) -> Result<(), ConnectionError> {
        if payload.len() > Packet::MAX_PAYLOAD_SIZE {
            return Err(ConnectionError::InvalidPayloadSize);
        }
        let mut packet = Packet {
            packet_type: p_type,
            receiver_id: self.remote_id,
            sender_id: self.sender_id,
            sequence_number,
            confirmed_sequence_number,
            timestamp: self.clock.protocol_timestamp().wire_millis(),
            confirmed_timestamp: self.last_received_timestamp.wire_millis(),
            payload: [0; 256],
            payload_len: payload.len(),
        };
        if !payload.is_empty() {
            packet
                .payload
                .get_mut(..payload.len())
                .ok_or(ConnectionError::InvalidPayloadSize)?
                .copy_from_slice(payload);
        }
        let size = packet
            .serialize(&mut self.tx_buffer, &self.safety_code)
            .map_err(ConnectionError::Packet)?;
        let tx_slice = self
            .tx_buffer
            .get(..size)
            .ok_or(ConnectionError::BufferFull)?;
        let send_result = self.redundancy.send_at(tx_slice, self.clock.now());
        self.record_channel_status_transitions();
        send_result.map_err(ConnectionError::Transport)?;
        if self.heartbeat_send_active() {
            self.restart_send_heartbeat_timer(self.clock.now());
        }
        if update_last_tx {
            self.last_tx_sequence = Some(sequence_number);
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

    fn validate_peer_confirmation(
        &mut self,
        packet: &Packet,
    ) -> Result<Option<u32>, ConnectionError> {
        if !Self::packet_carries_peer_confirmation(packet.packet_type) {
            return Ok(None);
        }

        let confirmed = packet.confirmed_sequence_number;
        let last_peer_confirmed_sequence = self
            .last_confirmed_by_peer
            .unwrap_or_else(|| self.initial_tx_sequence.wrapping_sub(1));

        let Some(newest_transmitted_sequence) = self.last_tx_sequence else {
            if confirmed == last_peer_confirmed_sequence {
                return Ok(Some(confirmed));
            }
            return self.reject_invalid_confirmation(confirmed);
        };

        let does_not_move_backwards = confirmed == last_peer_confirmed_sequence
            || serial::is_after(confirmed, last_peer_confirmed_sequence);
        let does_not_skip_beyond_newest = confirmed == newest_transmitted_sequence
            || serial::is_before(confirmed, newest_transmitted_sequence);

        if does_not_move_backwards && does_not_skip_beyond_newest {
            Ok(Some(confirmed))
        } else {
            self.reject_invalid_confirmation(confirmed)
        }
    }

    fn packet_carries_peer_confirmation(packet_type: PacketType) -> bool {
        !matches!(
            packet_type,
            PacketType::ConnectionRequest | PacketType::RetransmissionRequest
        )
    }

    fn reject_invalid_confirmation<T>(&mut self, confirmed: u32) -> Result<T, ConnectionError> {
        self.record_diagnostic(DiagnosticKind::ConfirmedSequenceError, confirmed);
        self.disconnect_with_error()?;
        Err(ConnectionError::ProtocolViolation)
    }

    fn apply_valid_confirmation(&mut self, confirmed: Option<u32>) {
        if let Some(confirmed) = confirmed {
            self.last_confirmed_by_peer = Some(confirmed);
            self.retransmission.clear_up_to(confirmed);
        }
    }

    #[cfg(test)]
    pub(crate) fn last_peer_confirmed_sequence_for_test(&self) -> Option<u32> {
        self.last_confirmed_by_peer
    }

    #[cfg(test)]
    pub(crate) fn queued_application_tx_count_for_test(&self) -> usize {
        self.app_tx_count
    }

    #[cfg(test)]
    pub(crate) fn timeliness_deadline_for_test(&self) -> Option<MonotonicInstant> {
        self.timeliness_deadline
    }

    fn start_timeliness_monitor(&mut self, now: MonotonicInstant) {
        self.confirmed_timestamp_reference = Some(self.clock.protocol_timestamp());
        self.timeliness_deadline = Some(now.deadline_after(self.t_max_duration));
    }

    fn enter_up(&mut self) -> Result<(), ConnectionError> {
        self.transition(RastaState::Up)?;
        self.error_counters.reset();
        self.send_packet(PacketType::Heartbeat, &[])
    }

    fn stop_timeliness_monitor(&mut self) {
        self.confirmed_timestamp_reference = None;
        self.timeliness_deadline = None;
    }

    fn validate_timeliness(
        &mut self,
        packet: &Packet,
        local_timestamp: ProtocolTimestamp,
    ) -> Result<Option<ConfirmedTimestampDecision>, ConnectionError> {
        if !matches!(
            packet.packet_type,
            PacketType::Heartbeat | PacketType::Data | PacketType::RetransmissionData
        ) {
            return Ok(None);
        }

        let supervisor = TimeSupervisor {
            t_max: self.t_max_duration,
            future_tolerance: DurationMs::from_millis(TimeSupervisor::DEFAULT_FUTURE_TOLERANCE_MS),
        };
        let remote_timestamp = ProtocolTimestamp::from_wire_millis(packet.timestamp);
        if let Err(error) = supervisor.validate(local_timestamp, remote_timestamp) {
            self.handle_time_supervision_error(error, packet.timestamp)?;
            return Err(ConnectionError::SafetyTimeout);
        }

        let reference = self
            .confirmed_timestamp_reference
            .unwrap_or(local_timestamp);
        let confirmed_timestamp = ProtocolTimestamp::from_wire_millis(packet.confirmed_timestamp);
        match supervisor.validate_confirmed_timestamp(
            local_timestamp,
            reference,
            confirmed_timestamp,
        ) {
            Ok(decision) => Ok(Some(decision)),
            Err(error) => {
                self.handle_time_supervision_error(error, packet.confirmed_timestamp)?;
                Err(ConnectionError::SafetyTimeout)
            }
        }
    }

    fn handle_time_supervision_error(
        &mut self,
        error: TimeSupervisionError,
        value: u32,
    ) -> Result<(), ConnectionError> {
        match error {
            TimeSupervisionError::TimestampTooOld
            | TimeSupervisionError::TimestampTooFarInFuture => {
                self.record_diagnostic(DiagnosticKind::ConnectionTimeout, value);
                self.disconnect_with_reason(DisconnectReason::IncomingMessageTimeout)?;
            }
            TimeSupervisionError::ConfirmedTimestampMovedBackwards
            | TimeSupervisionError::ConfirmedTimestampTooFarInFuture => {
                self.record_diagnostic(DiagnosticKind::ConfirmedTimestampError, value);
                self.disconnect_with_error()?;
            }
        }
        Ok(())
    }

    fn refresh_receive_supervision(
        &mut self,
        decision: Option<ConfirmedTimestampDecision>,
        now: MonotonicInstant,
    ) {
        let Some(decision) = decision else {
            return;
        };
        self.confirmed_timestamp_reference = Some(decision.confirmed_timestamp);
        self.timeliness_deadline = Some(now.deadline_after(self.t_max_duration));
    }

    fn restart_send_heartbeat_timer(&mut self, now: MonotonicInstant) {
        self.heartbeat.restart(now);
    }

    fn heartbeat_send_active(&self) -> bool {
        matches!(
            self.state_machine.current_state,
            RastaState::Up
                | RastaState::RetransmissionRequested
                | RastaState::RetransmissionRunning
        )
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

    pub fn channel_statuses(&self) -> [ChannelStatus; 2] {
        self.redundancy.channel_statuses()
    }

    fn record_channel_status_transitions(&mut self) {
        let statuses = self.redundancy.channel_statuses();
        for (index, status) in statuses.iter().enumerate() {
            if *status != self.last_channel_statuses[index] {
                if matches!(status, ChannelStatus::Degraded | ChannelStatus::Failed) {
                    self.record_diagnostic(DiagnosticKind::ChannelSupervisionFailure, index as u32);
                }
                self.last_channel_statuses[index] = *status;
            }
        }
    }

    fn record_diagnostic(&mut self, kind: DiagnosticKind, value: u32) {
        if kind == DiagnosticKind::SafetyCodeError {
            self.error_counters.safety = self.error_counters.safety.saturating_add(1);
        }
        if kind == DiagnosticKind::SequenceError {
            self.error_counters.sequence_number =
                self.error_counters.sequence_number.saturating_add(1);
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

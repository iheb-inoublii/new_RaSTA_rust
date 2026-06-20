use crate::core::connection_state_machine::{RastaState, StateMachine};
use crate::core::heartbeat::HeartbeatHandler;
use crate::core::pdu::{Packet, PacketError, PacketType};
use crate::core::redundancy_management::{RedundancyConfig, RedundancyLayer};
use crate::core::retransmission::RetransmissionBuffer;
use crate::core::safety_code::SafetyCodeConfig;
use crate::core::sequencing::{SequenceHandler, SequenceResult};
use crate::core::time_supervision::TimeSupervisor;
use crate::platform::clock::Clock;
use crate::platform::timer::Timer;
use crate::platform::transport::{Transport, TransportError};

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
    pub safety_code: SafetyCodeConfig,
    pub t_max: u32,
    pub n_send_max: u16,
    remote_n_send_max: u16,
    last_received_timestamp: u32,
    remote_clock_offset: Option<u32>,
    rx_buffer: [u8; 512],
    tx_buffer: [u8; 512],
    app_rx_buffer: [[u8; 256]; 8],
    app_rx_len: [usize; 8],
    app_rx_head: usize,
    app_rx_tail: usize,
    app_rx_count: usize,
}

impl<T1: Transport, T2: Transport, TimerCtx: Timer, C: Clock> RastaConnection<T1, T2, TimerCtx, C> {
    pub fn new(
        transport_a: T1,
        transport_b: T2,
        timer: TimerCtx,
        clock: C,
        config: RastaConfig,
    ) -> Self {
        RastaConnection {
            state_machine: StateMachine::new(),
            redundancy: RedundancyLayer::with_config(transport_a, transport_b, config.redundancy),
            clock,
            heartbeat: HeartbeatHandler::new(timer, config.heartbeat_interval_ms),
            sequence: SequenceHandler::with_initial_tx(config.initial_seq),
            retransmission: RetransmissionBuffer::new(),
            sender_id: config.sender_id,
            remote_id: config.remote_id,
            safety_code: config.safety_code,
            t_max: config.t_max,
            n_send_max: config.n_send_max,
            remote_n_send_max: config.n_send_max,
            last_received_timestamp: 0,
            remote_clock_offset: None,
            rx_buffer: [0; 512],
            tx_buffer: [0; 512],
            app_rx_buffer: [[0; 256]; 8],
            app_rx_len: [0; 8],
            app_rx_head: 0,
            app_rx_tail: 0,
            app_rx_count: 0,
        }
    }

    pub fn transition(&mut self, new_state: RastaState) -> Result<(), ConnectionError> {
        if self.state_machine.transition(new_state) {
            Ok(())
        } else {
            Err(ConnectionError::StateTransitionInvalid)
        }
    }

    pub fn connect(&mut self) -> Result<(), ConnectionError> {
        if self.state_machine.current_state != RastaState::Down {
            return Err(ConnectionError::ProtocolViolation);
        }
        self.transition(RastaState::Start)?;
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
        if self.state_machine.current_state == RastaState::Up
            || self.state_machine.current_state == RastaState::Retransmission
        {
            let _ = self.send_packet(PacketType::DisconnectionRequest, &[]);
            self.transition(RastaState::Closed)?;
        }
        self.transition(RastaState::Down)?;
        Ok(())
    }

    pub fn disconnect_with_error(&mut self) -> Result<(), ConnectionError> {
        let _ = self.send_packet(PacketType::DisconnectionRequest, &[]);
        let _ = self.transition(RastaState::Closed);
        let _ = self.transition(RastaState::Down);
        Ok(())
    }

    pub fn process(&mut self) -> Result<(), ConnectionError> {
        if self.heartbeat.is_due() {
            if self.state_machine.current_state == RastaState::Up {
                self.send_packet(PacketType::Heartbeat, &[])?;
            } else if self.state_machine.current_state != RastaState::Down
                && self.state_machine.current_state != RastaState::Closed
            {
                return self.disconnect_with_error();
            }
            self.heartbeat.reset();
        }

        let read_res = self.redundancy.receive(&mut self.rx_buffer);
        let bytes_read = match read_res {
            Ok(b) => b,
            Err(TransportError::ReceiveFailed) => 0,
            Err(e) => return Err(ConnectionError::Transport(e)),
        };

        if bytes_read > 0 {
            let rx_slice = self
                .rx_buffer
                .get(..bytes_read)
                .ok_or(ConnectionError::BufferFull)?;
            let parse_res = Packet::parse(rx_slice, &self.safety_code);
            match parse_res {
                Ok(packet) => self.handle_packet(packet)?,
                Err(PacketError::ChecksumMismatch) => {
                    return self.disconnect_with_error();
                }
                Err(e) => return Err(ConnectionError::Packet(e)),
            }
        }

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
            if packet.receiver_id != 0 && packet.receiver_id != self.sender_id {
                // Receiver ID in ConnectionRequest can be 0 or local sender_id
                return Err(ConnectionError::ProtocolViolation);
            }
        }

        let local_now = self.clock.now_ms();
        let remote_ts_local = if let Some(offset) = self.remote_clock_offset {
            packet.timestamp.wrapping_add(offset)
        } else {
            self.remote_clock_offset = Some(local_now.wrapping_sub(packet.timestamp));
            local_now
        };

        let time_supervisor = TimeSupervisor::new(self.t_max);
        if time_supervisor
            .validate(local_now, remote_ts_local)
            .is_err()
        {
            return self.disconnect_with_error();
        }

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
                    return Err(ConnectionError::ProtocolViolation);
                }
                match self.sequence.validate_rx(packet.sequence_number) {
                    SequenceResult::Ok => {}
                    SequenceResult::Gap(expected) => {
                        self.transition(RastaState::Retransmission)?;
                        let payload = expected.to_le_bytes();
                        self.send_packet(PacketType::RetransmissionRequest, &payload)?;
                        return Ok(());
                    }
                    SequenceResult::Duplicate => return Ok(()),
                }
            }
        }

        self.last_received_timestamp = packet.timestamp;
        self.retransmission
            .clear_up_to(packet.confirmed_sequence_number);

        match self.state_machine.current_state {
            RastaState::Down if packet.packet_type == PacketType::ConnectionRequest => {
                self.remote_id = packet.sender_id;
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
            RastaState::Start => {
                match packet.packet_type {
                    PacketType::ConnectionResponse => {
                        self.sequence.accept_initial_rx(packet.sequence_number);
                        self.apply_connection_payload(&packet)?;
                        self.transition(RastaState::Up)?;
                        self.send_packet(PacketType::Heartbeat, &[])?;
                    }
                    PacketType::Heartbeat => {
                        self.transition(RastaState::Up)?;
                    }
                    PacketType::ConnectionRequest => {
                        // Duplicate request, ignore or re-send response
                    }
                    _ => {
                        return Err(ConnectionError::UnexpectedPacket);
                    }
                }
            }
            RastaState::Up | RastaState::Retransmission => {
                match packet.packet_type {
                    PacketType::RetransmissionRequest => {
                        if packet.payload_len != 4 {
                            return Err(ConnectionError::ProtocolViolation);
                        }
                        let seq_bytes = packet
                            .payload
                            .get(0..4)
                            .ok_or(ConnectionError::InvalidPayloadSize)?;
                        let requested_seq = u32::from_le_bytes([
                            *seq_bytes
                                .first()
                                .ok_or(ConnectionError::InvalidPayloadSize)?,
                            *seq_bytes
                                .get(1)
                                .ok_or(ConnectionError::InvalidPayloadSize)?,
                            *seq_bytes
                                .get(2)
                                .ok_or(ConnectionError::InvalidPayloadSize)?,
                            *seq_bytes
                                .get(3)
                                .ok_or(ConnectionError::InvalidPayloadSize)?,
                        ]);
                        self.retransmit_from(requested_seq)?;
                        self.send_packet(PacketType::RetransmissionResponse, &[])?;
                    }
                    PacketType::RetransmissionResponse => {
                        if self.state_machine.current_state == RastaState::Retransmission {
                            self.sequence.accept_initial_rx(packet.sequence_number);
                            self.transition(RastaState::Up)?;
                        }
                    }
                    PacketType::DisconnectionRequest => {
                        let _ = self.transition(RastaState::Closed);
                        let _ = self.transition(RastaState::Down);
                    }
                    PacketType::Data | PacketType::RetransmissionData => {
                        self.enqueue_application_data(&packet)?;
                    }
                    PacketType::Heartbeat => {
                        // Normal operation
                    }
                    _ => {
                        return Err(ConnectionError::UnexpectedPacket);
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
        while count > 0 && iterations < 32 {
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

        if p_type == PacketType::Data && !self.retransmission.store(packet) {
            return Err(ConnectionError::BufferFull);
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

    fn enqueue_application_data(&mut self, packet: &Packet) -> Result<(), ConnectionError> {
        if self.app_rx_count == self.app_rx_buffer.len() {
            return Err(ConnectionError::ReceiveQueueFull);
        }
        let len = packet.payload_len;
        let dst = self
            .app_rx_buffer
            .get_mut(self.app_rx_tail)
            .and_then(|b| b.get_mut(..len))
            .ok_or(ConnectionError::InvalidPayloadSize)?;
        let src = packet
            .payload
            .get(..len)
            .ok_or(ConnectionError::InvalidPayloadSize)?;
        dst.copy_from_slice(src);
        if let Some(slot_len) = self.app_rx_len.get_mut(self.app_rx_tail) {
            *slot_len = len;
        }
        self.app_rx_tail = (self.app_rx_tail + 1) % self.app_rx_buffer.len();
        self.app_rx_count += 1;
        Ok(())
    }

    fn connection_payload(&self) -> [u8; 14] {
        let mut payload = [0u8; 14];
        payload[0] = b'0';
        payload[1] = b'3';
        payload[2] = b'0';
        payload[3] = b'1';
        payload[4..6].copy_from_slice(&self.n_send_max.to_le_bytes());
        payload
    }

    fn apply_connection_payload(&mut self, packet: &Packet) -> Result<(), ConnectionError> {
        if packet.payload_len < 14 {
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
        self.remote_n_send_max = nsend;
        Ok(())
    }
}

//! Structured, fixed-capacity tracing types.

use crate::connection::TimestampTraceRejection;
use crate::connection::pdu::PacketType;
use crate::srl::DiagnosticEvent;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceDirection {
    Tx,
    Rx,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RastaPacketType {
    ConnectionRequest,
    ConnectionResponse,
    RetransmissionRequest,
    RetransmissionResponse,
    DisconnectionRequest,
    Heartbeat,
    Data,
    RetransmissionData,
    Unknown(u16),
}

impl From<PacketType> for RastaPacketType {
    fn from(value: PacketType) -> Self {
        match value {
            PacketType::ConnectionRequest => Self::ConnectionRequest,
            PacketType::ConnectionResponse => Self::ConnectionResponse,
            PacketType::RetransmissionRequest => Self::RetransmissionRequest,
            PacketType::RetransmissionResponse => Self::RetransmissionResponse,
            PacketType::DisconnectionRequest => Self::DisconnectionRequest,
            PacketType::Heartbeat => Self::Heartbeat,
            PacketType::Data => Self::Data,
            PacketType::RetransmissionData => Self::RetransmissionData,
        }
    }
}

impl From<u16> for RastaPacketType {
    fn from(value: u16) -> Self {
        PacketType::from_u16(value).map_or(Self::Unknown(value), Self::from)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PacketTrace {
    pub direction: TraceDirection,
    pub channel: u8,
    pub frame_len: usize,
    pub redundancy_sequence: u32,
    pub packet_type: Option<RastaPacketType>,
    pub receiver_id: Option<u32>,
    pub sender_id: Option<u32>,
    pub sequence_number: Option<u32>,
    pub confirmed_sequence_number: Option<u32>,
    pub timestamp: Option<u32>,
    pub confirmed_timestamp: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateTransitionTrace {
    pub from: RastaConnectionState,
    pub to: RastaConnectionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RastaConnectionState {
    Closed,
    Down,
    Start,
    Up,
    RetransmissionRequested,
    RetransmissionRunning,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimestampCompatibilityTrace {
    pub raw_peer_timestamp: u32,
    pub learned_peer_offset: Option<u32>,
    pub normalized_peer_timestamp: u32,
    pub local_timestamp: u32,
    pub local_receive_deadline: Option<u32>,
    pub confirmed_timestamp: u32,
    pub rejection: Option<TimestampTraceRejection>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RastaTraceEvent {
    Packet(PacketTrace),
    StateTransition(StateTransitionTrace),
    HeartbeatSent,
    HeartbeatReceived,
    ApplicationDataSent { len: usize },
    ApplicationDataReceived { len: usize },
    Diagnostic(DiagnosticEvent),
    Timeout,
    GracefulDisconnect,
    TimestampCompatibility(TimestampCompatibilityTrace),
    TraceOverflow { dropped_count: u32 },
}

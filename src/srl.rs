//! DIN RaSTA 03.03 Safety and Retransmission Layer foundations.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SrlState {
    Closed,
    Down,
    Start,
    Up,
    RetransmissionRequested,
    RetransmissionRunning,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisconnectReason {
    UserRequest,
    UnexpectedMessageForState,
    SequenceErrorDuringConnectionEstablishment,
    IncomingMessageTimeout,
    ServiceNotAllowedInState,
    ProtocolVersionError,
    RetransmissionUnavailable,
    ProtocolSequenceError,
    Unknown(u16),
}

impl DisconnectReason {
    // DIN VDE V 0831-200:2015-06, Table 7 (clauses 5.4.6).
    pub fn code(self) -> u16 {
        match self {
            Self::UserRequest => 0,
            Self::UnexpectedMessageForState => 2,
            Self::SequenceErrorDuringConnectionEstablishment => 3,
            Self::IncomingMessageTimeout => 4,
            Self::ServiceNotAllowedInState => 5,
            Self::ProtocolVersionError => 6,
            Self::RetransmissionUnavailable => 7,
            Self::ProtocolSequenceError => 8,
            Self::Unknown(code) => code,
        }
    }

    pub fn from_code(code: u16) -> Self {
        match code {
            0 => Self::UserRequest,
            2 => Self::UnexpectedMessageForState,
            3 => Self::SequenceErrorDuringConnectionEstablishment,
            4 => Self::IncomingMessageTimeout,
            5 => Self::ServiceNotAllowedInState,
            6 => Self::ProtocolVersionError,
            7 => Self::RetransmissionUnavailable,
            8 => Self::ProtocolSequenceError,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticKind {
    SafetyCodeError,
    RedundancyCheckError,
    SequenceError,
    ConfirmedSequenceError,
    ConfirmedTimestampError,
    ProtocolVersionError,
    MalformedMessage,
    UnexpectedMessage,
    RetransmissionFailure,
    FlowControlEvent,
    DeferQueueOverflow,
    ChannelSupervisionFailure,
    ConnectionTimeout,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagnosticEvent {
    pub kind: DiagnosticKind,
    pub value: u32,
}

/// DIN VDE V 0831-200:2015-06, clause 5.5.5 error counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SrlErrorCounters {
    pub safety: u32,
    pub address: u32,
    pub message_type: u32,
    pub sequence_number: u32,
    pub confirmed_sequence_number: u32,
}

impl SrlErrorCounters {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

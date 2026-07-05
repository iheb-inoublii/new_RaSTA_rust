//! High-level public endpoint API for applications.
//!
//! This module keeps packet encoding, redundancy management, retransmission,
//! state-machine details, and queue plumbing behind a small application-facing
//! interface.

use crate::config::{ConfigError, RastaConfig, RastaProfile, SafetyCodeLength};
use crate::connection::safety_code::{SafetyCodeConfig, SafetyCodeMode};
use crate::connection::{ConnectionError, TimestampTraceEvent};
use crate::port::{RandomError, RandomSource, RastaTransport};
use crate::redundancy::{RedundancyCheckCode, RedundancyConfig, RedundancyCrc};
use crate::service::RastaService;
use crate::srl::DiagnosticEvent;
use crate::time::{MonotonicClock, ProtocolTimestampSource};

pub use crate::service::ConnectionStatus;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RastaError {
    Transport,
    Packet,
    UnexpectedPacket,
    BufferFull,
    ProtocolViolation,
    SafetyTimeout,
    InvalidState,
    InvalidPayloadSize,
    ReceiveQueueEmpty,
    ReceiveQueueFull,
    TransmitQueueFull,
    InvalidConfiguration,
    RetransmissionUnavailable,
    Random(RandomError),
    Config(ConfigError),
    MissingLocalId,
    MissingRemoteId,
    MissingConfig,
}

impl From<ConnectionError> for RastaError {
    fn from(error: ConnectionError) -> Self {
        match error {
            ConnectionError::Transport(_) => Self::Transport,
            ConnectionError::Packet(_) => Self::Packet,
            ConnectionError::UnexpectedPacket => Self::UnexpectedPacket,
            ConnectionError::BufferFull => Self::BufferFull,
            ConnectionError::ProtocolViolation => Self::ProtocolViolation,
            ConnectionError::SafetyTimeout => Self::SafetyTimeout,
            ConnectionError::StateTransitionInvalid => Self::InvalidState,
            ConnectionError::InvalidPayloadSize => Self::InvalidPayloadSize,
            ConnectionError::ReceiveQueueEmpty => Self::ReceiveQueueEmpty,
            ConnectionError::ReceiveQueueFull => Self::ReceiveQueueFull,
            ConnectionError::TransmitQueueFull => Self::TransmitQueueFull,
            ConnectionError::InvalidConfiguration => Self::InvalidConfiguration,
            ConnectionError::RetransmissionUnavailable => Self::RetransmissionUnavailable,
            ConnectionError::Random(error) => Self::Random(error),
        }
    }
}

impl From<ConfigError> for RastaError {
    fn from(error: ConfigError) -> Self {
        Self::Config(error)
    }
}

pub struct RastaEndpoint<
    T1: RastaTransport,
    T2: RastaTransport,
    C: MonotonicClock + ProtocolTimestampSource,
> {
    service: RastaService<T1, T2, C>,
}

impl<T1: RastaTransport, T2: RastaTransport, C: MonotonicClock + ProtocolTimestampSource>
    RastaEndpoint<T1, T2, C>
{
    pub fn builder(transport_a: T1, transport_b: T2, clock: C) -> RastaEndpointBuilder<T1, T2, C> {
        RastaEndpointBuilder::new(transport_a, transport_b, clock)
    }

    pub fn from_config(
        transport_a: T1,
        transport_b: T2,
        clock: C,
        config: RastaConfig,
    ) -> Result<Self, RastaError> {
        Ok(Self {
            service: RastaService::new(transport_a, transport_b, clock, config)?,
        })
    }

    pub fn from_config_with_random<R: RandomSource>(
        transport_a: T1,
        transport_b: T2,
        clock: C,
        config: RastaConfig,
        random: &mut R,
    ) -> Result<Self, RastaError> {
        Ok(Self {
            service: RastaService::new_with_random(
                transport_a,
                transport_b,
                clock,
                config,
                random,
            )?,
        })
    }

    pub fn connect(&mut self) -> Result<(), RastaError> {
        self.service.open_connection().map_err(Into::into)
    }

    pub fn poll(&mut self) -> Result<(), RastaError> {
        self.service.poll().map_err(Into::into)
    }

    pub fn send(&mut self, data: &[u8]) -> Result<(), RastaError> {
        self.service.send_data(data).map_err(Into::into)
    }

    pub fn receive(&mut self, output: &mut [u8]) -> Result<usize, RastaError> {
        self.service.receive_data(output).map_err(Into::into)
    }

    pub fn has_received_data(&self) -> bool {
        self.service.has_received_data()
    }

    pub fn close(&mut self) -> Result<(), RastaError> {
        self.service.close_connection().map_err(Into::into)
    }

    pub fn status(&self) -> ConnectionStatus {
        self.service.status()
    }

    pub fn take_diagnostic(&mut self) -> Option<DiagnosticEvent> {
        self.service.take_diagnostic()
    }

    pub fn drain_diagnostics<F: FnMut(DiagnosticEvent)>(&mut self, mut on_event: F) {
        while let Some(event) = self.take_diagnostic() {
            on_event(event);
        }
    }

    pub fn take_trace_event(&mut self) -> Option<TimestampTraceEvent> {
        self.service.take_timestamp_trace()
    }

    pub fn drain_trace_events<F: FnMut(TimestampTraceEvent)>(&mut self, mut on_event: F) {
        while let Some(event) = self.take_trace_event() {
            on_event(event);
        }
    }
}

pub struct RastaEndpointBuilder<
    T1: RastaTransport,
    T2: RastaTransport,
    C: MonotonicClock + ProtocolTimestampSource,
> {
    transport_a: T1,
    transport_b: T2,
    clock: C,
    local_id: Option<u32>,
    remote_id: Option<u32>,
    config: Option<RastaConfig>,
}

impl<T1: RastaTransport, T2: RastaTransport, C: MonotonicClock + ProtocolTimestampSource>
    RastaEndpointBuilder<T1, T2, C>
{
    pub fn new(transport_a: T1, transport_b: T2, clock: C) -> Self {
        Self {
            transport_a,
            transport_b,
            clock,
            local_id: None,
            remote_id: None,
            config: None,
        }
    }

    pub fn local_id(mut self, local_id: u32) -> Self {
        self.local_id = Some(local_id);
        self
    }

    pub fn remote_id(mut self, remote_id: u32) -> Self {
        self.remote_id = Some(remote_id);
        self
    }

    pub fn config(mut self, config: RastaConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn profile(mut self, profile: RastaProfile) -> Result<Self, RastaError> {
        self.config = Some(config_from_profile(
            self.local_id.ok_or(RastaError::MissingLocalId)?,
            self.remote_id.ok_or(RastaError::MissingRemoteId)?,
            profile,
            false,
        )?);
        Ok(self)
    }

    pub fn unsafe_interop_profile(mut self, profile: RastaProfile) -> Result<Self, RastaError> {
        self.config = Some(config_from_profile(
            self.local_id.ok_or(RastaError::MissingLocalId)?,
            self.remote_id.ok_or(RastaError::MissingRemoteId)?,
            profile,
            true,
        )?);
        Ok(self)
    }

    pub fn build(self) -> Result<RastaEndpoint<T1, T2, C>, RastaError> {
        let config = self.validated_config()?;
        RastaEndpoint::from_config(self.transport_a, self.transport_b, self.clock, config)
    }

    pub fn build_with_random<R: RandomSource>(
        self,
        random: &mut R,
    ) -> Result<RastaEndpoint<T1, T2, C>, RastaError> {
        let config = self.validated_config()?;
        RastaEndpoint::from_config_with_random(
            self.transport_a,
            self.transport_b,
            self.clock,
            config,
            random,
        )
    }

    fn validated_config(&self) -> Result<RastaConfig, RastaError> {
        let local_id = self.local_id.ok_or(RastaError::MissingLocalId)?;
        let remote_id = self.remote_id.ok_or(RastaError::MissingRemoteId)?;
        let mut config = self.config.ok_or(RastaError::MissingConfig)?;
        config.sender_id = local_id;
        config.remote_id = remote_id;
        Ok(config)
    }
}

pub fn config_from_profile(
    local_id: u32,
    remote_id: u32,
    profile: RastaProfile,
    allow_unsafe_no_checksums: bool,
) -> Result<RastaConfig, RastaError> {
    if allow_unsafe_no_checksums {
        profile.validate_allowing_unsafe_no_checksums()?;
    } else {
        profile.validate()?;
    }
    Ok(RastaConfig {
        sender_id: local_id,
        remote_id,
        safety_code: safety_code_from_profile(profile),
        redundancy: RedundancyConfig {
            check_code: redundancy_check_code_from_profile(profile.redundancy_crc),
            t_seq_ms: profile.t_seq_ms,
        },
        t_max: profile.t_max_ms,
        initial_seq: 0,
        heartbeat_interval_ms: profile.t_h_ms,
        n_send_max: profile.n_send_max as u16,
        mwa: profile.mwa as u16,
        allow_unsafe_no_checksums,
        timestamp_compatibility: profile.timestamp_compatibility,
    })
}

fn safety_code_from_profile(profile: RastaProfile) -> SafetyCodeConfig {
    match profile.safety_code_length {
        SafetyCodeLength::None => SafetyCodeConfig::none(),
        SafetyCodeLength::Md4Lower8 => SafetyCodeConfig::md4_low8(profile.md4_initial_value),
        SafetyCodeLength::Md4Full16 => SafetyCodeConfig {
            mode: SafetyCodeMode::Md4Full16,
            md4_initial_value: profile.md4_initial_value,
        },
    }
}

fn redundancy_check_code_from_profile(crc: RedundancyCrc) -> RedundancyCheckCode {
    match crc {
        RedundancyCrc::OptionA => RedundancyCheckCode::OptionA,
        RedundancyCrc::OptionB => RedundancyCheckCode::OptionB,
        RedundancyCrc::OptionC => RedundancyCheckCode::OptionC,
        RedundancyCrc::OptionD => RedundancyCheckCode::OptionD,
        RedundancyCrc::OptionE => RedundancyCheckCode::OptionE,
    }
}

#[cfg(test)]
mod tests {
    use super::{RastaEndpoint, RastaError, config_from_profile};
    use crate::config::{RastaConfig, RastaProfile, TimestampCompatibilityMode};
    use crate::connection::safety_code::SafetyCodeConfig;
    use crate::port::{Transport, TransportError};
    use crate::redundancy::{RedundancyCheckCode, RedundancyConfig};
    use crate::service::ConnectionStatus;
    use crate::time::{
        MonotonicClock, MonotonicInstant, ProtocolTimestamp, ProtocolTimestampSource,
    };
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    #[derive(Clone)]
    struct FakeClock(Rc<Cell<u32>>);

    impl FakeClock {
        fn new(now: u32) -> Self {
            Self(Rc::new(Cell::new(now)))
        }
    }

    impl MonotonicClock for FakeClock {
        fn now(&self) -> MonotonicInstant {
            MonotonicInstant::from_wrapping_millis(self.0.get())
        }
    }

    impl ProtocolTimestampSource for FakeClock {
        fn protocol_timestamp(&self) -> ProtocolTimestamp {
            ProtocolTimestamp::from_wire_millis(self.0.get())
        }
    }

    #[derive(Clone, Copy)]
    struct Frame {
        bytes: [u8; 520],
        len: usize,
    }

    struct Network {
        frames: [[Option<Frame>; 32]; 4],
        head: [usize; 4],
        tail: [usize; 4],
        count: [usize; 4],
    }

    impl Network {
        fn new() -> Self {
            Self {
                frames: [[None; 32]; 4],
                head: [0; 4],
                tail: [0; 4],
                count: [0; 4],
            }
        }

        fn peer(channel: usize) -> usize {
            match channel {
                0 => 2,
                1 => 3,
                2 => 0,
                _ => 1,
            }
        }
    }

    #[derive(Clone)]
    struct LinkedTransport {
        network: Rc<RefCell<Network>>,
        channel: usize,
        fail_send: bool,
    }

    impl LinkedTransport {
        fn new(network: Rc<RefCell<Network>>, channel: usize) -> Self {
            Self {
                network,
                channel,
                fail_send: false,
            }
        }
    }

    impl Transport for LinkedTransport {
        fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            if self.fail_send {
                return Err(TransportError::SendFailed);
            }
            if data.len() > 520 {
                return Err(TransportError::BufferTooSmall);
            }
            let mut network = self.network.borrow_mut();
            let target = Network::peer(self.channel);
            if network.count[target] == 32 {
                return Err(TransportError::SendFailed);
            }
            let mut bytes = [0u8; 520];
            bytes[..data.len()].copy_from_slice(data);
            let tail = network.tail[target];
            network.frames[target][tail] = Some(Frame {
                bytes,
                len: data.len(),
            });
            network.tail[target] = (tail + 1) % 32;
            network.count[target] += 1;
            Ok(())
        }

        fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError> {
            let mut network = self.network.borrow_mut();
            if network.count[self.channel] == 0 {
                return Ok(0);
            }
            let head = network.head[self.channel];
            let frame = network.frames[self.channel][head]
                .take()
                .ok_or(TransportError::ReceiveFailed)?;
            if buffer.len() < frame.len {
                return Err(TransportError::BufferTooSmall);
            }
            buffer[..frame.len].copy_from_slice(&frame.bytes[..frame.len]);
            network.head[self.channel] = (head + 1) % 32;
            network.count[self.channel] -= 1;
            Ok(frame.len)
        }
    }

    fn config(sender_id: u32, remote_id: u32) -> RastaConfig {
        RastaConfig {
            sender_id,
            remote_id,
            safety_code: SafetyCodeConfig::default(),
            redundancy: RedundancyConfig {
                check_code: RedundancyCheckCode::OptionB,
                t_seq_ms: 100,
            },
            t_max: 2_000,
            initial_seq: 0,
            heartbeat_interval_ms: 500,
            n_send_max: 16,
            mwa: 8,
            allow_unsafe_no_checksums: false,
            timestamp_compatibility: TimestampCompatibilityMode::StrictSynchronized,
        }
    }

    type Endpoint = RastaEndpoint<LinkedTransport, LinkedTransport, FakeClock>;

    fn endpoint_pair() -> (Endpoint, Endpoint, FakeClock) {
        let network = Rc::new(RefCell::new(Network::new()));
        let clock = FakeClock::new(0);
        let a = RastaEndpoint::builder(
            LinkedTransport::new(network.clone(), 0),
            LinkedTransport::new(network.clone(), 1),
            clock.clone(),
        )
        .local_id(1)
        .remote_id(2)
        .config(config(1, 2))
        .build()
        .unwrap();
        let b = RastaEndpoint::builder(
            LinkedTransport::new(network.clone(), 2),
            LinkedTransport::new(network, 3),
            clock.clone(),
        )
        .local_id(2)
        .remote_id(1)
        .config(config(2, 1))
        .build()
        .unwrap();
        (a, b, clock)
    }

    fn poll_pair(a: &mut Endpoint, b: &mut Endpoint, iterations: usize) {
        for _ in 0..iterations {
            let _ = a.poll();
            let _ = b.poll();
        }
    }

    #[test]
    fn endpoint_builder_accepts_valid_config_and_transports() {
        let network = Rc::new(RefCell::new(Network::new()));
        let endpoint = RastaEndpoint::builder(
            LinkedTransport::new(network.clone(), 0),
            LinkedTransport::new(network, 1),
            FakeClock::new(0),
        )
        .local_id(1)
        .remote_id(2)
        .config(config(1, 2))
        .build();

        assert!(endpoint.is_ok());
    }

    #[test]
    fn endpoint_builder_rejects_missing_required_fields() {
        let network = Rc::new(RefCell::new(Network::new()));
        assert_eq!(
            RastaEndpoint::builder(
                LinkedTransport::new(network.clone(), 0),
                LinkedTransport::new(network.clone(), 1),
                FakeClock::new(0),
            )
            .remote_id(2)
            .config(config(1, 2))
            .build()
            .map(|_| ()),
            Err(RastaError::MissingLocalId)
        );
        assert_eq!(
            RastaEndpoint::builder(
                LinkedTransport::new(network.clone(), 0),
                LinkedTransport::new(network.clone(), 1),
                FakeClock::new(0),
            )
            .local_id(1)
            .config(config(1, 2))
            .build()
            .map(|_| ()),
            Err(RastaError::MissingRemoteId)
        );
        assert_eq!(
            RastaEndpoint::builder(
                LinkedTransport::new(network.clone(), 0),
                LinkedTransport::new(network, 1),
                FakeClock::new(0),
            )
            .local_id(1)
            .remote_id(2)
            .build()
            .map(|_| ()),
            Err(RastaError::MissingConfig)
        );
    }

    #[test]
    fn endpoint_builder_rejects_invalid_config() {
        let network = Rc::new(RefCell::new(Network::new()));
        let mut invalid = config(1, 2);
        invalid.mwa = invalid.n_send_max;

        assert_eq!(
            RastaEndpoint::builder(
                LinkedTransport::new(network.clone(), 0),
                LinkedTransport::new(network, 1),
                FakeClock::new(0),
            )
            .local_id(1)
            .remote_id(2)
            .config(invalid)
            .build()
            .map(|_| ()),
            Err(RastaError::InvalidConfiguration)
        );
    }

    #[test]
    fn endpoint_builder_accepts_predefined_profile() {
        let network = Rc::new(RefCell::new(Network::new()));
        let endpoint = RastaEndpoint::builder(
            LinkedTransport::new(network.clone(), 0),
            LinkedTransport::new(network, 1),
            FakeClock::new(0),
        )
        .local_id(1)
        .remote_id(2)
        .profile(RastaProfile::ACADEMIC_DEFAULT)
        .unwrap()
        .build();

        assert!(endpoint.is_ok());
    }

    #[test]
    fn config_from_profile_preserves_librasta_local_unsafe_opt_in() {
        let config = config_from_profile(0x60, 0x61, RastaProfile::LIBRASTA_LOCAL, true).unwrap();

        assert_eq!(config.sender_id, 0x60);
        assert_eq!(config.remote_id, 0x61);
        assert!(config.allow_unsafe_no_checksums);
        assert_eq!(config.t_max, 10_000);
        assert_eq!(config.heartbeat_interval_ms, 2_000);
    }

    #[test]
    fn connect_starts_active_opening_path() {
        let (mut a, mut b, _clock) = endpoint_pair();

        a.connect().unwrap();
        b.connect().unwrap();

        assert_eq!(a.status(), ConnectionStatus::Opening);
        assert_eq!(b.status(), ConnectionStatus::Opening);
    }

    #[test]
    fn poll_processes_incoming_frames_and_receive_hides_internal_queue() {
        let (mut a, mut b, _clock) = endpoint_pair();
        a.connect().unwrap();
        b.connect().unwrap();
        poll_pair(&mut a, &mut b, 8);
        assert_eq!(a.status(), ConnectionStatus::Up);
        assert_eq!(b.status(), ConnectionStatus::Up);

        a.send(b"hello").unwrap();
        poll_pair(&mut a, &mut b, 8);

        let mut output = [0u8; 16];
        let length = b.receive(&mut output).unwrap();
        assert_eq!(&output[..length], b"hello");
    }

    #[test]
    fn send_is_rejected_until_connection_is_up() {
        let (mut a, _b, _clock) = endpoint_pair();

        assert_eq!(a.send(b"not-yet"), Err(RastaError::InvalidState));
    }

    #[test]
    fn close_sends_graceful_disconnection() {
        let (mut a, mut b, _clock) = endpoint_pair();
        a.connect().unwrap();
        b.connect().unwrap();
        poll_pair(&mut a, &mut b, 8);

        a.close().unwrap();
        poll_pair(&mut a, &mut b, 4);

        assert_eq!(a.status(), ConnectionStatus::Down);
        assert_eq!(b.status(), ConnectionStatus::Down);
    }

    #[test]
    fn take_diagnostic_drains_diagnostics() {
        let network = Rc::new(RefCell::new(Network::new()));
        let mut endpoint = RastaEndpoint::builder(
            LinkedTransport {
                fail_send: true,
                ..LinkedTransport::new(network.clone(), 0)
            },
            LinkedTransport::new(network, 1),
            FakeClock::new(0),
        )
        .local_id(1)
        .remote_id(2)
        .config(config(1, 2))
        .build()
        .unwrap();

        endpoint.connect().unwrap();
        endpoint.poll().unwrap();
        assert!(endpoint.take_diagnostic().is_some());
        assert!(endpoint.take_diagnostic().is_none());
    }

    #[test]
    fn take_trace_event_drains_trace_events() {
        let (mut a, mut b, _clock) = endpoint_pair();
        a.connect().unwrap();
        b.connect().unwrap();
        poll_pair(&mut a, &mut b, 8);

        assert!(a.take_trace_event().is_some());
        let mut count = 0;
        a.drain_trace_events(|_| count += 1);
        assert!(count <= 16);
    }
}

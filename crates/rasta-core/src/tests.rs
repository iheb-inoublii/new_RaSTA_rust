#[cfg(test)]
mod cases {
    use crate::config::{InteroperabilityProfile, ProfileError, SafetyCodeLength};
    use crate::connection::pdu::{Packet, PacketType};
    use crate::connection::retransmission::RetransmissionBuffer;
    use crate::connection::safety_code::{Md4, SafetyCodeConfig};
    use crate::connection::sequencing::{SequenceHandler, SequenceResult};
    use crate::connection::state_machine::{RastaState, StateMachine};
    use crate::connection::time_supervision::{TimeSupervisionError, TimeSupervisor};
    use crate::connection::{RastaConfig, RastaConnection};
    use crate::port::{RandomError, RandomSource, Transport, TransportError};
    use crate::redundancy::{
        RedundancyCheckCode, RedundancyConfig, RedundancyCrc, RedundancyLayer,
    };
    use crate::srl::{DisconnectReason, SrlState};
    use crate::time::{
        DurationMs, MonotonicClock, MonotonicInstant, ProtocolTimestamp, ProtocolTimestampSource,
    };
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    struct MockClock {
        time: u32,
    }
    impl MonotonicClock for MockClock {
        fn now(&self) -> MonotonicInstant {
            MonotonicInstant::from_wrapping_millis(self.time)
        }
    }
    impl ProtocolTimestampSource for MockClock {
        fn protocol_timestamp(&self) -> ProtocolTimestamp {
            ProtocolTimestamp::from_wire_millis(self.time)
        }
    }

    struct DeterministicRandom(u32);

    impl RandomSource for DeterministicRandom {
        fn next_u32(&mut self) -> Result<u32, RandomError> {
            Ok(self.0)
        }
    }

    #[derive(Clone, Copy)]
    struct SimpleMockTransport {
        receive_data: [u8; 512],
        receive_len: usize,
        sent: [u8; 512],
        sent_len: usize,
    }

    impl SimpleMockTransport {
        fn empty() -> Self {
            Self {
                receive_data: [0; 512],
                receive_len: 0,
                sent: [0; 512],
                sent_len: 0,
            }
        }

        fn with_receive(data: &[u8]) -> Self {
            let mut transport = Self::empty();
            transport.receive_data[..data.len()].copy_from_slice(data);
            transport.receive_len = data.len();
            transport
        }
    }

    impl Transport for SimpleMockTransport {
        fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            if data.len() > self.sent.len() {
                return Err(TransportError::BufferTooSmall);
            }
            self.sent[..data.len()].copy_from_slice(data);
            self.sent_len = data.len();
            Ok(())
        }
        fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError> {
            if self.receive_len == 0 {
                return Ok(0);
            }
            if buffer.len() < self.receive_len {
                return Err(TransportError::BufferTooSmall);
            }
            buffer[..self.receive_len].copy_from_slice(&self.receive_data[..self.receive_len]);
            let len = self.receive_len;
            self.receive_len = 0;
            Ok(len)
        }
    }

    #[derive(Clone, Copy)]
    struct TestFrame {
        bytes: [u8; 520],
        len: usize,
    }

    struct TestNetwork {
        frames: [[Option<TestFrame>; 32]; 4],
        head: [usize; 4],
        tail: [usize; 4],
        count: [usize; 4],
    }

    impl TestNetwork {
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
        network: Rc<RefCell<TestNetwork>>,
        channel: usize,
    }

    impl LinkedTransport {
        fn new(network: Rc<RefCell<TestNetwork>>, channel: usize) -> Self {
            Self { network, channel }
        }
    }

    impl Transport for LinkedTransport {
        fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            if data.len() > 520 {
                return Err(TransportError::BufferTooSmall);
            }
            let mut network = self.network.borrow_mut();
            let target = TestNetwork::peer(self.channel);
            if network.count[target] == 32 {
                return Err(TransportError::SendFailed);
            }
            let mut bytes = [0u8; 520];
            bytes[..data.len()].copy_from_slice(data);
            let tail = network.tail[target];
            network.frames[target][tail] = Some(TestFrame {
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

    #[derive(Clone)]
    struct SharedClock(Rc<Cell<u32>>);

    impl MonotonicClock for SharedClock {
        fn now(&self) -> MonotonicInstant {
            MonotonicInstant::from_wrapping_millis(self.0.get())
        }
    }
    impl ProtocolTimestampSource for SharedClock {
        fn protocol_timestamp(&self) -> ProtocolTimestamp {
            ProtocolTimestamp::from_wire_millis(self.0.get())
        }
    }

    #[derive(Clone)]
    struct FakeClock(Rc<Cell<u32>>);

    impl FakeClock {
        fn new(now: u32) -> Self {
            Self(Rc::new(Cell::new(now)))
        }

        fn set(&self, now: u32) {
            self.0.set(now);
        }

        fn advance(&self, duration: DurationMs) {
            self.0.set(self.0.get().wrapping_add(duration.as_millis()));
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

    fn config(sender_id: u32, remote_id: u32) -> RastaConfig {
        RastaConfig {
            sender_id,
            remote_id,
            safety_code: SafetyCodeConfig::default(),
            redundancy: RedundancyConfig::default(),
            t_max: 2000,
            initial_seq: 0,
            heartbeat_interval_ms: 500,
            n_send_max: 16,
            mwa: 8,
        }
    }

    #[test]
    fn test_packet_serialization() {
        let safety = SafetyCodeConfig::default();
        let packet = Packet {
            receiver_id: 1,
            sender_id: 2,
            sequence_number: 10,
            confirmed_sequence_number: 5,
            timestamp: 1000,
            confirmed_timestamp: 900,
            packet_type: PacketType::Data,
            payload: [0u8; 256],
            payload_len: 4,
        };
        // Set some dummy payload
        let mut p = packet;
        p.payload[0] = 0xAA;
        p.payload[1] = 0xBB;
        p.payload[2] = 0xCC;
        p.payload[3] = 0xDD;

        let mut buffer = [0u8; 512];
        let size = p
            .serialize(&mut buffer, &safety)
            .expect("Serialization failed");

        assert_eq!(u16::from_le_bytes([buffer[2], buffer[3]]), 6240);

        let parsed = Packet::parse(&buffer[..size], &safety).expect("Parsing failed");
        assert_eq!(parsed.receiver_id, 1);
        assert_eq!(parsed.sender_id, 2);
        assert_eq!(parsed.sequence_number, 10);
        assert_eq!(parsed.payload_len, 4);
        assert_eq!(parsed.payload[0], 0xAA);
    }

    #[test]
    fn test_state_machine_transitions() {
        let mut sm = StateMachine::new();
        assert_eq!(sm.current_state, RastaState::Closed);
        assert!(sm.transition(RastaState::Down));

        // Valid transition
        assert!(sm.transition(RastaState::Start));
        assert_eq!(sm.current_state, RastaState::Start);

        // Invalid transition: Down -> Up (must go through Start)
        let mut sm2 = StateMachine::new();
        assert!(!sm2.transition(RastaState::Up));
        assert_eq!(sm2.current_state, RastaState::Closed);
    }

    #[test]
    fn test_sequence_handler() {
        let mut sh = SequenceHandler::new();
        assert_eq!(sh.next_tx(), 0);
        assert_eq!(sh.next_tx(), 1);

        // Receive 0 (expecting 0)
        assert_eq!(sh.validate_rx(0), SequenceResult::Ok);
        // Receive 1 (expecting 1)
        assert_eq!(sh.validate_rx(1), SequenceResult::Ok);
        // Receive 3 (Gap)
        match sh.validate_rx(3) {
            SequenceResult::Gap(expected) => assert_eq!(expected, 2),
            _ => panic!("Expected Gap"),
        }
    }

    #[test]
    fn test_md4_known_vectors() {
        let empty_digest = Md4::new().finalize();
        assert_eq!(
            empty_digest,
            [
                0x31, 0xd6, 0xcf, 0xe0, 0xd1, 0x6a, 0xe9, 0x31, 0xb7, 0x3c, 0x59, 0xd7, 0xe0, 0xc0,
                0x89, 0xc0,
            ]
        );

        let mut md4 = Md4::new();
        md4.update(b"abc");
        assert_eq!(
            md4.finalize(),
            [
                0xa4, 0x48, 0x01, 0x7a, 0xaf, 0x21, 0xd8, 0x52, 0x5f, 0xc1, 0x0a, 0xe8, 0x7a, 0xa6,
                0x72, 0x9d,
            ]
        );
    }

    #[test]
    fn test_time_supervision() {
        let supervisor = TimeSupervisor::new(2000);
        let timestamp = ProtocolTimestamp::from_wire_millis;

        assert!(
            supervisor
                .validate(timestamp(3000), timestamp(1500))
                .is_ok()
        );
        assert_eq!(
            supervisor.validate(timestamp(3000), timestamp(900)),
            Err(TimeSupervisionError::TimestampTooOld)
        );
        assert_eq!(
            supervisor.validate(timestamp(3000), timestamp(3200)),
            Err(TimeSupervisionError::TimestampTooFarInFuture)
        );
    }

    #[test]
    fn time_supervision_preserves_exact_boundaries_and_wraparound() {
        let supervisor = TimeSupervisor::new(100);
        let timestamp = ProtocolTimestamp::from_wire_millis;
        assert!(supervisor.validate(timestamp(100), timestamp(0)).is_ok());
        assert_eq!(
            supervisor.validate(timestamp(101), timestamp(0)),
            Err(TimeSupervisionError::TimestampTooOld)
        );
        assert!(
            supervisor
                .validate(timestamp(2), timestamp(u32::MAX - 2))
                .is_ok()
        );
    }

    #[test]
    fn fake_clock_advances_and_wraps_without_sleeping() {
        let clock = FakeClock::new(u32::MAX - 2);
        clock.advance(DurationMs::from_millis(5));
        assert_eq!(clock.now().wrapping_millis(), 2);
        assert_eq!(clock.protocol_timestamp().wire_millis(), 2);
        clock.set(10);
        assert_eq!(clock.now().wrapping_millis(), 10);
    }

    #[test]
    fn test_retransmission_buffer() {
        let mut rb = RetransmissionBuffer::new();
        let packet = Packet {
            receiver_id: 1,
            sender_id: 2,
            sequence_number: 100,
            confirmed_sequence_number: 0,
            timestamp: 0,
            confirmed_timestamp: 0,
            packet_type: PacketType::Data,
            payload: [0u8; 256],
            payload_len: 0,
        };

        assert!(rb.store(packet));
        assert_eq!(rb.count(), 1);

        let retrieved = rb.get_packet(100).expect("Packet not found");
        assert_eq!(retrieved.sequence_number, 100);

        rb.clear_up_to(100);
        assert_eq!(rb.count(), 0);
    }

    #[test]
    fn test_connection_handshake_start() {
        let clock = MockClock { time: 0 };
        let config = config(123, 456);
        let mut conn = RastaConnection::try_new(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            clock,
            config,
        )
        .unwrap();

        assert_eq!(conn.state_machine.current_state, RastaState::Closed);
        conn.connect().expect("Connect failed");
        assert_eq!(conn.state_machine.current_state, RastaState::Start);
    }

    #[test]
    fn connection_uses_injected_random_initial_sequence() {
        let mut random = DeterministicRandom(0xa5a5_5a5a);
        let connection = RastaConnection::try_new_with_random(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            MockClock { time: 0 },
            config(1, 2),
            &mut random,
        )
        .unwrap();
        assert_eq!(connection.sequence.next_tx_value(), 0xa5a5_5a5a);
    }

    #[test]
    fn application_tx_queue_is_bounded_when_flow_control_blocks() {
        let mut connection = RastaConnection::try_new(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            MockClock { time: 0 },
            config(1, 2),
        )
        .unwrap();
        connection.transition(RastaState::Down).unwrap();
        connection.transition(RastaState::Start).unwrap();
        connection.transition(RastaState::Up).unwrap();

        for _ in 0..36 {
            assert!(connection.send_application_data(b"x").is_ok());
        }
        assert!(matches!(
            connection.send_application_data(b"x"),
            Err(crate::connection::ConnectionError::TransmitQueueFull)
        ));
    }

    #[test]
    fn two_endpoint_two_channel_connection_and_data_interoperate() {
        let network = Rc::new(RefCell::new(TestNetwork::new()));
        let time = Rc::new(Cell::new(0));
        let mut client = RastaConnection::try_new(
            LinkedTransport::new(network.clone(), 0),
            LinkedTransport::new(network.clone(), 1),
            SharedClock(time.clone()),
            config(1, 2),
        )
        .unwrap();
        let mut server = RastaConnection::try_new(
            LinkedTransport::new(network.clone(), 2),
            LinkedTransport::new(network, 3),
            SharedClock(time.clone()),
            config(2, 1),
        )
        .unwrap();

        client.connect().unwrap();
        server.connect().unwrap();
        server.process().unwrap();
        client.process().unwrap();
        server.process().unwrap();
        assert_eq!(client.state_machine.current_state, RastaState::Up);
        assert_eq!(server.state_machine.current_state, RastaState::Up);

        client.send_application_data(b"rail-safe").unwrap();
        server.process().unwrap();
        let mut output = [0u8; 32];
        let length = server.receive_data(&mut output).unwrap();
        assert_eq!(&output[..length], b"rail-safe");

        // Exercise several heartbeat periods beyond T_max. A stale confirmed
        // timestamp would make either peer enter SafetyTimeout here.
        for now in [300u32, 600, 900, 1_200, 1_500, 1_800, 2_100] {
            time.set(now);
            client.process().unwrap();
            server.process().unwrap();
            client.process().unwrap();
            server.process().unwrap();
            assert_eq!(client.state_machine.current_state, RastaState::Up);
            assert_eq!(server.state_machine.current_state, RastaState::Up);
        }
    }

    #[test]
    fn bad_safety_code_is_rejected_and_counted_without_closing_connection() {
        let mut connection = RastaConnection::try_new(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            MockClock { time: 0 },
            config(1, 2),
        )
        .unwrap();
        connection.transition(RastaState::Down).unwrap();
        connection.transition(RastaState::Start).unwrap();
        connection.transition(RastaState::Up).unwrap();

        let packet = Packet {
            receiver_id: 1,
            sender_id: 2,
            sequence_number: 0,
            confirmed_sequence_number: 0,
            timestamp: 0,
            confirmed_timestamp: 0,
            packet_type: PacketType::Heartbeat,
            payload: [0; 256],
            payload_len: 0,
        };
        let mut srl = [0u8; 512];
        let length = packet
            .serialize(&mut srl, &SafetyCodeConfig::default())
            .unwrap();
        srl[length - 1] ^= 0xff;
        let total =
            length + RedundancyLayer::<SimpleMockTransport, SimpleMockTransport>::HEADER_SIZE;
        let mut rl = [0u8; 520];
        rl[..2].copy_from_slice(&(total as u16).to_le_bytes());
        rl[4..8].copy_from_slice(&0u32.to_le_bytes());
        rl[8..total].copy_from_slice(&srl[..length]);
        connection.redundancy = RedundancyLayer::with_config(
            SimpleMockTransport::with_receive(&rl[..total]),
            SimpleMockTransport::empty(),
            RedundancyConfig {
                check_code: RedundancyCheckCode::None,
                t_seq_ms: 100,
            },
        );

        assert!(connection.process().is_ok());
        assert_eq!(connection.state_machine.current_state, RastaState::Up);
        assert_eq!(connection.error_counters().safety, 1);
    }

    #[test]
    fn test_insecure_configuration_is_rejected() {
        let clock = MockClock { time: 0 };
        let mut insecure = config(1, 2);
        insecure.redundancy = RedundancyConfig {
            check_code: RedundancyCheckCode::None,
            t_seq_ms: 100,
        };

        assert!(
            RastaConnection::try_new(
                SimpleMockTransport::empty(),
                SimpleMockTransport::empty(),
                clock,
                insecure,
            )
            .is_err()
        );
    }

    #[test]
    fn test_application_receive_queue() {
        let clock = MockClock { time: 0 };
        let mut conn = RastaConnection::try_new(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            clock,
            config(1, 2),
        )
        .unwrap();

        conn.transition(RastaState::Down).unwrap();
        conn.transition(RastaState::Start).unwrap();
        conn.sequence.accept_initial_rx(99);
        conn.transition(RastaState::Up).unwrap();

        let mut packet = Packet {
            receiver_id: 1,
            sender_id: 2,
            sequence_number: 100,
            confirmed_sequence_number: 0,
            timestamp: 0,
            confirmed_timestamp: 0,
            packet_type: PacketType::Data,
            payload: [0; 256],
            payload_len: 7,
        };
        packet.payload[..2].copy_from_slice(&5u16.to_le_bytes());
        packet.payload[2..7].copy_from_slice(b"hello");

        let mut wire = [0u8; 512];
        let len = packet
            .serialize(&mut wire, &SafetyCodeConfig::default())
            .unwrap();
        let mut rl_frame = [0u8; 520];
        let total = len + RedundancyLayer::<SimpleMockTransport, SimpleMockTransport>::HEADER_SIZE;
        rl_frame[..2].copy_from_slice(&(total as u16).to_le_bytes());
        rl_frame[4..8].copy_from_slice(&0u32.to_le_bytes());
        rl_frame[8..total].copy_from_slice(&wire[..len]);

        conn.redundancy = RedundancyLayer::with_config(
            SimpleMockTransport::with_receive(&rl_frame[..total]),
            SimpleMockTransport::empty(),
            RedundancyConfig {
                check_code: RedundancyCheckCode::None,
                t_seq_ms: 100,
            },
        );
        conn.process().unwrap();

        let mut out = [0u8; 16];
        let received = conn.receive_data(&mut out).unwrap();
        assert_eq!(&out[..received], b"hello");
    }

    #[test]
    fn interoperability_profile_validation_is_value_based() {
        let profile = InteroperabilityProfile {
            protocol_version: InteroperabilityProfile::VERSION_03_03,
            safety_code_length: SafetyCodeLength::Md4Lower8,
            redundancy_crc: RedundancyCrc::OptionB,
            channel_count: 2,
            network_identifier: 0x0000_0001,
            md4_initial_value: [
                0x02, 0x23, 0x45, 0x67, 0x98, 0xab, 0xcd, 0xef, 0xff, 0xdc, 0xba, 0x98, 0x77, 0x54,
                0x32, 0x10,
            ],
            t_max_ms: 1_800,
            t_h_ms: 300,
            t_seq_ms: 100,
            n_send_max: 20,
            mwa: 10,
            defer_queue_capacity: 4,
            retransmission_capacity: 20,
            application_queue_capacity: 20,
            diagnostic_queue_capacity: 16,
            max_messages_per_packet: 1,
        };
        assert_eq!(profile.protocol_version, *b"0303");
        assert!(profile.validate().is_ok());

        let mut invalid = profile;
        invalid.mwa = invalid.n_send_max;
        assert_eq!(invalid.validate(), Err(ProfileError::InvalidFlowControl));

        invalid = profile;
        invalid.protocol_version = *b"0301";
        assert_eq!(
            invalid.validate(),
            Err(ProfileError::UnsupportedProtocolVersion)
        );

        invalid = profile;
        invalid.md4_initial_value = InteroperabilityProfile::RFC_MD4_INITIAL_VALUE;
        assert_eq!(invalid.validate(), Err(ProfileError::UnsafeMd4InitialValue));
    }

    #[test]
    fn din_disconnect_reason_codes_round_trip_without_unknown_panics() {
        assert_eq!(DisconnectReason::UserRequest.code(), 0);
        assert_eq!(DisconnectReason::ProtocolVersionError.code(), 6);
        assert_eq!(
            DisconnectReason::from_code(7),
            DisconnectReason::RetransmissionUnavailable
        );
        assert_eq!(
            DisconnectReason::from_code(0x1234),
            DisconnectReason::Unknown(0x1234)
        );
        assert_eq!(
            SrlState::RetransmissionRunning,
            SrlState::RetransmissionRunning
        );
    }

    #[test]
    fn md4_safety_code_matches_din_annex_a_lower_half() {
        let safety = SafetyCodeConfig::md4_low8([
            0x82, 0x67, 0xb1, 0xaf, 0xde, 0x59, 0x4c, 0x30, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54,
            0x32, 0x10,
        ]);
        let message = [
            0x24, 0x00, 0x4c, 0x18, 0x3f, 0xb4, 0x96, 0x00, 0xce, 0xca, 0x23, 0x00, 0x56, 0x44,
            0x33, 0x22, 0x66, 0x55, 0x44, 0x33, 0x57, 0x01, 0x00, 0x00, 0xcb, 0x00, 0x00, 0x00,
        ];
        assert_eq!(
            &safety.calculate(&message)[..8],
            &[0x93, 0x9f, 0x1c, 0x86, 0x59, 0xcf, 0xf5, 0x03]
        );
    }

    #[test]
    fn pdu_parser_does_not_panic_on_malformed_input() {
        let safety = SafetyCodeConfig::default();
        let mut bytes = [0u8; 512];
        for length in 0..=bytes.len() {
            for (index, byte) in bytes.iter_mut().enumerate() {
                *byte = (index as u8).wrapping_mul(37).wrapping_add(length as u8);
            }
            let result = std::panic::catch_unwind(|| Packet::parse(&bytes[..length], &safety));
            assert!(result.is_ok(), "parser panicked for length {length}");
        }
    }

    #[test]
    fn din_control_pdus_enforce_exact_payload_rules() {
        let safety = SafetyCodeConfig::default();
        let mut buffer = [0u8; 64];
        let mut retransmission_request = Packet {
            receiver_id: 1,
            sender_id: 2,
            sequence_number: 1,
            confirmed_sequence_number: 0,
            timestamp: 0,
            confirmed_timestamp: 0,
            packet_type: PacketType::RetransmissionRequest,
            payload: [0; 256],
            payload_len: 1,
        };
        retransmission_request.payload[0] = 42;
        assert!(
            retransmission_request
                .serialize(&mut buffer, &safety)
                .is_err()
        );

        let disconnection_request = Packet {
            packet_type: PacketType::DisconnectionRequest,
            payload_len: 4,
            ..retransmission_request
        };
        assert!(
            disconnection_request
                .serialize(&mut buffer, &safety)
                .is_ok()
        );
    }
}

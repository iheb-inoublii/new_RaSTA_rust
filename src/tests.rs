#[cfg(test)]
mod cases {
    use crate::config::{
        DIN_RASTA_03_03_INTEROPERABILITY_TEST_PROFILE, InteroperabilityProfile, ProfileError,
    };
    use crate::core::connection::{RastaConfig, RastaConnection};
    use crate::core::connection_state_machine::{RastaState, StateMachine};
    use crate::core::pdu::{Packet, PacketType};
    use crate::core::redundancy_management::{
        RedundancyCheckCode, RedundancyConfig, RedundancyLayer,
    };
    use crate::core::retransmission::RetransmissionBuffer;
    use crate::core::safety_code::{Md4, SafetyCodeConfig};
    use crate::core::sequencing::{SequenceHandler, SequenceResult};
    use crate::core::time_supervision::{TimeSupervisionError, TimeSupervisor};
    use crate::fixed_queue::{FixedQueue, FixedQueueError};
    use crate::packet_io::{PacketIoError, PacketReader, PacketWriter};
    use crate::platform::clock::Clock;
    use crate::platform::random::{RandomError, RandomSource};
    use crate::platform::timer::Timer;
    use crate::platform::transport::{Transport, TransportError};
    use crate::serial;
    use crate::srl::{DisconnectReason, SrlState};
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    struct MockClock {
        time: u32,
    }
    impl Clock for MockClock {
        fn now_ms(&self) -> u32 {
            self.time
        }
    }

    struct MockTimer {
        end_time: u32,
        running: bool,
    }
    impl Timer for MockTimer {
        fn start(&mut self, duration_ms: u32) {
            self.end_time = duration_ms; // Simplified for test
            self.running = true;
        }
        fn expired(&self) -> bool {
            self.running
        } // Simplified
        fn stop(&mut self) {
            self.running = false;
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

    impl Clock for SharedClock {
        fn now_ms(&self) -> u32 {
            self.0.get()
        }
    }

    struct SharedTimer {
        clock: Rc<Cell<u32>>,
        deadline: Option<u32>,
    }

    impl Timer for SharedTimer {
        fn start(&mut self, duration_ms: u32) {
            self.deadline = Some(self.clock.get().wrapping_add(duration_ms));
        }

        fn expired(&self) -> bool {
            self.deadline.is_some_and(|deadline| {
                self.clock.get() == deadline
                    || self.clock.get().wrapping_sub(deadline) < 0x8000_0000
            })
        }

        fn stop(&mut self) {
            self.deadline = None;
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

        assert!(supervisor.validate(3000, 1500).is_ok());
        assert_eq!(
            supervisor.validate(3000, 900),
            Err(TimeSupervisionError::TimestampTooOld)
        );
        assert_eq!(
            supervisor.validate(3000, 3200),
            Err(TimeSupervisionError::TimestampTooFarInFuture)
        );
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
        let timer = MockTimer {
            end_time: 0,
            running: false,
        };
        let config = config(123, 456);
        let mut conn = RastaConnection::try_new(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            timer,
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
            MockTimer {
                end_time: 0,
                running: false,
            },
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
            MockTimer {
                end_time: 0,
                running: false,
            },
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
            Err(crate::core::connection::ConnectionError::TransmitQueueFull)
        ));
    }

    #[test]
    fn two_endpoint_two_channel_connection_and_data_interoperate() {
        let network = Rc::new(RefCell::new(TestNetwork::new()));
        let time = Rc::new(Cell::new(0));
        let mut client = RastaConnection::try_new(
            LinkedTransport::new(network.clone(), 0),
            LinkedTransport::new(network.clone(), 1),
            SharedTimer {
                clock: time.clone(),
                deadline: None,
            },
            SharedClock(time.clone()),
            config(1, 2),
        )
        .unwrap();
        let mut server = RastaConnection::try_new(
            LinkedTransport::new(network.clone(), 2),
            LinkedTransport::new(network, 3),
            SharedTimer {
                clock: time.clone(),
                deadline: None,
            },
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
            MockTimer {
                end_time: 0,
                running: false,
            },
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
        let timer = MockTimer {
            end_time: 0,
            running: false,
        };
        let mut insecure = config(1, 2);
        insecure.redundancy = RedundancyConfig {
            check_code: RedundancyCheckCode::None,
            t_seq_ms: 100,
        };

        assert!(
            RastaConnection::try_new(
                SimpleMockTransport::empty(),
                SimpleMockTransport::empty(),
                timer,
                clock,
                insecure,
            )
            .is_err()
        );
    }

    #[test]
    fn test_application_receive_queue() {
        let clock = MockClock { time: 0 };
        let timer = MockTimer {
            end_time: 0,
            running: false,
        };
        let mut conn = RastaConnection::try_new(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            timer,
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
    fn test_redundancy_discards_duplicate_channel_copy() {
        let payload = b"safe-pdu";
        let total = payload.len()
            + RedundancyLayer::<SimpleMockTransport, SimpleMockTransport>::HEADER_SIZE;
        let mut frame = [0u8; 520];
        frame[..2].copy_from_slice(&(total as u16).to_le_bytes());
        // DIN 6.3.3: the receiver must not reject a non-zero reserve field.
        frame[2..4].copy_from_slice(&0xbeefu16.to_le_bytes());
        frame[4..8].copy_from_slice(&0u32.to_le_bytes());
        frame[8..total].copy_from_slice(payload);

        let mut redundancy = RedundancyLayer::with_config(
            SimpleMockTransport::with_receive(&frame[..total]),
            SimpleMockTransport::with_receive(&frame[..total]),
            RedundancyConfig {
                check_code: RedundancyCheckCode::None,
                t_seq_ms: 100,
            },
        );

        let mut out = [0u8; 32];
        let len = redundancy.receive(&mut out).unwrap();
        assert_eq!(&out[..len], payload);

        let second = redundancy.receive(&mut out).unwrap();
        assert_eq!(second, 0);
    }

    #[test]
    fn redundancy_defers_ahead_frame_until_missing_sequence_arrives() {
        let mut ahead = [0u8; 520];
        let mut expected = [0u8; 520];
        let ahead_payload = b"one";
        let expected_payload = b"zero";
        let ahead_len = RedundancyLayer::<SimpleMockTransport, SimpleMockTransport>::HEADER_SIZE
            + ahead_payload.len();
        let expected_len = RedundancyLayer::<SimpleMockTransport, SimpleMockTransport>::HEADER_SIZE
            + expected_payload.len();
        ahead[..2].copy_from_slice(&(ahead_len as u16).to_le_bytes());
        ahead[4..8].copy_from_slice(&1u32.to_le_bytes());
        ahead[8..ahead_len].copy_from_slice(ahead_payload);
        expected[..2].copy_from_slice(&(expected_len as u16).to_le_bytes());
        expected[4..8].copy_from_slice(&0u32.to_le_bytes());
        expected[8..expected_len].copy_from_slice(expected_payload);

        let mut redundancy = RedundancyLayer::with_config(
            SimpleMockTransport::with_receive(&ahead[..ahead_len]),
            SimpleMockTransport::with_receive(&expected[..expected_len]),
            RedundancyConfig {
                check_code: RedundancyCheckCode::None,
                t_seq_ms: 100,
            },
        );
        let mut out = [0u8; 32];
        let len = redundancy.receive_at(&mut out, 0).unwrap();
        assert_eq!(&out[..len], expected_payload);
        let len = redundancy.receive_at(&mut out, 1).unwrap();
        assert_eq!(&out[..len], ahead_payload);
    }

    #[test]
    fn redundancy_releases_deferred_frame_after_t_seq() {
        let mut ahead = [0u8; 520];
        let payload = b"one";
        let len = RedundancyLayer::<SimpleMockTransport, SimpleMockTransport>::HEADER_SIZE
            + payload.len();
        ahead[..2].copy_from_slice(&(len as u16).to_le_bytes());
        ahead[4..8].copy_from_slice(&1u32.to_le_bytes());
        ahead[8..len].copy_from_slice(payload);
        let mut redundancy = RedundancyLayer::with_config(
            SimpleMockTransport::with_receive(&ahead[..len]),
            SimpleMockTransport::empty(),
            RedundancyConfig {
                check_code: RedundancyCheckCode::None,
                t_seq_ms: 100,
            },
        );
        let mut out = [0u8; 32];
        assert_eq!(redundancy.receive_at(&mut out, 0).unwrap(), 0);
        let delivered = redundancy.receive_at(&mut out, 100).unwrap();
        assert_eq!(&out[..delivered], payload);
    }

    #[test]
    fn din_interoperability_profile_is_valid_and_immutable_by_copy() {
        let profile = DIN_RASTA_03_03_INTEROPERABILITY_TEST_PROFILE;
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
    fn serial_number_arithmetic_handles_wraparound() {
        assert!(serial::is_after(0, u32::MAX));
        assert!(serial::is_before(u32::MAX, 0));
        assert_eq!(serial::forward_distance(u32::MAX, 1), 2);
        assert!(serial::is_in_forward_window(1, u32::MAX, 2));
        assert!(!serial::is_after(0x8000_0000, 0));
    }

    #[test]
    fn fixed_queue_preserves_order_and_reports_overflow() {
        let mut queue = FixedQueue::<u8, 2>::new();
        assert!(queue.is_empty());
        assert_eq!(queue.push(1), Ok(()));
        assert_eq!(queue.push(2), Ok(()));
        assert_eq!(queue.push(3), Err(FixedQueueError::Full));
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.pop(), Some(1));
        assert_eq!(queue.pop(), Some(2));
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn packet_reader_writer_are_checked() {
        let mut bytes = [0u8; 6];
        let mut writer = PacketWriter::new(&mut bytes);
        assert_eq!(writer.write_u16_le(0x1234), Ok(()));
        assert_eq!(writer.write_u32_le(0x89ab_cdef), Ok(()));
        assert_eq!(writer.write_bytes(&[1]), Err(PacketIoError::BufferFull));

        let mut reader = PacketReader::new(&bytes);
        assert_eq!(reader.read_u16_le(), Ok(0x1234));
        assert_eq!(reader.read_u32_le(), Ok(0x89ab_cdef));
        assert_eq!(reader.read_u16_le(), Err(PacketIoError::Truncated));
        assert_eq!(reader.remaining(), 0);
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

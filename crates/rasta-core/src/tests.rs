#[cfg(test)]
mod cases {
    use crate::config::{
        ConfigError, InteroperabilityProfile, ProfileError, RastaProfile, RastaProfileBuilder,
        SafetyCodeLength, TimestampCompatibilityMode,
    };
    use crate::connection::pdu::{Packet, PacketError, PacketType};
    use crate::connection::retransmission::RetransmissionBuffer;
    use crate::connection::safety_code::{Md4, SafetyCodeConfig, SafetyCodeMode};
    use crate::connection::sequencing::{SequenceHandler, SequenceResult};
    use crate::connection::state_machine::{RastaState, StateMachine};
    use crate::connection::time_supervision::{TimeSupervisionError, TimeSupervisor};
    use crate::connection::{
        ConnectionError, RastaConfig, RastaConnection, TimestampTraceRejection,
    };
    use crate::port::{RandomError, RandomSource, Transport, TransportError};
    use crate::redundancy::{
        ChannelStatus, RedundancyCheckCode, RedundancyConfig, RedundancyCrc, RedundancyLayer,
    };
    use crate::srl::{DiagnosticEvent, DiagnosticKind, DisconnectReason, SrlState};
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

        fn drop_next(&mut self, channel: usize) -> bool {
            if self.count[channel] == 0 {
                return false;
            }
            let head = self.head[channel];
            self.frames[channel][head] = None;
            self.head[channel] = (head + 1) % 32;
            self.count[channel] -= 1;
            true
        }

        fn peek_payload(&self, channel: usize, output: &mut [u8]) -> Option<usize> {
            if self.count[channel] == 0 {
                return None;
            }
            let frame = self.frames[channel][self.head[channel]]?;
            let check_len = 4usize;
            let payload_start = RedundancyLayer::<LinkedTransport, LinkedTransport>::HEADER_SIZE;
            let payload_end = frame.len.checked_sub(check_len)?;
            let payload = frame.bytes.get(payload_start..payload_end)?;
            output.get_mut(..payload.len())?.copy_from_slice(payload);
            Some(payload.len())
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

    #[derive(Clone, Copy)]
    struct FailingTransport;

    impl Transport for FailingTransport {
        fn send(&mut self, _data: &[u8]) -> Result<(), TransportError> {
            Err(TransportError::SendFailed)
        }

        fn receive(&mut self, _buffer: &mut [u8]) -> Result<usize, TransportError> {
            Ok(0)
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
    struct SplitClock {
        monotonic: Rc<Cell<u32>>,
        protocol: Rc<Cell<u32>>,
    }

    impl SplitClock {
        fn new(monotonic: u32, protocol: u32) -> Self {
            Self {
                monotonic: Rc::new(Cell::new(monotonic)),
                protocol: Rc::new(Cell::new(protocol)),
            }
        }

        fn advance(&self, duration: u32) {
            self.monotonic
                .set(self.monotonic.get().wrapping_add(duration));
            self.protocol
                .set(self.protocol.get().wrapping_add(duration));
        }
    }

    impl MonotonicClock for SplitClock {
        fn now(&self) -> MonotonicInstant {
            MonotonicInstant::from_wrapping_millis(self.monotonic.get())
        }
    }

    impl ProtocolTimestampSource for SplitClock {
        fn protocol_timestamp(&self) -> ProtocolTimestamp {
            ProtocolTimestamp::from_wire_millis(self.protocol.get())
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
            allow_unsafe_no_checksums: false,
            timestamp_compatibility: TimestampCompatibilityMode::StrictSynchronized,
        }
    }

    fn peer_relative_config(sender_id: u32, remote_id: u32) -> RastaConfig {
        let mut cfg = config(sender_id, remote_id);
        cfg.t_max = 10_000;
        cfg.timestamp_compatibility = TimestampCompatibilityMode::PeerRelative;
        cfg
    }

    fn valid_profile() -> InteroperabilityProfile {
        InteroperabilityProfile {
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
            timestamp_compatibility: TimestampCompatibilityMode::StrictSynchronized,
        }
    }

    fn packet(packet_type: PacketType, payload_len: usize) -> Packet {
        Packet {
            receiver_id: 1,
            sender_id: 2,
            sequence_number: 10,
            confirmed_sequence_number: 5,
            timestamp: 1000,
            confirmed_timestamp: 900,
            packet_type,
            payload: [0; 256],
            payload_len,
        }
    }

    fn redundancy_frame_from_packet(
        packet: &Packet,
        safety: &SafetyCodeConfig,
    ) -> ([u8; 520], usize) {
        let mut pdu = [0u8; 512];
        let pdu_len = packet.serialize(&mut pdu, safety).unwrap();
        let total =
            pdu_len + RedundancyLayer::<SimpleMockTransport, SimpleMockTransport>::HEADER_SIZE;
        let mut frame = [0u8; 520];
        frame[..2].copy_from_slice(&(total as u16).to_le_bytes());
        frame[4..8].copy_from_slice(&0u32.to_le_bytes());
        frame[8..total].copy_from_slice(&pdu[..pdu_len]);
        (frame, total)
    }

    fn receive_packet_transport(packet: &Packet, safety: &SafetyCodeConfig) -> SimpleMockTransport {
        let (frame, len) = redundancy_frame_from_packet(packet, safety);
        SimpleMockTransport::with_receive(&frame[..len])
    }

    fn inject_received_packet(
        connection: &mut RastaConnection<SimpleMockTransport, SimpleMockTransport, FakeClock>,
        packet: &Packet,
    ) {
        connection.redundancy = RedundancyLayer::with_config(
            receive_packet_transport(packet, &connection.safety_code),
            SimpleMockTransport::empty(),
            RedundancyConfig {
                check_code: RedundancyCheckCode::None,
                t_seq_ms: 100,
            },
        );
    }

    fn confirmation_connection(
        initial_seq: u32,
        n_send_max: u16,
        mwa: u16,
    ) -> (
        RastaConnection<SimpleMockTransport, SimpleMockTransport, FakeClock>,
        FakeClock,
    ) {
        let clock = FakeClock::new(1_000);
        let mut cfg = config(1, 2);
        cfg.initial_seq = initial_seq;
        cfg.n_send_max = n_send_max;
        cfg.mwa = mwa;
        let mut connection = RastaConnection::try_new(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            clock.clone(),
            cfg,
        )
        .unwrap();
        connection.transition(RastaState::Down).unwrap();
        connection.transition(RastaState::Start).unwrap();
        connection.transition(RastaState::Up).unwrap();
        (connection, clock)
    }

    fn receive_packet(
        connection: &mut RastaConnection<SimpleMockTransport, SimpleMockTransport, FakeClock>,
        packet: &Packet,
    ) {
        let transport = receive_packet_transport(packet, &connection.safety_code);
        connection.redundancy = RedundancyLayer::with_config(
            transport,
            SimpleMockTransport::empty(),
            RedundancyConfig {
                check_code: RedundancyCheckCode::None,
                t_seq_ms: 100,
            },
        );
    }

    fn ack_heartbeat(rx_sequence: u32, confirmed_sequence: u32, timestamp: u32) -> Packet {
        let mut heartbeat = packet(PacketType::Heartbeat, 0);
        heartbeat.receiver_id = 1;
        heartbeat.sender_id = 2;
        heartbeat.sequence_number = rx_sequence;
        heartbeat.confirmed_sequence_number = confirmed_sequence;
        heartbeat.timestamp = timestamp;
        heartbeat.confirmed_timestamp = timestamp;
        heartbeat
    }

    fn data_ack(
        rx_sequence: u32,
        confirmed_sequence: u32,
        timestamp: u32,
        payload: &[u8],
    ) -> Packet {
        let mut data = packet(PacketType::Data, payload.len() + 2);
        data.receiver_id = 1;
        data.sender_id = 2;
        data.sequence_number = rx_sequence;
        data.confirmed_sequence_number = confirmed_sequence;
        data.timestamp = timestamp;
        data.confirmed_timestamp = timestamp;
        data.payload[..2].copy_from_slice(&(payload.len() as u16).to_le_bytes());
        data.payload[2..2 + payload.len()].copy_from_slice(payload);
        data
    }

    fn clear_network_channels(network: &Rc<RefCell<TestNetwork>>, channels: &[usize]) {
        let mut network = network.borrow_mut();
        for channel in channels {
            while network.drop_next(*channel) {}
        }
    }

    fn network_has_packet_type(
        network: &Rc<RefCell<TestNetwork>>,
        channels: &[usize],
        safety: &SafetyCodeConfig,
        packet_type: PacketType,
    ) -> bool {
        let network = network.borrow();
        for channel in channels {
            for offset in 0..network.count[*channel] {
                let index = (network.head[*channel] + offset) % 32;
                let Some(frame) = network.frames[*channel][index] else {
                    continue;
                };
                let Some(payload_end) = frame.len.checked_sub(4) else {
                    continue;
                };
                let Some(payload) = frame.bytes.get(
                    RedundancyLayer::<LinkedTransport, LinkedTransport>::HEADER_SIZE..payload_end,
                ) else {
                    continue;
                };
                if Packet::parse(payload, safety)
                    .is_ok_and(|packet| packet.packet_type == packet_type)
                {
                    return true;
                }
            }
        }
        false
    }

    fn network_packet_type_count(
        network: &Rc<RefCell<TestNetwork>>,
        channels: &[usize],
        safety: &SafetyCodeConfig,
        packet_type: PacketType,
    ) -> usize {
        let network = network.borrow();
        let mut count = 0;
        for channel in channels {
            for offset in 0..network.count[*channel] {
                let index = (network.head[*channel] + offset) % 32;
                let Some(frame) = network.frames[*channel][index] else {
                    continue;
                };
                let Some(payload_end) = frame.len.checked_sub(4) else {
                    continue;
                };
                let Some(payload) = frame.bytes.get(
                    RedundancyLayer::<LinkedTransport, LinkedTransport>::HEADER_SIZE..payload_end,
                ) else {
                    continue;
                };
                if Packet::parse(payload, safety)
                    .is_ok_and(|packet| packet.packet_type == packet_type)
                {
                    count += 1;
                }
            }
        }
        count
    }

    fn network_head_packet_type(
        network: &Rc<RefCell<TestNetwork>>,
        channel: usize,
        safety: &SafetyCodeConfig,
    ) -> Option<PacketType> {
        let network = network.borrow();
        let frame = network.frames[channel][network.head[channel]]?;
        let payload_end = frame.len.checked_sub(4)?;
        let payload = frame
            .bytes
            .get(RedundancyLayer::<LinkedTransport, LinkedTransport>::HEADER_SIZE..payload_end)?;
        Packet::parse(payload, safety)
            .ok()
            .map(|packet| packet.packet_type)
    }

    fn instant_is_later(left: MonotonicInstant, right: MonotonicInstant) -> bool {
        left != right && left.elapsed_since(right).as_millis() < 0x8000_0000
    }

    fn librasta_safety_none() -> SafetyCodeConfig {
        SafetyCodeConfig::none()
    }

    fn librasta_frame_packet(frame: &[u8]) -> Packet {
        assert_eq!(
            u16::from_le_bytes([frame[0], frame[1]]) as usize,
            frame.len()
        );
        assert_eq!(u16::from_le_bytes([frame[2], frame[3]]), 0);
        Packet::parse(&frame[8..], &librasta_safety_none()).unwrap()
    }

    fn encode_librasta_frame(packet: &Packet, rl_sequence: u32) -> ([u8; 520], usize) {
        let mut srl = [0u8; 512];
        let srl_len = packet.serialize(&mut srl, &librasta_safety_none()).unwrap();
        let total =
            RedundancyLayer::<SimpleMockTransport, SimpleMockTransport>::HEADER_SIZE + srl_len;
        let mut frame = [0u8; 520];
        frame[..2].copy_from_slice(&(total as u16).to_le_bytes());
        frame[2..4].copy_from_slice(&0u16.to_le_bytes());
        frame[4..8].copy_from_slice(&rl_sequence.to_le_bytes());
        frame[8..total].copy_from_slice(&srl[..srl_len]);
        (frame, total)
    }

    fn encode_frame_with_safety(
        packet: &Packet,
        safety: &SafetyCodeConfig,
        rl_sequence: u32,
    ) -> ([u8; 520], usize) {
        let mut srl = [0u8; 512];
        let srl_len = packet.serialize(&mut srl, safety).unwrap();
        let total =
            RedundancyLayer::<SimpleMockTransport, SimpleMockTransport>::HEADER_SIZE + srl_len;
        let mut frame = [0u8; 520];
        frame[..2].copy_from_slice(&(total as u16).to_le_bytes());
        frame[2..4].copy_from_slice(&0u16.to_le_bytes());
        frame[4..8].copy_from_slice(&rl_sequence.to_le_bytes());
        frame[8..total].copy_from_slice(&srl[..srl_len]);
        (frame, total)
    }

    const LIBRASTA_C_CONNREQ: [u8; 50] = [
        0x32, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2a, 0x00, 0x38, 0x18, 0x61, 0x00, 0x00,
        0x00, 0x60, 0x00, 0x00, 0x00, 0x04, 0x03, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00, 0x44, 0x33,
        0x22, 0x11, 0x00, 0x00, 0x00, 0x00, b'0', b'3', b'0', b'3', 0x0a, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    const LIBRASTA_C_CONNRESP: [u8; 50] = [
        0x32, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x2a, 0x00, 0x39, 0x18, 0x60, 0x00, 0x00,
        0x00, 0x61, 0x00, 0x00, 0x00, 0x0d, 0x0c, 0x0b, 0x0a, 0x04, 0x03, 0x02, 0x01, 0x88, 0x77,
        0x66, 0x55, 0x44, 0x33, 0x22, 0x11, b'0', b'3', b'0', b'3', 0x0a, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    const LIBRASTA_C_HEARTBEAT: [u8; 36] = [
        0x24, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x1c, 0x00, 0x4c, 0x18, 0x60, 0x00, 0x00,
        0x00, 0x61, 0x00, 0x00, 0x00, 0x14, 0x13, 0x12, 0x11, 0x04, 0x03, 0x02, 0x01, 0xcc, 0xbb,
        0xaa, 0x99, 0x44, 0x33, 0x22, 0x11,
    ];

    const LIBRASTA_C_DISCREQ: [u8; 40] = [
        0x28, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x20, 0x00, 0x48, 0x18, 0x61, 0x00, 0x00,
        0x00, 0x60, 0x00, 0x00, 0x00, 0x24, 0x23, 0x22, 0x21, 0x14, 0x13, 0x12, 0x11, 0xef, 0xbe,
        0xad, 0xde, 0xcc, 0xbb, 0xaa, 0x99, 0x00, 0x00, 0x00, 0x00,
    ];

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
    fn librasta_type_a_no_sr_checksum_vectors_decode() {
        let request = librasta_frame_packet(&LIBRASTA_C_CONNREQ);
        assert_eq!(request.packet_type, PacketType::ConnectionRequest);
        assert_eq!(request.receiver_id, 0x61);
        assert_eq!(request.sender_id, 0x60);
        assert_eq!(request.sequence_number, 0x0102_0304);
        assert_eq!(request.confirmed_sequence_number, 0);
        assert_eq!(request.timestamp, 0x1122_3344);
        assert_eq!(request.confirmed_timestamp, 0);
        assert_eq!(request.payload_len, 14);
        assert_eq!(&request.payload[..6], &[b'0', b'3', b'0', b'3', 0x0a, 0x00]);

        let response = librasta_frame_packet(&LIBRASTA_C_CONNRESP);
        assert_eq!(response.packet_type, PacketType::ConnectionResponse);
        assert_eq!(response.receiver_id, 0x60);
        assert_eq!(response.sender_id, 0x61);
        assert_eq!(response.sequence_number, 0x0a0b_0c0d);
        assert_eq!(response.confirmed_sequence_number, 0x0102_0304);
        assert_eq!(response.timestamp, 0x5566_7788);
        assert_eq!(response.confirmed_timestamp, 0x1122_3344);
        assert_eq!(response.payload_len, 14);

        let heartbeat = librasta_frame_packet(&LIBRASTA_C_HEARTBEAT);
        assert_eq!(heartbeat.packet_type, PacketType::Heartbeat);
        assert_eq!(heartbeat.receiver_id, 0x60);
        assert_eq!(heartbeat.sender_id, 0x61);
        assert_eq!(heartbeat.sequence_number, 0x1112_1314);
        assert_eq!(heartbeat.confirmed_sequence_number, 0x0102_0304);
        assert_eq!(heartbeat.timestamp, 0x99aa_bbcc);
        assert_eq!(heartbeat.confirmed_timestamp, 0x1122_3344);
        assert_eq!(heartbeat.payload_len, 0);

        let disconnect = librasta_frame_packet(&LIBRASTA_C_DISCREQ);
        assert_eq!(disconnect.packet_type, PacketType::DisconnectionRequest);
        assert_eq!(disconnect.receiver_id, 0x61);
        assert_eq!(disconnect.sender_id, 0x60);
        assert_eq!(disconnect.sequence_number, 0x2122_2324);
        assert_eq!(disconnect.confirmed_sequence_number, 0x1112_1314);
        assert_eq!(disconnect.timestamp, 0xdead_beef);
        assert_eq!(disconnect.confirmed_timestamp, 0x99aa_bbcc);
        assert_eq!(disconnect.payload_len, 4);
    }

    #[test]
    fn librasta_type_a_no_sr_checksum_vectors_encode_exact_lengths_and_bytes() {
        for (vector, rl_sequence) in [
            (&LIBRASTA_C_CONNREQ[..], 0u32),
            (&LIBRASTA_C_CONNRESP[..], 1),
            (&LIBRASTA_C_HEARTBEAT[..], 2),
            (&LIBRASTA_C_DISCREQ[..], 3),
        ] {
            let packet = librasta_frame_packet(vector);
            let (encoded, encoded_len) = encode_librasta_frame(&packet, rl_sequence);
            assert_eq!(encoded_len, vector.len());
            assert_eq!(&encoded[..encoded_len], vector);
        }
    }

    #[test]
    fn librasta_type_a_none_profile_uses_c_observed_frame_lengths() {
        let safety = librasta_safety_none();
        let redundancy = RedundancyConfig {
            check_code: RedundancyCheckCode::OptionA,
            t_seq_ms: 50,
        };
        assert_eq!(safety.len(), 0);
        assert_eq!(redundancy.check_code_len(), 0);

        assert_eq!(LIBRASTA_C_CONNREQ.len(), 50);
        assert_eq!(LIBRASTA_C_CONNRESP.len(), 50);
        assert_eq!(LIBRASTA_C_HEARTBEAT.len(), 36);
        assert_eq!(LIBRASTA_C_DISCREQ.len(), 40);
    }

    #[test]
    fn sbb_local_profile_uses_sbb_observed_redl_datagram_lengths() {
        let profile = RastaProfile::sbb_local().unwrap();
        assert_eq!(profile, RastaProfile::SBB_LOCAL);
        assert_eq!(profile.network_identifier, 123_456);
        assert_eq!(profile.safety_code_length, SafetyCodeLength::Md4Lower8);
        assert_eq!(profile.redundancy_crc, RedundancyCrc::OptionA);
        assert_eq!(profile.t_max_ms, 750);
        assert_eq!(profile.t_h_ms, 300);
        assert_eq!(profile.t_seq_ms, 50);
        assert_eq!(
            profile.timestamp_compatibility,
            TimestampCompatibilityMode::PeerRelative
        );
        assert_eq!(
            profile.validate(),
            Err(ConfigError::UnsafeNoChecksumRequiresOptIn)
        );
        assert!(profile.validate_allowing_unsafe_no_checksums().is_ok());

        let safety = SafetyCodeConfig::md4_low8(profile.md4_initial_value);
        let redundancy = RedundancyConfig {
            check_code: RedundancyCheckCode::OptionA,
            t_seq_ms: profile.t_seq_ms,
        };
        assert_eq!(safety.len(), 8);
        assert_eq!(redundancy.check_code_len(), 0);

        let mut conn_req = packet(PacketType::ConnectionRequest, 14);
        conn_req.payload[0..4].copy_from_slice(&profile.protocol_version);
        conn_req.payload[4..6].copy_from_slice(&(profile.mwa as u16).to_le_bytes());
        let (_, conn_req_len) = encode_frame_with_safety(&conn_req, &safety, 0);
        assert_eq!(conn_req_len, 58);

        let heartbeat = packet(PacketType::Heartbeat, 0);
        let (_, heartbeat_len) = encode_frame_with_safety(&heartbeat, &safety, 1);
        assert_eq!(heartbeat_len, 44);

        let mut disconnect = packet(PacketType::DisconnectionRequest, 4);
        disconnect.payload[0..4].copy_from_slice(&0u32.to_le_bytes());
        let (_, disconnect_len) = encode_frame_with_safety(&disconnect, &safety, 2);
        assert_eq!(disconnect_len, 48);
    }

    #[test]
    fn pdu_message_types_lengths_and_payload_rules_are_enforced() {
        let safety = SafetyCodeConfig::default();
        let mut buffer = [0u8; 512];

        for packet_type in [
            PacketType::ConnectionRequest,
            PacketType::ConnectionResponse,
            PacketType::RetransmissionRequest,
            PacketType::RetransmissionResponse,
            PacketType::DisconnectionRequest,
            PacketType::Heartbeat,
            PacketType::Data,
            PacketType::RetransmissionData,
        ] {
            let mut p = packet(
                packet_type,
                match packet_type {
                    PacketType::ConnectionRequest | PacketType::ConnectionResponse => 14,
                    PacketType::DisconnectionRequest => 4,
                    _ => 0,
                },
            );
            if matches!(
                packet_type,
                PacketType::ConnectionRequest | PacketType::ConnectionResponse
            ) {
                p.payload[0..4].copy_from_slice(b"0303");
            }
            let len = p.serialize(&mut buffer, &safety).unwrap();
            let parsed = Packet::parse(&buffer[..len], &safety).unwrap();
            assert_eq!(parsed.packet_type, packet_type);
            assert_eq!(parsed.payload_len, p.payload_len);
        }

        let mut heartbeat = packet(PacketType::Heartbeat, 0);
        let exact_len = heartbeat.serialize(&mut buffer, &safety).unwrap();
        assert!(matches!(
            Packet::parse(&buffer[..exact_len - 1], &safety),
            Err(PacketError::BufferTooSmall)
        ));
        let mut with_extra = [0u8; 520];
        with_extra[..exact_len].copy_from_slice(&buffer[..exact_len]);
        assert!(matches!(
            Packet::parse(&with_extra[..exact_len + 1], &safety),
            Err(PacketError::InvalidLength)
        ));

        with_extra[..exact_len].copy_from_slice(&buffer[..exact_len]);
        with_extra[0..2].copy_from_slice(&27u16.to_le_bytes());
        assert!(matches!(
            Packet::parse(&with_extra[..exact_len], &safety),
            Err(PacketError::InvalidLength)
        ));

        with_extra[..exact_len].copy_from_slice(&buffer[..exact_len]);
        with_extra[2..4].copy_from_slice(&9999u16.to_le_bytes());
        assert!(matches!(
            Packet::parse(&with_extra[..exact_len], &safety),
            Err(PacketError::InvalidType)
        ));

        heartbeat.payload_len = 1;
        assert!(matches!(
            heartbeat.serialize(&mut buffer, &safety),
            Err(PacketError::InvalidPayload)
        ));
    }

    #[test]
    fn retransmission_request_uses_zero_payload_and_confirmed_sequence_point() {
        let safety = SafetyCodeConfig::default();
        let mut request = packet(PacketType::RetransmissionRequest, 0);
        request.confirmed_sequence_number = 41;
        let mut buffer = [0u8; 512];
        let len = request.serialize(&mut buffer, &safety).unwrap();
        assert_eq!(len, Packet::HEADER_SIZE + safety.len());

        let parsed = Packet::parse(&buffer[..len], &safety).unwrap();
        assert_eq!(parsed.packet_type, PacketType::RetransmissionRequest);
        assert_eq!(parsed.payload_len, 0);
        assert_eq!(parsed.confirmed_sequence_number, 41);

        request.payload_len = 4;
        request.payload[..4].copy_from_slice(&42u32.to_le_bytes());
        assert!(matches!(
            request.serialize(&mut buffer, &safety),
            Err(PacketError::InvalidPayload)
        ));
    }

    #[test]
    fn pdu_connection_version_and_max_payload_boundaries_are_enforced() {
        let safety = SafetyCodeConfig::default();
        let mut buffer = [0u8; 512];
        let mut request = packet(PacketType::ConnectionRequest, 14);
        request.payload[0..4].copy_from_slice(b"0301");
        assert!(matches!(
            request.serialize(&mut buffer, &safety),
            Err(PacketError::UnsupportedProtocolVersion)
        ));

        let mut data = packet(PacketType::Data, Packet::MAX_PAYLOAD_SIZE);
        for (index, byte) in data.payload.iter_mut().enumerate() {
            *byte = index as u8;
        }
        let len = data.serialize(&mut buffer, &safety).unwrap();
        let parsed = Packet::parse(&buffer[..len], &safety).unwrap();
        assert_eq!(parsed.payload_len, Packet::MAX_PAYLOAD_SIZE);
        assert_eq!(parsed.payload, data.payload);

        data.payload_len = Packet::MAX_PAYLOAD_SIZE + 1;
        assert!(matches!(
            data.serialize(&mut buffer, &safety),
            Err(PacketError::InvalidLength)
        ));
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
    fn state_machine_all_implemented_transitions_and_rejections() {
        let states = [
            RastaState::Closed,
            RastaState::Down,
            RastaState::Start,
            RastaState::Up,
            RastaState::RetransmissionRequested,
            RastaState::RetransmissionRunning,
        ];
        let legal = [
            (RastaState::Closed, RastaState::Down),
            (RastaState::Down, RastaState::Start),
            (RastaState::Down, RastaState::Closed),
            (RastaState::Start, RastaState::Up),
            (RastaState::Start, RastaState::Closed),
            (RastaState::Up, RastaState::RetransmissionRequested),
            (RastaState::Up, RastaState::Closed),
            (
                RastaState::RetransmissionRequested,
                RastaState::RetransmissionRunning,
            ),
            (RastaState::RetransmissionRequested, RastaState::Closed),
            (
                RastaState::RetransmissionRunning,
                RastaState::RetransmissionRequested,
            ),
            (RastaState::RetransmissionRunning, RastaState::Up),
            (RastaState::RetransmissionRunning, RastaState::Closed),
        ];

        for &from in &states {
            for &to in &states {
                let mut sm = StateMachine {
                    current_state: from,
                };
                let expected = from == to || legal.contains(&(from, to));
                assert_eq!(sm.transition(to), expected, "{from:?} -> {to:?}");
                assert_eq!(sm.current_state, if expected { to } else { from });
            }
        }
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
    fn sequencing_duplicates_gaps_range_and_wraparound_are_classified() {
        let mut sh = SequenceHandler::with_initial_tx(u32::MAX);
        assert_eq!(sh.next_tx(), u32::MAX);
        assert_eq!(sh.next_tx(), 0);
        assert_eq!(sh.next_tx_value(), 1);

        sh.accept_initial_rx(u32::MAX - 1);
        assert_eq!(sh.expected_rx(), u32::MAX);
        assert_eq!(sh.validate_rx(u32::MAX), SequenceResult::Ok);
        // Existing behavior: once current_rx wraps to zero, confirmed_seq()
        // returns its zero sentinel rather than u32::MAX.
        assert_eq!(sh.confirmed_seq(), 0);
        assert_eq!(sh.last_received_seq(), None);
        assert_eq!(sh.validate_rx(u32::MAX), SequenceResult::Duplicate);
        assert_eq!(sh.validate_rx(2), SequenceResult::Gap(0));
        assert!(sh.validate_range(10, 2));
        assert!(!sh.validate_range(21, 2));
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
    fn timestamp_validation_covers_future_boundary_and_half_range() {
        let supervisor = TimeSupervisor::new(100);
        let timestamp = ProtocolTimestamp::from_wire_millis;
        assert!(
            supervisor
                .validate(timestamp(1000), timestamp(1100))
                .is_ok()
        );
        assert_eq!(
            supervisor.validate(timestamp(1000), timestamp(1101)),
            Err(TimeSupervisionError::TimestampTooFarInFuture)
        );
        assert_eq!(
            supervisor.validate(timestamp(0), timestamp(0x8000_0000)),
            Err(TimeSupervisionError::TimestampTooFarInFuture)
        );
    }

    #[test]
    fn unsynchronized_protocol_timestamp_offsets_hit_future_tolerance_boundary() {
        let supervisor = TimeSupervisor::new(2_000);
        let timestamp = ProtocolTimestamp::from_wire_millis;

        assert!(supervisor.validate(timestamp(0), timestamp(100)).is_ok());
        for offset in [999u32, 1_000, 1_001, 5_000] {
            assert_eq!(
                supervisor.validate(timestamp(0), timestamp(offset)),
                Err(TimeSupervisionError::TimestampTooFarInFuture)
            );
        }
        assert!(
            supervisor
                .validate(timestamp(u32::MAX - 50), timestamp(49))
                .is_ok()
        );
        assert_eq!(
            supervisor.validate(timestamp(u32::MAX - 50), timestamp(50)),
            Err(TimeSupervisionError::TimestampTooFarInFuture)
        );
    }

    #[test]
    fn confirmed_timestamp_validation_covers_progression_repeat_future_and_wrap() {
        let supervisor = TimeSupervisor::new(200);
        let timestamp = ProtocolTimestamp::from_wire_millis;
        let repeated = supervisor
            .validate_confirmed_timestamp(timestamp(100), timestamp(90), timestamp(90))
            .unwrap();
        assert_eq!(repeated.round_trip, DurationMs::from_millis(10));

        let progressed = supervisor
            .validate_confirmed_timestamp(timestamp(120), timestamp(90), timestamp(100))
            .unwrap();
        assert_eq!(progressed.confirmed_timestamp, timestamp(100));

        assert_eq!(
            supervisor.validate_confirmed_timestamp(timestamp(120), timestamp(100), timestamp(99)),
            Err(TimeSupervisionError::ConfirmedTimestampMovedBackwards)
        );
        assert_eq!(
            supervisor.validate_confirmed_timestamp(timestamp(120), timestamp(100), timestamp(121)),
            Err(TimeSupervisionError::ConfirmedTimestampTooFarInFuture)
        );

        let wrapped = supervisor
            .validate_confirmed_timestamp(timestamp(10), timestamp(u32::MAX - 5), timestamp(5))
            .unwrap();
        assert_eq!(wrapped.round_trip, DurationMs::from_millis(5));
    }

    #[test]
    fn peer_relative_timestamp_validation_accepts_offsets_and_wraparound() {
        let supervisor = TimeSupervisor::new(10_000);
        let timestamp = ProtocolTimestamp::from_wire_millis;

        let positive = supervisor
            .validate_peer_relative(
                timestamp(10_000),
                timestamp(9_000),
                DurationMs::from_millis(1_000),
            )
            .unwrap();
        assert_eq!(positive.normalized_timestamp, timestamp(10_000));

        let negative = supervisor
            .validate_peer_relative(
                timestamp(10_000),
                timestamp(11_000),
                DurationMs::from_millis(u32::MAX - 999),
            )
            .unwrap();
        assert_eq!(negative.normalized_timestamp, timestamp(10_000));

        let wrapped = supervisor
            .validate_peer_relative(
                timestamp(5),
                timestamp(u32::MAX - 4),
                DurationMs::from_millis(10),
            )
            .unwrap();
        assert_eq!(wrapped.normalized_timestamp, timestamp(5));
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
    fn retransmission_capacity_confirmation_and_wraparound_are_deterministic() {
        let mut rb = RetransmissionBuffer::with_capacity(2);
        let mut first = packet(PacketType::Data, 3);
        first.sequence_number = u32::MAX - 1;
        first.payload[..3].copy_from_slice(b"one");
        let mut second = packet(PacketType::Data, 3);
        second.sequence_number = u32::MAX;
        second.payload[..3].copy_from_slice(b"two");
        let mut third = packet(PacketType::Data, 5);
        third.sequence_number = 0;
        third.payload[..5].copy_from_slice(b"three");

        assert!(rb.store(first.clone()));
        assert!(rb.store(second.clone()));
        assert!(!rb.store(third));
        assert_eq!(rb.count(), 2);
        assert_eq!(&rb.get_packet(u32::MAX - 1).unwrap().payload[..3], b"one");
        assert_eq!(&rb.get_packet(u32::MAX).unwrap().payload[..3], b"two");

        rb.clear_up_to(u32::MAX - 1);
        assert_eq!(rb.count(), 1);
        assert!(rb.get_packet(u32::MAX - 1).is_none());
        assert!(rb.get_packet(u32::MAX).is_some());

        rb.clear_up_to(u32::MAX);
        assert_eq!(rb.count(), 0);
    }

    #[test]
    fn confirmed_sequence_first_duplicate_single_cumulative_and_boundaries_release_exactly() {
        let (mut connection, clock) = confirmation_connection(10, 8, 4);
        for payload in [b"a".as_slice(), b"b", b"c", b"d"] {
            connection.send_application_data(payload).unwrap();
        }
        assert_eq!(connection.retransmission.count(), 4);

        let mut heartbeat = ack_heartbeat(0, 9, 1_000);
        receive_packet(&mut connection, &heartbeat);
        connection.process().unwrap();
        assert_eq!(connection.last_peer_confirmed_sequence_for_test(), Some(9));
        assert_eq!(connection.retransmission.count(), 4);

        clock.set(1_001);
        heartbeat = ack_heartbeat(1, 9, 1_001);
        receive_packet(&mut connection, &heartbeat);
        connection.process().unwrap();
        assert_eq!(connection.last_peer_confirmed_sequence_for_test(), Some(9));
        assert_eq!(connection.retransmission.count(), 4);

        clock.set(1_002);
        heartbeat = ack_heartbeat(2, 10, 1_002);
        receive_packet(&mut connection, &heartbeat);
        connection.process().unwrap();
        assert_eq!(connection.last_peer_confirmed_sequence_for_test(), Some(10));
        assert!(connection.retransmission.get_packet(10).is_none());
        assert!(connection.retransmission.get_packet(11).is_some());
        assert_eq!(connection.retransmission.count(), 3);

        clock.set(1_003);
        heartbeat = ack_heartbeat(3, 12, 1_003);
        receive_packet(&mut connection, &heartbeat);
        connection.process().unwrap();
        assert_eq!(connection.last_peer_confirmed_sequence_for_test(), Some(12));
        assert!(connection.retransmission.get_packet(11).is_none());
        assert!(connection.retransmission.get_packet(12).is_none());
        assert!(connection.retransmission.get_packet(13).is_some());
        assert_eq!(connection.retransmission.count(), 1);

        clock.set(1_004);
        heartbeat = ack_heartbeat(4, 13, 1_004);
        receive_packet(&mut connection, &heartbeat);
        connection.process().unwrap();
        assert_eq!(connection.last_peer_confirmed_sequence_for_test(), Some(13));
        assert_eq!(connection.retransmission.count(), 0);
    }

    #[test]
    fn confirmed_sequence_initial_values_zero_one_max_and_before_max_are_not_sentinels() {
        for initial in [0, 1, u32::MAX, u32::MAX - 1] {
            let (mut connection, clock) = confirmation_connection(initial, 4, 2);
            connection.send_application_data(b"x").unwrap();
            assert!(connection.retransmission.get_packet(initial).is_some());

            let heartbeat = ack_heartbeat(0, initial, 1_000);
            receive_packet(&mut connection, &heartbeat);
            connection.process().unwrap();

            assert_eq!(
                connection.last_peer_confirmed_sequence_for_test(),
                Some(initial)
            );
            assert_eq!(connection.retransmission.count(), 0);
            assert_eq!(connection.state_machine.current_state, RastaState::Up);
            clock.set(clock.now().wrapping_millis().wrapping_add(1));
        }
    }

    #[test]
    fn confirmed_sequence_with_empty_retransmission_buffer_updates_ack_without_release() {
        let (mut connection, _) = confirmation_connection(5, 4, 2);
        assert_eq!(connection.retransmission.count(), 0);

        let heartbeat = ack_heartbeat(0, 4, 1_000);
        receive_packet(&mut connection, &heartbeat);
        connection.process().unwrap();

        assert_eq!(connection.last_peer_confirmed_sequence_for_test(), Some(4));
        assert_eq!(connection.retransmission.count(), 0);
        assert_eq!(connection.state_machine.current_state, RastaState::Up);
    }

    #[test]
    fn wraparound_confirmation_releases_only_confirmed_window_entries() {
        let (mut connection, clock) = confirmation_connection(u32::MAX - 1, 4, 2);
        connection.send_application_data(b"a").unwrap();
        connection.send_application_data(b"b").unwrap();
        connection.send_application_data(b"c").unwrap();
        assert!(connection.retransmission.get_packet(u32::MAX - 1).is_some());
        assert!(connection.retransmission.get_packet(u32::MAX).is_some());
        assert!(connection.retransmission.get_packet(0).is_some());

        let mut heartbeat = ack_heartbeat(0, u32::MAX, 1_000);
        receive_packet(&mut connection, &heartbeat);
        connection.process().unwrap();
        assert!(connection.retransmission.get_packet(u32::MAX - 1).is_none());
        assert!(connection.retransmission.get_packet(u32::MAX).is_none());
        assert!(connection.retransmission.get_packet(0).is_some());
        assert_eq!(connection.retransmission.count(), 1);

        clock.set(1_001);
        heartbeat = ack_heartbeat(1, 0, 1_001);
        receive_packet(&mut connection, &heartbeat);
        connection.process().unwrap();
        assert_eq!(connection.retransmission.count(), 0);
    }

    #[test]
    fn invalid_confirmations_disconnect_without_releasing_or_delivering() {
        let (mut connection, clock) = confirmation_connection(20, 4, 2);
        connection.send_application_data(b"a").unwrap();
        connection.send_application_data(b"b").unwrap();
        assert_eq!(connection.retransmission.count(), 2);

        let valid = ack_heartbeat(0, 20, 1_000);
        receive_packet(&mut connection, &valid);
        connection.process().unwrap();
        assert_eq!(connection.retransmission.count(), 1);
        assert_eq!(connection.sequence.expected_rx(), 1);
        assert_eq!(connection.last_peer_confirmed_sequence_for_test(), Some(20));

        clock.set(1_001);
        let invalid = data_ack(1, 19, 1_001, b"bad");
        receive_packet(&mut connection, &invalid);
        assert!(matches!(
            connection.process(),
            Err(ConnectionError::ProtocolViolation)
        ));

        assert_eq!(connection.state_machine.current_state, RastaState::Closed);
        assert_eq!(connection.sequence.expected_rx(), 1);
        assert_eq!(connection.retransmission.count(), 1);
        assert!(connection.retransmission.get_packet(21).is_some());
        assert!(!connection.has_received_data());
        assert_eq!(connection.last_peer_confirmed_sequence_for_test(), Some(20));
        assert_eq!(connection.error_counters().confirmed_sequence_number, 1);
        assert_eq!(
            connection.take_diagnostic().map(|event| event.kind),
            Some(DiagnosticKind::ConfirmedSequenceError)
        );
    }

    #[test]
    fn confirmation_of_unsent_future_and_half_range_ambiguous_values_are_rejected() {
        let (mut future, _) = confirmation_connection(0, 4, 2);
        future.send_application_data(b"x").unwrap();
        let future_ack = ack_heartbeat(0, 1, 1_000);
        receive_packet(&mut future, &future_ack);
        assert!(matches!(
            future.process(),
            Err(ConnectionError::ProtocolViolation)
        ));
        assert_eq!(future.retransmission.count(), 1);

        let (mut ambiguous, _) = confirmation_connection(1, 4, 2);
        ambiguous.send_application_data(b"x").unwrap();
        let ambiguous_ack = ack_heartbeat(0, 0x8000_0000, 1_000);
        receive_packet(&mut ambiguous, &ambiguous_ack);
        assert!(matches!(
            ambiguous.process(),
            Err(ConnectionError::ProtocolViolation)
        ));
        assert_eq!(ambiguous.retransmission.count(), 1);
    }

    #[test]
    fn invalid_confirmation_does_not_reopen_flow_control_but_valid_ack_does() {
        let (mut connection, clock) = confirmation_connection(0, 4, 2);
        for _ in 0..4 {
            connection.send_application_data(b"x").unwrap();
        }
        assert_eq!(connection.retransmission.count(), 4);
        connection.send_application_data(b"queued").unwrap();
        assert_eq!(connection.queued_application_tx_count_for_test(), 1);

        let invalid = ack_heartbeat(0, 4, 1_000);
        receive_packet(&mut connection, &invalid);
        assert!(matches!(
            connection.process(),
            Err(ConnectionError::ProtocolViolation)
        ));
        assert_eq!(connection.retransmission.count(), 4);
        assert_eq!(connection.queued_application_tx_count_for_test(), 1);

        let (mut connection, clock2) = confirmation_connection(0, 4, 2);
        for _ in 0..4 {
            connection.send_application_data(b"x").unwrap();
        }
        connection.send_application_data(b"queued").unwrap();
        assert_eq!(connection.queued_application_tx_count_for_test(), 1);

        let valid = ack_heartbeat(0, 1, 1_000);
        receive_packet(&mut connection, &valid);
        connection.process().unwrap();
        assert_eq!(connection.queued_application_tx_count_for_test(), 0);
        assert_eq!(connection.retransmission.count(), 3);
        assert!(connection.retransmission.get_packet(0).is_none());
        assert!(connection.retransmission.get_packet(1).is_none());
        assert!(connection.retransmission.get_packet(4).is_some());
        clock.set(1_001);
        clock2.set(1_001);
    }

    #[test]
    fn retransmission_request_point_is_not_processed_as_cumulative_ack() {
        let (mut connection, _) = confirmation_connection(0, 8, 4);
        connection.send_application_data(b"zero").unwrap();
        connection.send_application_data(b"one").unwrap();
        assert_eq!(connection.retransmission.count(), 2);

        let mut request = ack_heartbeat(0, 0, 1_000);
        request.packet_type = PacketType::RetransmissionRequest;
        receive_packet(&mut connection, &request);
        connection.process().unwrap();

        assert_eq!(connection.retransmission.count(), 2);
        assert!(connection.retransmission.get_packet(0).is_some());
        assert!(connection.retransmission.get_packet(1).is_some());
        assert_eq!(connection.last_peer_confirmed_sequence_for_test(), None);
    }

    #[test]
    fn retransmission_data_confirmation_is_classified_and_releases_retained_packets() {
        let (mut connection, _) = confirmation_connection(0, 8, 4);
        connection.send_application_data(b"zero").unwrap();
        connection.send_application_data(b"one").unwrap();
        assert_eq!(connection.retransmission.count(), 2);

        let mut data = data_ack(0, 1, 1_000, b"peer");
        data.packet_type = PacketType::RetransmissionData;
        receive_packet(&mut connection, &data);
        connection.process().unwrap();

        assert_eq!(connection.retransmission.count(), 0);
        assert!(connection.has_received_data());
    }

    #[test]
    fn invalid_confirmation_in_retransmission_state_does_not_transition_or_release() {
        let (mut connection, _) = confirmation_connection(0, 4, 2);
        connection.send_application_data(b"x").unwrap();
        connection
            .transition(RastaState::RetransmissionRequested)
            .unwrap();

        let mut response = ack_heartbeat(0, 1, 1_000);
        response.packet_type = PacketType::RetransmissionResponse;
        receive_packet(&mut connection, &response);
        assert!(matches!(
            connection.process(),
            Err(ConnectionError::ProtocolViolation)
        ));

        assert_eq!(connection.state_machine.current_state, RastaState::Closed);
        assert_eq!(connection.retransmission.count(), 1);
    }

    #[test]
    fn retransmit_from_validates_window_and_propagates_transport_failure() {
        let mut empty = RastaConnection::try_new(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            MockClock { time: 0 },
            config(1, 2),
        )
        .unwrap();
        assert!(matches!(
            empty.retransmit_from(0),
            Err(ConnectionError::RetransmissionUnavailable)
        ));

        let mut sender = RastaConnection::try_new(
            FailingTransport,
            FailingTransport,
            MockClock { time: 0 },
            config(1, 2),
        )
        .unwrap();
        let mut stored = packet(PacketType::Data, 4);
        stored.sequence_number = 7;
        stored.payload[..4].copy_from_slice(b"data");
        assert!(sender.retransmission.store(stored));
        assert!(matches!(
            sender.retransmit_from(6),
            Err(ConnectionError::RetransmissionUnavailable)
        ));
        assert!(matches!(
            sender.retransmit_from(8),
            Err(ConnectionError::RetransmissionUnavailable)
        ));
        assert!(matches!(
            sender.retransmit_from(7),
            Err(ConnectionError::Transport(TransportError::SendFailed))
        ));
        assert_eq!(sender.retransmission.count(), 1);
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
    fn active_open_does_not_emit_heartbeat_before_connection_response() {
        let network = Rc::new(RefCell::new(TestNetwork::new()));
        let time = Rc::new(Cell::new(0));
        let mut client = RastaConnection::try_new(
            LinkedTransport::new(network.clone(), 0),
            LinkedTransport::new(network.clone(), 1),
            SharedClock(time.clone()),
            config(1, 2),
        )
        .unwrap();

        client.connect().unwrap();
        assert_eq!(client.state_machine.current_state, RastaState::Start);
        assert_eq!(
            network_packet_type_count(
                &network,
                &[2, 3],
                &client.safety_code,
                PacketType::ConnectionRequest
            ),
            2
        );
        assert_eq!(
            network_packet_type_count(
                &network,
                &[2, 3],
                &client.safety_code,
                PacketType::Heartbeat
            ),
            0
        );

        for now in [500u32, 1_000, 1_500] {
            time.set(now);
            client.process().unwrap();
            assert_eq!(client.state_machine.current_state, RastaState::Start);
            assert_eq!(
                network_packet_type_count(
                    &network,
                    &[2, 3],
                    &client.safety_code,
                    PacketType::Heartbeat
                ),
                0
            );
        }
    }

    #[test]
    fn active_heartbeat_starts_only_after_valid_connection_response() {
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
            LinkedTransport::new(network.clone(), 3),
            SharedClock(time),
            config(2, 1),
        )
        .unwrap();

        client.connect().unwrap();
        server.connect().unwrap();
        server.process().unwrap();
        assert_eq!(
            network_packet_type_count(
                &network,
                &[2, 3],
                &client.safety_code,
                PacketType::Heartbeat
            ),
            0
        );

        client.process().unwrap();
        assert_eq!(client.state_machine.current_state, RastaState::Up);
        assert_eq!(
            network_packet_type_count(
                &network,
                &[2, 3],
                &client.safety_code,
                PacketType::Heartbeat
            ),
            2
        );
    }

    #[test]
    fn passive_rejects_heartbeat_before_connection_request() {
        let clock = FakeClock::new(0);
        let mut server = RastaConnection::try_new(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            clock,
            config(2, 1),
        )
        .unwrap();
        server.connect().unwrap();

        let mut heartbeat = ack_heartbeat(0, 0, 0);
        heartbeat.receiver_id = 2;
        heartbeat.sender_id = 1;
        server.redundancy = RedundancyLayer::with_config(
            receive_packet_transport(&heartbeat, &server.safety_code),
            SimpleMockTransport::empty(),
            RedundancyConfig {
                check_code: RedundancyCheckCode::None,
                t_seq_ms: 100,
            },
        );

        assert!(matches!(
            server.process(),
            Err(ConnectionError::UnexpectedPacket)
        ));
        assert_eq!(server.state_machine.current_state, RastaState::Down);
    }

    #[test]
    fn complete_handshake_wire_order_is_request_response_then_heartbeats() {
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
            LinkedTransport::new(network.clone(), 3),
            SharedClock(time),
            config(2, 1),
        )
        .unwrap();

        client.connect().unwrap();
        assert_eq!(
            network_head_packet_type(&network, 2, &client.safety_code),
            Some(PacketType::ConnectionRequest)
        );

        server.connect().unwrap();
        server.process().unwrap();
        assert_eq!(
            network_head_packet_type(&network, 0, &server.safety_code),
            Some(PacketType::ConnectionResponse)
        );

        client.process().unwrap();
        assert_eq!(
            network_head_packet_type(&network, 2, &client.safety_code),
            Some(PacketType::Heartbeat)
        );

        server.process().unwrap();
        assert_eq!(
            network_head_packet_type(&network, 0, &server.safety_code),
            Some(PacketType::Heartbeat)
        );

        client.process().unwrap();
        assert_eq!(client.state_machine.current_state, RastaState::Up);
        assert_eq!(server.state_machine.current_state, RastaState::Up);
    }

    #[test]
    fn stale_pre_handshake_heartbeat_cannot_establish_connection() {
        let clock = FakeClock::new(0);
        let mut client = RastaConnection::try_new(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            clock,
            config(1, 2),
        )
        .unwrap();
        client.connect().unwrap();

        let mut heartbeat = ack_heartbeat(0, 0, 0);
        heartbeat.receiver_id = 1;
        heartbeat.sender_id = 2;
        client.redundancy = RedundancyLayer::with_config(
            receive_packet_transport(&heartbeat, &client.safety_code),
            SimpleMockTransport::empty(),
            RedundancyConfig {
                check_code: RedundancyCheckCode::None,
                t_seq_ms: 100,
            },
        );

        assert!(matches!(
            client.process(),
            Err(ConnectionError::UnexpectedPacket)
        ));
        assert_ne!(client.state_machine.current_state, RastaState::Up);
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
    fn single_redundancy_channel_send_failure_is_diagnostic_but_connection_continues() {
        let mut connection = RastaConnection::try_new(
            FailingTransport,
            SimpleMockTransport::empty(),
            MockClock { time: 0 },
            config(1, 2),
        )
        .unwrap();
        connection.transition(RastaState::Down).unwrap();
        connection.transition(RastaState::Start).unwrap();
        connection.transition(RastaState::Up).unwrap();

        connection.send_application_data(b"x").unwrap();
        assert_eq!(
            connection.channel_statuses(),
            [ChannelStatus::Degraded, ChannelStatus::Healthy]
        );
        assert_eq!(
            connection.take_diagnostic().map(|event| event.kind),
            Some(DiagnosticKind::ChannelSupervisionFailure)
        );
        assert_eq!(connection.state_machine.current_state, RastaState::Up);
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
        for now in [300u32, 600, 900, 1_200, 1_500, 1_800, 1_999] {
            time.set(now);
            client
                .process()
                .unwrap_or_else(|error| panic!("client first poll failed at {now}: {error:?}"));
            server
                .process()
                .unwrap_or_else(|error| panic!("server first poll failed at {now}: {error:?}"));
            client
                .process()
                .unwrap_or_else(|error| panic!("client second poll failed at {now}: {error:?}"));
            server
                .process()
                .unwrap_or_else(|error| panic!("server second poll failed at {now}: {error:?}"));
            assert_eq!(client.state_machine.current_state, RastaState::Up);
            assert_eq!(server.state_machine.current_state, RastaState::Up);
        }
    }

    #[test]
    fn active_client_path_schedules_and_emits_heartbeat_after_up() {
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
            LinkedTransport::new(network.clone(), 3),
            SharedClock(time.clone()),
            config(2, 1),
        )
        .unwrap();

        client.connect().unwrap();
        server.connect().unwrap();
        server.process().unwrap();
        client.process().unwrap();

        assert_eq!(client.state_machine.current_state, RastaState::Up);
        assert_eq!(
            network_packet_type_count(
                &network,
                &[2, 3],
                &client.safety_code,
                PacketType::Heartbeat
            ),
            2
        );

        clear_network_channels(&network, &[2, 3]);
        time.set(500);
        client.process().unwrap();
        assert_eq!(
            network_packet_type_count(
                &network,
                &[2, 3],
                &client.safety_code,
                PacketType::Heartbeat
            ),
            2
        );
    }

    #[test]
    fn passive_server_path_schedules_and_emits_heartbeat_after_up() {
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
            LinkedTransport::new(network.clone(), 3),
            SharedClock(time.clone()),
            config(2, 1),
        )
        .unwrap();

        client.connect().unwrap();
        server.connect().unwrap();
        server.process().unwrap();
        client.process().unwrap();
        server.process().unwrap();

        assert_eq!(server.state_machine.current_state, RastaState::Up);
        assert_eq!(
            network_packet_type_count(
                &network,
                &[0, 1],
                &server.safety_code,
                PacketType::Heartbeat
            ),
            2
        );

        clear_network_channels(&network, &[0, 1]);
        time.set(500);
        server.process().unwrap();
        assert_eq!(
            network_packet_type_count(
                &network,
                &[0, 1],
                &server.safety_code,
                PacketType::Heartbeat
            ),
            2
        );
    }

    #[test]
    fn both_endpoints_emit_heartbeats_across_multiple_periods_without_timeout() {
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
            LinkedTransport::new(network.clone(), 3),
            SharedClock(time.clone()),
            config(2, 1),
        )
        .unwrap();

        client.connect().unwrap();
        server.connect().unwrap();
        server.process().unwrap();
        client.process().unwrap();
        server.process().unwrap();
        client.process().unwrap();
        clear_network_channels(&network, &[0, 1, 2, 3]);

        let mut client_sent_heartbeats = 0usize;
        let mut server_sent_heartbeats = 0usize;
        for now in [500u32, 1_000, 1_500, 2_000, 2_500] {
            time.set(now);
            client.process().unwrap();
            client_sent_heartbeats += network_packet_type_count(
                &network,
                &[2, 3],
                &client.safety_code,
                PacketType::Heartbeat,
            );

            server.process().unwrap();
            assert_eq!(
                server
                    .timeliness_deadline_for_test()
                    .map(|deadline| deadline.wrapping_millis()),
                Some(now + 2_000)
            );
            server_sent_heartbeats += network_packet_type_count(
                &network,
                &[0, 1],
                &server.safety_code,
                PacketType::Heartbeat,
            );

            client.process().unwrap();
            assert_eq!(
                client
                    .timeliness_deadline_for_test()
                    .map(|deadline| deadline.wrapping_millis()),
                Some(now + 2_000)
            );
            assert_eq!(client.state_machine.current_state, RastaState::Up);
            assert_eq!(server.state_machine.current_state, RastaState::Up);
            clear_network_channels(&network, &[0, 1, 2, 3]);
        }

        assert!(client_sent_heartbeats >= 6);
        assert!(server_sent_heartbeats >= 6);
    }

    #[test]
    fn process_relative_protocol_timestamp_offset_reproduces_future_rejection() {
        let network = Rc::new(RefCell::new(TestNetwork::new()));
        let client_clock = SplitClock::new(0, 0);
        let server_clock = SplitClock::new(1_000, 1_000);
        let mut client = RastaConnection::try_new(
            LinkedTransport::new(network.clone(), 0),
            LinkedTransport::new(network.clone(), 1),
            client_clock,
            config(1, 2),
        )
        .unwrap();
        let mut server = RastaConnection::try_new(
            LinkedTransport::new(network.clone(), 2),
            LinkedTransport::new(network.clone(), 3),
            server_clock,
            config(2, 1),
        )
        .unwrap();

        client.connect().unwrap();
        server.connect().unwrap();
        server.process().unwrap();
        client.process().unwrap();
        server.process().unwrap();
        assert_eq!(
            network_head_packet_type(&network, 0, &server.safety_code),
            Some(PacketType::Heartbeat)
        );

        assert!(matches!(
            client.process(),
            Err(ConnectionError::SafetyTimeout)
        ));
        assert_eq!(client.state_machine.current_state, RastaState::Closed);
        assert_eq!(
            client.take_diagnostic(),
            Some(DiagnosticEvent {
                kind: DiagnosticKind::ConnectionTimeout,
                value: 1_000,
            })
        );
    }

    #[test]
    fn librasta_peer_relative_live_values_accept_heartbeat_and_refresh_deadline() {
        let clock = FakeClock::new(0x1628_c50b);
        let mut client = RastaConnection::try_new(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            clock.clone(),
            peer_relative_config(0x60, 0x61),
        )
        .unwrap();
        client.connect().unwrap();

        let mut response = packet(PacketType::ConnectionResponse, 14);
        response.receiver_id = 0x60;
        response.sender_id = 0x61;
        response.sequence_number = 0;
        response.confirmed_sequence_number = 0;
        response.timestamp = 0x008b_e8ae;
        response.confirmed_timestamp = 0x1628_c50b;
        response.payload[..4].copy_from_slice(b"0303");
        response.payload[4..6].copy_from_slice(&20u16.to_le_bytes());
        inject_received_packet(&mut client, &response);
        client.process().unwrap();
        assert_eq!(client.state_machine.current_state, RastaState::Up);

        clock.set(0x1628_c50c);
        client.send_application_data(b"Hello from A").unwrap();

        let mut heartbeat = packet(PacketType::Heartbeat, 0);
        heartbeat.receiver_id = 0x60;
        heartbeat.sender_id = 0x61;
        heartbeat.sequence_number = 1;
        heartbeat.confirmed_sequence_number = 2;
        heartbeat.timestamp = 0x008b_e8af;
        heartbeat.confirmed_timestamp = 0x1628_c50c;
        inject_received_packet(&mut client, &heartbeat);
        client.process().unwrap();

        assert_eq!(client.state_machine.current_state, RastaState::Up);
        assert_eq!(
            client
                .timeliness_deadline_for_test()
                .map(MonotonicInstant::wrapping_millis),
            Some(0x1628_ec1c)
        );
        let trace = client.take_timestamp_trace().unwrap();
        assert_eq!(trace.raw_peer_timestamp, 0x008b_e8af);
        assert_eq!(trace.normalized_peer_timestamp, 0x1628_c50c);
        assert_eq!(trace.local_timestamp, 0x1628_c50c);
        assert_eq!(trace.confirmed_timestamp, 0x1628_c50c);
        assert_eq!(trace.rejection, None);
        while let Some(diagnostic) = client.take_diagnostic() {
            assert_ne!(diagnostic.kind, DiagnosticKind::ConnectionTimeout);
        }
    }

    #[test]
    fn peer_relative_rejects_stale_peer_timestamp() {
        let clock = FakeClock::new(1_000);
        let mut conn = RastaConnection::try_new(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            clock.clone(),
            peer_relative_config(1, 2),
        )
        .unwrap();
        conn.connect().unwrap();
        conn.sequence.accept_initial_rx(9);
        conn.transition(RastaState::Up).unwrap();
        conn.learn_peer_timestamp_offset_for_test(1_000, 5_000);

        let mut valid = packet(PacketType::Heartbeat, 0);
        valid.receiver_id = 1;
        valid.sender_id = 2;
        valid.sequence_number = 10;
        valid.confirmed_sequence_number = 0;
        valid.timestamp = 5_000;
        valid.confirmed_timestamp = 1_000;
        inject_received_packet(&mut conn, &valid);
        conn.process().unwrap();
        assert_eq!(conn.take_timestamp_trace().unwrap().rejection, None);

        clock.set(11_001);
        let mut stale = valid;
        stale.sequence_number = 11;
        stale.timestamp = 5_000;
        stale.confirmed_timestamp = 1_000;
        inject_received_packet(&mut conn, &stale);

        assert!(matches!(
            conn.process(),
            Err(ConnectionError::SafetyTimeout)
        ));
        let trace = conn.take_timestamp_trace().unwrap();
        assert_eq!(
            trace.rejection,
            Some(TimestampTraceRejection::RemoteTimestampTooOld)
        );
        assert_eq!(conn.state_machine.current_state, RastaState::Closed);
    }

    #[test]
    fn peer_relative_rejects_backward_peer_timestamp() {
        let clock = FakeClock::new(1_000);
        let mut conn = RastaConnection::try_new(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            clock.clone(),
            peer_relative_config(1, 2),
        )
        .unwrap();
        conn.connect().unwrap();
        conn.sequence.accept_initial_rx(9);
        conn.transition(RastaState::Up).unwrap();
        conn.learn_peer_timestamp_offset_for_test(1_000, 5_000);

        let mut valid = packet(PacketType::Heartbeat, 0);
        valid.receiver_id = 1;
        valid.sender_id = 2;
        valid.sequence_number = 10;
        valid.confirmed_sequence_number = 0;
        valid.timestamp = 5_000;
        valid.confirmed_timestamp = 1_000;
        inject_received_packet(&mut conn, &valid);
        conn.process().unwrap();
        assert_eq!(conn.take_timestamp_trace().unwrap().rejection, None);

        clock.set(1_100);
        let mut backward = valid;
        backward.sequence_number = 11;
        backward.timestamp = 4_999;
        backward.confirmed_timestamp = 1_000;
        inject_received_packet(&mut conn, &backward);

        assert!(matches!(
            conn.process(),
            Err(ConnectionError::SafetyTimeout)
        ));
        let trace = conn.take_timestamp_trace().unwrap();
        assert_eq!(
            trace.rejection,
            Some(TimestampTraceRejection::RemoteTimestampMovedBackwards)
        );
        assert_eq!(conn.state_machine.current_state, RastaState::Closed);
    }

    #[test]
    fn peer_relative_rejects_invalid_confirmed_timestamp() {
        let clock = FakeClock::new(1_000);
        let mut conn = RastaConnection::try_new(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            clock.clone(),
            peer_relative_config(1, 2),
        )
        .unwrap();
        conn.connect().unwrap();
        conn.sequence.accept_initial_rx(9);
        conn.transition(RastaState::Up).unwrap();
        conn.learn_peer_timestamp_offset_for_test(1_000, 5_000);

        let mut valid = packet(PacketType::Heartbeat, 0);
        valid.receiver_id = 1;
        valid.sender_id = 2;
        valid.sequence_number = 10;
        valid.confirmed_sequence_number = 0;
        valid.timestamp = 5_000;
        valid.confirmed_timestamp = 1_000;
        inject_received_packet(&mut conn, &valid);
        conn.process().unwrap();
        assert_eq!(conn.take_timestamp_trace().unwrap().rejection, None);

        clock.set(1_100);
        let mut invalid = valid;
        invalid.sequence_number = 11;
        invalid.timestamp = 5_100;
        invalid.confirmed_timestamp = 999;
        inject_received_packet(&mut conn, &invalid);

        assert!(matches!(
            conn.process(),
            Err(ConnectionError::SafetyTimeout)
        ));
        let trace = conn.take_timestamp_trace().unwrap();
        assert_eq!(
            trace.rejection,
            Some(TimestampTraceRejection::ConfirmedTimestampMovedBackwards)
        );
        assert_eq!(
            conn.take_diagnostic().map(|event| event.kind),
            Some(DiagnosticKind::ConfirmedTimestampError)
        );
    }

    #[test]
    fn peer_relative_rejects_future_confirmed_timestamp() {
        let clock = FakeClock::new(1_000);
        let mut conn = RastaConnection::try_new(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            clock.clone(),
            peer_relative_config(1, 2),
        )
        .unwrap();
        conn.connect().unwrap();
        conn.sequence.accept_initial_rx(9);
        conn.transition(RastaState::Up).unwrap();
        conn.learn_peer_timestamp_offset_for_test(1_000, 5_000);

        let mut heartbeat = packet(PacketType::Heartbeat, 0);
        heartbeat.receiver_id = 1;
        heartbeat.sender_id = 2;
        heartbeat.sequence_number = 10;
        heartbeat.confirmed_sequence_number = 0;
        heartbeat.timestamp = 5_000;
        heartbeat.confirmed_timestamp = 1_001;
        inject_received_packet(&mut conn, &heartbeat);

        assert!(matches!(
            conn.process(),
            Err(ConnectionError::SafetyTimeout)
        ));
        let trace = conn.take_timestamp_trace().unwrap();
        assert_eq!(
            trace.rejection,
            Some(TimestampTraceRejection::ConfirmedTimestampTooFarInFuture)
        );
        assert_eq!(
            conn.take_diagnostic().map(|event| event.kind),
            Some(DiagnosticKind::ConfirmedTimestampError)
        );
    }

    #[test]
    fn shared_protocol_epoch_accepts_unequal_local_clock_origins() {
        let network = Rc::new(RefCell::new(TestNetwork::new()));
        let client_clock = SplitClock::new(0, 10_000);
        let server_clock = SplitClock::new(5_000, 10_000);
        let mut client = RastaConnection::try_new(
            LinkedTransport::new(network.clone(), 0),
            LinkedTransport::new(network.clone(), 1),
            client_clock.clone(),
            config(1, 2),
        )
        .unwrap();
        let mut server = RastaConnection::try_new(
            LinkedTransport::new(network.clone(), 2),
            LinkedTransport::new(network.clone(), 3),
            server_clock.clone(),
            config(2, 1),
        )
        .unwrap();

        client.connect().unwrap();
        server.connect().unwrap();
        server.process().unwrap();
        client.process().unwrap();
        server.process().unwrap();
        client.process().unwrap();
        assert_eq!(client.state_machine.current_state, RastaState::Up);
        assert_eq!(server.state_machine.current_state, RastaState::Up);
        clear_network_channels(&network, &[0, 1, 2, 3]);

        for _ in 0..5 {
            client_clock.advance(500);
            server_clock.advance(500);
            client.process().unwrap();
            server.process().unwrap();
            client.process().unwrap();
            assert_eq!(client.state_machine.current_state, RastaState::Up);
            assert_eq!(server.state_machine.current_state, RastaState::Up);
            clear_network_channels(&network, &[0, 1, 2, 3]);
        }

        while let Some(diagnostic) = client.take_diagnostic() {
            assert_ne!(diagnostic.kind, DiagnosticKind::ConnectionTimeout);
        }
        while let Some(diagnostic) = server.take_diagnostic() {
            assert_ne!(diagnostic.kind, DiagnosticKind::ConnectionTimeout);
        }
    }

    #[test]
    fn incoming_heartbeats_do_not_suppress_outgoing_heartbeat() {
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
            LinkedTransport::new(network.clone(), 3),
            SharedClock(time.clone()),
            config(2, 1),
        )
        .unwrap();

        client.connect().unwrap();
        server.connect().unwrap();
        server.process().unwrap();
        client.process().unwrap();
        server.process().unwrap();
        client.process().unwrap();
        clear_network_channels(&network, &[0, 1, 2, 3]);

        let mut server_heartbeats = 0usize;
        for (incoming_at, due_at) in [(499u32, 500u32), (999, 1_000), (1_499, 1_500)] {
            time.set(incoming_at);
            client.send_packet(PacketType::Heartbeat, &[]).unwrap();
            server.process().unwrap();
            assert_eq!(
                network_packet_type_count(
                    &network,
                    &[0, 1],
                    &server.safety_code,
                    PacketType::Heartbeat
                ),
                0
            );

            time.set(due_at);
            server.process().unwrap();
            server_heartbeats += network_packet_type_count(
                &network,
                &[0, 1],
                &server.safety_code,
                PacketType::Heartbeat,
            );
            assert_eq!(server.state_machine.current_state, RastaState::Up);
            clear_network_channels(&network, &[0, 1, 2, 3]);
        }

        assert_eq!(server_heartbeats, 6);
    }

    #[test]
    fn incoming_data_does_not_suppress_outgoing_heartbeat() {
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
            LinkedTransport::new(network.clone(), 3),
            SharedClock(time.clone()),
            config(2, 1),
        )
        .unwrap();

        client.connect().unwrap();
        server.connect().unwrap();
        server.process().unwrap();
        client.process().unwrap();
        server.process().unwrap();
        client.process().unwrap();
        clear_network_channels(&network, &[0, 1, 2, 3]);

        let mut server_heartbeats = 0usize;
        for (index, (incoming_at, due_at)) in [(499u32, 500u32), (999, 1_000), (1_499, 1_500)]
            .iter()
            .copied()
            .enumerate()
        {
            time.set(incoming_at);
            let payload = [b'a'.wrapping_add(index as u8)];
            client.send_application_data(&payload).unwrap();
            server.process().unwrap();
            assert!(server.has_received_data());
            let mut output = [0u8; 8];
            assert_eq!(server.receive_data(&mut output).unwrap(), 1);

            time.set(due_at);
            server.process().unwrap();
            server_heartbeats += network_packet_type_count(
                &network,
                &[0, 1],
                &server.safety_code,
                PacketType::Heartbeat,
            );
            assert_eq!(server.state_machine.current_state, RastaState::Up);
            clear_network_channels(&network, &[0, 1, 2, 3]);
        }

        assert_eq!(server_heartbeats, 6);
    }

    #[test]
    fn outgoing_data_restarts_send_heartbeat_interval() {
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
            LinkedTransport::new(network.clone(), 3),
            SharedClock(time.clone()),
            config(2, 1),
        )
        .unwrap();

        client.connect().unwrap();
        server.connect().unwrap();
        server.process().unwrap();
        client.process().unwrap();
        server.process().unwrap();
        client.process().unwrap();
        clear_network_channels(&network, &[0, 1, 2, 3]);

        time.set(400);
        client.send_application_data(b"x").unwrap();
        assert_eq!(
            network_packet_type_count(&network, &[2, 3], &client.safety_code, PacketType::Data),
            2
        );

        clear_network_channels(&network, &[2, 3]);
        time.set(500);
        client.process().unwrap();
        assert_eq!(
            network_packet_type_count(
                &network,
                &[2, 3],
                &client.safety_code,
                PacketType::Heartbeat
            ),
            0
        );

        time.set(900);
        client.process().unwrap();
        assert_eq!(
            network_packet_type_count(
                &network,
                &[2, 3],
                &client.safety_code,
                PacketType::Heartbeat
            ),
            2
        );
        assert_eq!(client.state_machine.current_state, RastaState::Up);
    }

    #[test]
    fn sequence_gap_retransmission_recovers_lost_data_in_order() {
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
            LinkedTransport::new(network.clone(), 3),
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
        client.process().unwrap();

        client.send_application_data(b"zero").unwrap();
        server.process().unwrap();
        let mut output = [0u8; 32];
        let len = server.receive_data(&mut output).unwrap();
        assert_eq!(&output[..len], b"zero");

        client.send_application_data(b"one").unwrap();
        {
            let mut network = network.borrow_mut();
            assert!(network.drop_next(2));
            assert!(network.drop_next(3));
        }
        let missing_sequence = server.sequence.expected_rx();

        client.send_application_data(b"two").unwrap();
        server.process().unwrap();
        assert!(!server.has_received_data());
        time.set(100);
        server.process().unwrap();
        assert_eq!(
            server.state_machine.current_state,
            RastaState::RetransmissionRequested
        );
        assert_eq!(server.sequence.expected_rx(), missing_sequence);
        assert!(!server.has_received_data());
        assert_eq!(server.error_counters().sequence_number, 1);

        let mut request_bytes = [0u8; 512];
        let request_len = network
            .borrow()
            .peek_payload(0, &mut request_bytes)
            .expect("retransmission request frame");
        let request = Packet::parse(&request_bytes[..request_len], &server.safety_code).unwrap();
        assert_eq!(request.packet_type, PacketType::RetransmissionRequest);
        assert_eq!(request.payload_len, 0);
        assert_eq!(
            request.confirmed_sequence_number,
            missing_sequence.wrapping_sub(1)
        );

        client.process().unwrap();
        assert_eq!(client.state_machine.current_state, RastaState::Up);
        assert!(client.retransmission.count() >= 2);

        server.process().unwrap();
        assert_eq!(server.state_machine.current_state, RastaState::Up);
        assert_eq!(
            server.sequence.expected_rx(),
            client.sequence.next_tx_value()
        );

        let len = server.receive_data(&mut output).unwrap();
        assert_eq!(&output[..len], b"one");
        let len = server.receive_data(&mut output).unwrap();
        assert_eq!(&output[..len], b"two");
        assert!(!server.has_received_data());
        assert_eq!(client.retransmission.count(), 3);
    }

    #[test]
    fn peer_silence_times_out_at_exact_t_max_and_sends_disconnect_once() {
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
            LinkedTransport::new(network.clone(), 3),
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
        clear_network_channels(&network, &[0, 1, 2, 3]);

        time.set(1_999);
        assert!(client.process().is_ok());
        assert_eq!(client.state_machine.current_state, RastaState::Up);
        {
            let mut network = network.borrow_mut();
            while network.drop_next(2) {}
            while network.drop_next(3) {}
        }

        time.set(2_000);
        assert!(matches!(
            client.process(),
            Err(ConnectionError::SafetyTimeout)
        ));
        assert_eq!(client.state_machine.current_state, RastaState::Closed);
        let mut saw_timeout = false;
        while let Some(diagnostic) = client.take_diagnostic() {
            if diagnostic
                == (DiagnosticEvent {
                    kind: DiagnosticKind::ConnectionTimeout,
                    value: 2_000,
                })
            {
                saw_timeout = true;
                break;
            }
        }
        assert!(saw_timeout);

        let mut disconnect_bytes = [0u8; 512];
        let disconnect_len = network
            .borrow()
            .peek_payload(2, &mut disconnect_bytes)
            .expect("timeout disconnection request");
        let disconnect = Packet::parse(&disconnect_bytes[..disconnect_len], &client.safety_code)
            .expect("parse disconnect");
        assert_eq!(disconnect.packet_type, PacketType::DisconnectionRequest);
        assert_eq!(
            u16::from_le_bytes([disconnect.payload[2], disconnect.payload[3]]),
            DisconnectReason::IncomingMessageTimeout.code()
        );

        let count_after_timeout = network.borrow().count[2] + network.borrow().count[3];
        assert!(client.process().is_ok());
        assert_eq!(
            network.borrow().count[2] + network.borrow().count[3],
            count_after_timeout
        );
    }

    #[test]
    fn queued_heartbeat_at_deadline_prevents_false_timeout() {
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
            LinkedTransport::new(network.clone(), 3),
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
        client.process().unwrap();

        time.set(1_999);
        client.send_packet(PacketType::Heartbeat, &[]).unwrap();
        server.process().unwrap();

        time.set(2_000);
        let expected_rx = client.sequence.expected_rx();
        assert!(client.process().is_ok());

        assert_eq!(client.state_machine.current_state, RastaState::Up);
        assert_eq!(client.sequence.expected_rx(), expected_rx.wrapping_add(1));
        assert_eq!(
            client
                .timeliness_deadline_for_test()
                .map(|deadline| deadline.wrapping_millis()),
            Some(4_000)
        );
        while let Some(diagnostic) = client.take_diagnostic() {
            assert_ne!(diagnostic.kind, DiagnosticKind::ConnectionTimeout);
        }
        assert!(!network_has_packet_type(
            &network,
            &[2, 3],
            &client.safety_code,
            PacketType::DisconnectionRequest
        ));
    }

    #[test]
    fn first_peer_heartbeat_refreshes_active_receive_deadline() {
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
            LinkedTransport::new(network.clone(), 3),
            SharedClock(time.clone()),
            config(2, 1),
        )
        .unwrap();

        client.connect().unwrap();
        server.connect().unwrap();
        server.process().unwrap();
        client.process().unwrap();
        assert_eq!(client.state_machine.current_state, RastaState::Up);
        let deadline_before = client.timeliness_deadline_for_test().unwrap();

        time.set(1_999);
        server.process().unwrap();
        assert_eq!(
            network_head_packet_type(&network, 0, &server.safety_code),
            Some(PacketType::Heartbeat)
        );

        let state_before = client.state_machine.current_state;
        client.process().unwrap();
        let state_after = client.state_machine.current_state;
        let deadline_after = client.timeliness_deadline_for_test().unwrap();

        assert_eq!(state_before, RastaState::Up);
        assert_eq!(state_after, RastaState::Up);
        assert!(instant_is_later(deadline_after, deadline_before));
        assert_eq!(deadline_after.wrapping_millis(), 3_999);
        while let Some(diagnostic) = client.take_diagnostic() {
            assert_ne!(diagnostic.kind, DiagnosticKind::ConnectionTimeout);
        }
        assert!(!network_has_packet_type(
            &network,
            &[2, 3],
            &client.safety_code,
            PacketType::DisconnectionRequest
        ));
    }

    #[test]
    fn first_peer_heartbeat_at_exact_deadline_refreshes_before_timeout() {
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
            LinkedTransport::new(network.clone(), 3),
            SharedClock(time.clone()),
            config(2, 1),
        )
        .unwrap();

        client.connect().unwrap();
        server.connect().unwrap();
        server.process().unwrap();
        client.process().unwrap();
        let deadline_before = client.timeliness_deadline_for_test().unwrap();
        assert_eq!(deadline_before.wrapping_millis(), 2_000);

        time.set(2_000);
        server.process().unwrap();
        let state_before = client.state_machine.current_state;
        client.process().unwrap();
        let deadline_after = client.timeliness_deadline_for_test().unwrap();

        assert_eq!(state_before, RastaState::Up);
        assert_eq!(client.state_machine.current_state, RastaState::Up);
        assert!(instant_is_later(deadline_after, deadline_before));
        assert_eq!(deadline_after.wrapping_millis(), 4_000);
        while let Some(diagnostic) = client.take_diagnostic() {
            assert_ne!(diagnostic.kind, DiagnosticKind::ConnectionTimeout);
        }
    }

    #[test]
    fn refreshed_deadline_is_not_overwritten_later_in_same_poll() {
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
            LinkedTransport::new(network.clone(), 3),
            SharedClock(time.clone()),
            config(2, 1),
        )
        .unwrap();

        client.connect().unwrap();
        server.connect().unwrap();
        server.process().unwrap();
        client.process().unwrap();
        let deadline_before = client.timeliness_deadline_for_test().unwrap();

        time.set(1_250);
        server.process().unwrap();
        client.process().unwrap();
        let deadline_after_packet = client.timeliness_deadline_for_test().unwrap();
        assert_eq!(deadline_after_packet.wrapping_millis(), 3_250);
        assert!(instant_is_later(deadline_after_packet, deadline_before));

        client.process().unwrap();
        assert_eq!(
            client.timeliness_deadline_for_test(),
            Some(deadline_after_packet)
        );
    }

    #[test]
    fn duplicate_redundancy_heartbeat_refreshes_once_without_shortening_deadline() {
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
            LinkedTransport::new(network.clone(), 3),
            SharedClock(time.clone()),
            config(2, 1),
        )
        .unwrap();

        client.connect().unwrap();
        server.connect().unwrap();
        server.process().unwrap();
        client.process().unwrap();
        client.process().unwrap();
        clear_network_channels(&network, &[0, 1, 2, 3]);
        let deadline_before = client.timeliness_deadline_for_test().unwrap();

        time.set(1_000);
        server.send_packet(PacketType::Heartbeat, &[]).unwrap();
        assert_eq!(
            network_packet_type_count(
                &network,
                &[0, 1],
                &server.safety_code,
                PacketType::Heartbeat
            ),
            2
        );
        client.process().unwrap();
        let deadline_after = client.timeliness_deadline_for_test().unwrap();

        assert_eq!(client.state_machine.current_state, RastaState::Up);
        assert_eq!(deadline_after.wrapping_millis(), 3_000);
        assert!(instant_is_later(deadline_after, deadline_before));
        while let Some(diagnostic) = client.take_diagnostic() {
            assert_ne!(diagnostic.kind, DiagnosticKind::ConnectionTimeout);
        }
    }

    #[test]
    fn rejected_heartbeat_does_not_refresh_receive_deadline() {
        let clock = FakeClock::new(0);
        let mut conn = RastaConnection::try_new(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            clock.clone(),
            config(1, 2),
        )
        .unwrap();
        conn.connect().unwrap();
        conn.sequence.accept_initial_rx(9);
        conn.transition(RastaState::Up).unwrap();
        let deadline_before = conn.timeliness_deadline_for_test().unwrap();

        let mut heartbeat = packet(PacketType::Heartbeat, 0);
        heartbeat.receiver_id = 1;
        heartbeat.sender_id = 2;
        heartbeat.sequence_number = 10;
        heartbeat.confirmed_sequence_number = 0;
        heartbeat.timestamp = 1_000;
        heartbeat.confirmed_timestamp = 1_000;
        let mut srl = [0u8; 512];
        let length = heartbeat
            .serialize(&mut srl, &conn.safety_code)
            .expect("serialize heartbeat");
        srl[length - 1] ^= 0xff;
        let total =
            length + RedundancyLayer::<SimpleMockTransport, SimpleMockTransport>::HEADER_SIZE;
        let mut rl = [0u8; 520];
        rl[..2].copy_from_slice(&(total as u16).to_le_bytes());
        rl[4..8].copy_from_slice(&0u32.to_le_bytes());
        rl[8..total].copy_from_slice(&srl[..length]);
        conn.redundancy = RedundancyLayer::with_config(
            SimpleMockTransport::with_receive(&rl[..total]),
            SimpleMockTransport::empty(),
            RedundancyConfig {
                check_code: RedundancyCheckCode::None,
                t_seq_ms: 100,
            },
        );

        clock.set(1_000);
        conn.process().unwrap();
        assert_eq!(conn.timeliness_deadline_for_test(), Some(deadline_before));
        assert_eq!(
            conn.take_diagnostic(),
            Some(DiagnosticEvent {
                kind: DiagnosticKind::SafetyCodeError,
                value: 1,
            })
        );
    }

    #[test]
    fn invalid_queued_packet_at_deadline_does_not_prevent_timeout() {
        let clock = FakeClock::new(0);
        let mut conn = RastaConnection::try_new(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            clock.clone(),
            config(1, 2),
        )
        .unwrap();
        conn.connect().unwrap();
        conn.sequence.accept_initial_rx(9);
        conn.transition(RastaState::Up).unwrap();

        let mut heartbeat = packet(PacketType::Heartbeat, 0);
        heartbeat.receiver_id = 1;
        heartbeat.sender_id = 2;
        heartbeat.sequence_number = 10;
        heartbeat.confirmed_sequence_number = 0;
        heartbeat.timestamp = 2_000;
        heartbeat.confirmed_timestamp = 2_000;
        let mut srl = [0u8; 512];
        let length = heartbeat
            .serialize(&mut srl, &conn.safety_code)
            .expect("serialize heartbeat");
        srl[length - 1] ^= 0xff;
        let total =
            length + RedundancyLayer::<SimpleMockTransport, SimpleMockTransport>::HEADER_SIZE;
        let mut rl = [0u8; 520];
        rl[..2].copy_from_slice(&(total as u16).to_le_bytes());
        rl[4..8].copy_from_slice(&0u32.to_le_bytes());
        rl[8..total].copy_from_slice(&srl[..length]);
        conn.redundancy = RedundancyLayer::with_config(
            SimpleMockTransport::with_receive(&rl[..total]),
            SimpleMockTransport::empty(),
            RedundancyConfig {
                check_code: RedundancyCheckCode::None,
                t_seq_ms: 100,
            },
        );

        clock.set(2_000);
        assert!(matches!(
            conn.process(),
            Err(ConnectionError::SafetyTimeout)
        ));
        assert_eq!(conn.state_machine.current_state, RastaState::Closed);
        assert_eq!(conn.sequence.expected_rx(), 10);
        assert_eq!(
            conn.take_diagnostic(),
            Some(DiagnosticEvent {
                kind: DiagnosticKind::SafetyCodeError,
                value: 1,
            })
        );
        assert_eq!(
            conn.take_diagnostic(),
            Some(DiagnosticEvent {
                kind: DiagnosticKind::ConnectionTimeout,
                value: 2_000,
            })
        );
    }

    #[test]
    fn one_valid_redundancy_channel_at_deadline_prevents_connection_timeout() {
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
            LinkedTransport::new(network.clone(), 3),
            SharedClock(time.clone()),
            config(2, 1),
        )
        .unwrap();

        client.connect().unwrap();
        server.connect().unwrap();
        server.process().unwrap();
        client.process().unwrap();
        server.process().unwrap();
        assert_eq!(
            client.channel_statuses(),
            [ChannelStatus::Healthy, ChannelStatus::Healthy]
        );
        while client.take_diagnostic().is_some() {}
        client.process().unwrap();

        time.set(1_999);
        client.send_packet(PacketType::Heartbeat, &[]).unwrap();
        server.process().unwrap();

        time.set(2_000);
        assert!(network.borrow_mut().drop_next(1));
        assert!(client.process().is_ok());

        assert_eq!(client.state_machine.current_state, RastaState::Up);
        assert_eq!(client.channel_statuses()[0], ChannelStatus::Healthy);
        let mut channel_failure_count = 0;
        while let Some(diagnostic) = client.take_diagnostic() {
            assert_ne!(diagnostic.kind, DiagnosticKind::ConnectionTimeout);
            if diagnostic.kind == DiagnosticKind::ChannelSupervisionFailure {
                channel_failure_count += 1;
                assert_eq!(diagnostic.value, 1);
            }
        }
        assert_eq!(channel_failure_count, 1);
    }

    #[test]
    fn valid_peer_heartbeat_restarts_deadline_but_sent_heartbeat_alone_does_not() {
        let clock = FakeClock::new(0);
        let mut conn = RastaConnection::try_new(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            clock.clone(),
            config(1, 2),
        )
        .unwrap();
        conn.connect().unwrap();
        conn.sequence.accept_initial_rx(9);
        conn.transition(RastaState::Up).unwrap();

        let mut heartbeat = packet(PacketType::Heartbeat, 0);
        heartbeat.receiver_id = 1;
        heartbeat.sender_id = 2;
        heartbeat.sequence_number = 10;
        heartbeat.confirmed_sequence_number = 0;
        heartbeat.timestamp = 1_500;
        heartbeat.confirmed_timestamp = 1_499;
        clock.set(1_500);
        conn.redundancy = RedundancyLayer::with_config(
            receive_packet_transport(&heartbeat, &conn.safety_code),
            SimpleMockTransport::empty(),
            RedundancyConfig {
                check_code: RedundancyCheckCode::None,
                t_seq_ms: 100,
            },
        );

        conn.process().unwrap();
        assert_eq!(conn.state_machine.current_state, RastaState::Up);

        clock.set(3_499);
        assert!(conn.process().is_ok());
        assert_eq!(conn.state_machine.current_state, RastaState::Up);

        clock.set(3_500);
        assert!(matches!(
            conn.process(),
            Err(ConnectionError::SafetyTimeout)
        ));
        assert_eq!(conn.state_machine.current_state, RastaState::Closed);
    }

    #[test]
    fn invalid_remote_timestamp_rejects_packet_before_sequence_or_deadline_refresh() {
        let clock = FakeClock::new(3_000);
        let mut conn = RastaConnection::try_new(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            clock,
            config(1, 2),
        )
        .unwrap();
        conn.transition(RastaState::Down).unwrap();
        conn.transition(RastaState::Start).unwrap();
        conn.sequence.accept_initial_rx(9);
        conn.transition(RastaState::Up).unwrap();

        let mut invalid = packet(PacketType::Data, 7);
        invalid.receiver_id = 1;
        invalid.sender_id = 2;
        invalid.sequence_number = 10;
        invalid.timestamp = 999;
        invalid.confirmed_timestamp = 3_000;
        invalid.payload[..2].copy_from_slice(&5u16.to_le_bytes());
        invalid.payload[2..7].copy_from_slice(b"stale");
        conn.redundancy = RedundancyLayer::with_config(
            receive_packet_transport(&invalid, &conn.safety_code),
            SimpleMockTransport::empty(),
            RedundancyConfig {
                check_code: RedundancyCheckCode::None,
                t_seq_ms: 100,
            },
        );

        assert!(matches!(
            conn.process(),
            Err(ConnectionError::SafetyTimeout)
        ));
        assert_eq!(conn.state_machine.current_state, RastaState::Closed);
        assert_eq!(conn.sequence.expected_rx(), 10);
        assert!(!conn.has_received_data());
        assert_eq!(
            conn.take_diagnostic().map(|event| event.kind),
            Some(DiagnosticKind::ConnectionTimeout)
        );
    }

    #[test]
    fn invalid_confirmed_timestamp_rejects_packet_before_sequence_or_deadline_refresh() {
        let clock = FakeClock::new(1_000);
        let mut conn = RastaConnection::try_new(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            clock.clone(),
            config(1, 2),
        )
        .unwrap();
        conn.transition(RastaState::Down).unwrap();
        conn.transition(RastaState::Start).unwrap();
        conn.sequence.accept_initial_rx(9);
        conn.transition(RastaState::Up).unwrap();

        let mut valid = packet(PacketType::Heartbeat, 0);
        valid.receiver_id = 1;
        valid.sender_id = 2;
        valid.sequence_number = 10;
        valid.confirmed_sequence_number = u32::MAX;
        valid.timestamp = 1_000;
        valid.confirmed_timestamp = 1_000;
        conn.redundancy = RedundancyLayer::with_config(
            receive_packet_transport(&valid, &conn.safety_code),
            SimpleMockTransport::empty(),
            RedundancyConfig {
                check_code: RedundancyCheckCode::None,
                t_seq_ms: 100,
            },
        );
        conn.process().unwrap();
        assert_eq!(conn.sequence.expected_rx(), 11);

        clock.set(1_001);
        let mut invalid = packet(PacketType::Heartbeat, 0);
        invalid.receiver_id = 1;
        invalid.sender_id = 2;
        invalid.sequence_number = 11;
        invalid.confirmed_sequence_number = u32::MAX;
        invalid.timestamp = 1_001;
        invalid.confirmed_timestamp = 999;
        conn.redundancy = RedundancyLayer::with_config(
            receive_packet_transport(&invalid, &conn.safety_code),
            SimpleMockTransport::empty(),
            RedundancyConfig {
                check_code: RedundancyCheckCode::None,
                t_seq_ms: 100,
            },
        );

        assert!(matches!(
            conn.process(),
            Err(ConnectionError::SafetyTimeout)
        ));
        assert_eq!(conn.state_machine.current_state, RastaState::Closed);
        assert_eq!(conn.sequence.expected_rx(), 11);
        assert_eq!(
            conn.take_diagnostic().map(|event| event.kind),
            Some(DiagnosticKind::ConfirmedTimestampError)
        );
    }

    #[test]
    fn wraparound_timestamps_and_deadlines_remain_valid() {
        let clock = FakeClock::new(u32::MAX - 10);
        let mut conn = RastaConnection::try_new(
            SimpleMockTransport::empty(),
            SimpleMockTransport::empty(),
            clock.clone(),
            config(1, 2),
        )
        .unwrap();
        conn.connect().unwrap();
        conn.sequence.accept_initial_rx(9);
        conn.transition(RastaState::Up).unwrap();

        let mut valid = packet(PacketType::Heartbeat, 0);
        valid.receiver_id = 1;
        valid.sender_id = 2;
        valid.sequence_number = 10;
        valid.confirmed_sequence_number = 0;
        valid.timestamp = u32::MAX - 9;
        valid.confirmed_timestamp = u32::MAX - 10;
        clock.set(u32::MAX - 9);
        conn.redundancy = RedundancyLayer::with_config(
            receive_packet_transport(&valid, &conn.safety_code),
            SimpleMockTransport::empty(),
            RedundancyConfig {
                check_code: RedundancyCheckCode::None,
                t_seq_ms: 100,
            },
        );
        conn.process().unwrap();
        assert_eq!(conn.state_machine.current_state, RastaState::Up);

        clock.set(1_989);
        assert!(conn.process().is_ok());
        clock.set(1_990);
        assert!(matches!(
            conn.process(),
            Err(ConnectionError::SafetyTimeout)
        ));
        assert_eq!(conn.state_machine.current_state, RastaState::Closed);
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
    fn diagnostics_queue_overflow_is_counted_without_unrelated_counter_changes() {
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

        let bad_packet = packet(PacketType::Heartbeat, 0);
        let mut srl = [0u8; 512];
        let length = bad_packet
            .serialize(&mut srl, &SafetyCodeConfig::default())
            .unwrap();
        srl[length - 1] ^= 0xff;
        let total =
            length + RedundancyLayer::<SimpleMockTransport, SimpleMockTransport>::HEADER_SIZE;
        let mut rl = [0u8; 520];
        rl[..2].copy_from_slice(&(total as u16).to_le_bytes());
        rl[4..8].copy_from_slice(&0u32.to_le_bytes());
        rl[8..total].copy_from_slice(&srl[..length]);

        for _ in 0..20 {
            connection.redundancy = RedundancyLayer::with_config(
                SimpleMockTransport::with_receive(&rl[..total]),
                SimpleMockTransport::empty(),
                RedundancyConfig {
                    check_code: RedundancyCheckCode::None,
                    t_seq_ms: 100,
                },
            );
            assert!(connection.process().is_ok());
        }

        assert_eq!(connection.error_counters().safety, 20);
        assert_eq!(connection.error_counters().message_type, 0);
        assert!(connection.diagnostic_overflow_count() > 0);
        let mut taken = 0;
        while connection.take_diagnostic().is_some() {
            taken += 1;
        }
        assert_eq!(taken, 16);
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
    fn connection_configuration_rejects_each_invalid_rule_independently() {
        let base = config(1, 2);
        let cases: [fn(&mut RastaConfig); 12] = [
            |c| c.sender_id = 0,
            |c| c.remote_id = 0,
            |c| c.remote_id = c.sender_id,
            |c| c.t_max = 0,
            |c| c.t_max = 0x8000_0000,
            |c| c.heartbeat_interval_ms = 0,
            |c| c.heartbeat_interval_ms = 0x8000_0000,
            |c| c.n_send_max = 0,
            |c| c.n_send_max = 21,
            |c| c.mwa = 0,
            |c| c.mwa = c.n_send_max,
            |c| c.redundancy.t_seq_ms = 0,
        ];

        for mutate in cases {
            let mut invalid = base;
            mutate(&mut invalid);
            assert!(matches!(
                RastaConnection::try_new(
                    SimpleMockTransport::empty(),
                    SimpleMockTransport::empty(),
                    MockClock { time: 0 },
                    invalid,
                ),
                Err(ConnectionError::InvalidConfiguration)
            ));
        }

        let mut invalid = base;
        invalid.safety_code = SafetyCodeConfig {
            mode: SafetyCodeMode::None,
            md4_initial_value: SafetyCodeConfig::STANDARD_MD4_INITIAL_VALUE,
        };
        assert!(matches!(
            RastaConnection::try_new(
                SimpleMockTransport::empty(),
                SimpleMockTransport::empty(),
                MockClock { time: 0 },
                invalid,
            ),
            Err(ConnectionError::InvalidConfiguration)
        ));

        let mut invalid = base;
        invalid.redundancy.check_code = RedundancyCheckCode::OptionA;
        assert!(matches!(
            RastaConnection::try_new(
                SimpleMockTransport::empty(),
                SimpleMockTransport::empty(),
                MockClock { time: 0 },
                invalid,
            ),
            Err(ConnectionError::InvalidConfiguration)
        ));

        let mut compatible = base;
        compatible.safety_code = SafetyCodeConfig::none();
        compatible.redundancy.check_code = RedundancyCheckCode::OptionA;
        compatible.allow_unsafe_no_checksums = true;
        assert!(
            RastaConnection::try_new(
                SimpleMockTransport::empty(),
                SimpleMockTransport::empty(),
                MockClock { time: 0 },
                compatible,
            )
            .is_ok()
        );
    }

    #[test]
    fn interoperability_profile_validation_reports_each_typed_error() {
        let base = valid_profile();
        type ProfileMutation = fn(&mut InteroperabilityProfile);
        let cases: [(ProfileMutation, ProfileError); 11] = [
            (
                |p| p.protocol_version = *b"0301",
                ProfileError::UnsupportedProtocolVersion,
            ),
            (|p| p.channel_count = 1, ProfileError::InvalidChannelCount),
            (|p| p.mwa = 0, ProfileError::InvalidFlowControl),
            (|p| p.mwa = p.n_send_max, ProfileError::InvalidFlowControl),
            (
                |p| p.retransmission_capacity = p.n_send_max - 1,
                ProfileError::InvalidCapacity,
            ),
            (
                |p| p.defer_queue_capacity = 3,
                ProfileError::InvalidCapacity,
            ),
            (|p| p.t_h_ms = 0, ProfileError::InvalidTiming),
            (|p| p.t_max_ms = p.t_h_ms, ProfileError::InvalidTiming),
            (
                |p| p.max_messages_per_packet = 2,
                ProfileError::InvalidPacketization,
            ),
            (
                |p| p.network_identifier = 0,
                ProfileError::InvalidNetworkIdentifier,
            ),
            (
                |p| p.md4_initial_value = [0; 16],
                ProfileError::UnsafeMd4InitialValue,
            ),
        ];

        for (mutate, expected) in cases {
            let mut invalid = base;
            mutate(&mut invalid);
            assert_eq!(invalid.validate(), Err(expected));
        }

        let mut invalid = base;
        invalid.md4_initial_value = InteroperabilityProfile::RFC_MD4_INITIAL_VALUE;
        assert_eq!(invalid.validate(), Err(ProfileError::UnsafeMd4InitialValue));
    }

    #[test]
    fn predefined_academic_profile_is_valid() {
        let profile = RastaProfile::academic_default().unwrap();

        assert_eq!(profile, RastaProfile::ACADEMIC_DEFAULT);
        assert_eq!(profile.network_identifier, 0x0000_0001);
        assert_eq!(profile.safety_code_length, SafetyCodeLength::Md4Lower8);
        assert_eq!(profile.redundancy_crc, RedundancyCrc::OptionB);
        assert_eq!(profile.t_max_ms, 1_800);
        assert_eq!(profile.t_h_ms, 300);
        assert_eq!(profile.t_seq_ms, 100);
        assert_eq!(
            profile.timestamp_compatibility,
            TimestampCompatibilityMode::StrictSynchronized
        );
    }

    #[test]
    fn predefined_librasta_local_profile_is_valid_with_explicit_unsafe_opt_in() {
        let profile = RastaProfile::librasta_local().unwrap();

        assert_eq!(profile, RastaProfile::LIBRASTA_LOCAL);
        assert_eq!(profile.network_identifier, 1234);
        assert_eq!(profile.safety_code_length, SafetyCodeLength::None);
        assert_eq!(profile.redundancy_crc, RedundancyCrc::OptionA);
        assert_eq!(profile.t_max_ms, 10_000);
        assert_eq!(profile.t_h_ms, 2_000);
        assert_eq!(profile.t_seq_ms, 50);
        assert_eq!(
            profile.timestamp_compatibility,
            TimestampCompatibilityMode::PeerRelative
        );
    }

    #[test]
    fn custom_profile_builder_can_create_valid_profile_values() {
        let profile = RastaProfileBuilder::new()
            .network_identifier(0x55aa)
            .timing(2_500, 500, 125)
            .flow_control(12, 6)
            .timestamp_compatibility(TimestampCompatibilityMode::StrictSynchronized)
            .build()
            .unwrap();

        assert_eq!(profile.network_identifier, 0x55aa);
        assert_eq!(profile.t_max_ms, 2_500);
        assert_eq!(profile.t_h_ms, 500);
        assert_eq!(profile.t_seq_ms, 125);
        assert_eq!(profile.n_send_max, 12);
        assert_eq!(profile.mwa, 6);
        assert_eq!(profile.retransmission_capacity, 12);
    }

    #[test]
    fn custom_profile_builder_rejects_invalid_timing() {
        assert_eq!(
            RastaProfileBuilder::new().timing(300, 300, 100).build(),
            Err(ConfigError::InvalidTiming)
        );
        assert_eq!(
            RastaProfileBuilder::new().timing(1_800, 0, 100).build(),
            Err(ConfigError::InvalidTiming)
        );
    }

    #[test]
    fn no_checksum_profile_requires_explicit_unsafe_opt_in() {
        let builder = RastaProfileBuilder::new()
            .safety_code_length(SafetyCodeLength::None)
            .redundancy_crc(RedundancyCrc::OptionA);

        assert_eq!(
            builder.build(),
            Err(ConfigError::UnsafeNoChecksumRequiresOptIn)
        );
        assert!(builder.allow_unsafe_no_checksums(true).build().is_ok());
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
            confirmed_sequence_number: u32::MAX,
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
            timestamp_compatibility: TimestampCompatibilityMode::StrictSynchronized,
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

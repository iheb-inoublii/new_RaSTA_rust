#[cfg(test)]
mod tests {
    use crate::core::connection::{RastaConfig, RastaConnection};
    use crate::core::packet::{Packet, PacketType};
    use crate::core::retransmission::RetransmissionBuffer;
    use crate::core::sequence::{SequenceHandler, SequenceResult};
    use crate::core::state_machine::{RastaState, StateMachine};
    use crate::platform::clock::Clock;
    use crate::platform::timer::Timer;
    use crate::platform::transport::{Transport, TransportError};

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

    struct SimpleMockTransport;
    impl Transport for SimpleMockTransport {
        fn send(&mut self, _data: &[u8]) -> Result<(), TransportError> {
            Ok(())
        }
        fn receive(&mut self, _buffer: &mut [u8]) -> Result<usize, TransportError> {
            Ok(0)
        }
    }

    #[test]
    fn test_packet_serialization() {
        let key = [0u8; 16];
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
            .serialize(&mut buffer, &key)
            .expect("Serialization failed");

        let parsed = Packet::parse(&buffer[..size], &key).expect("Parsing failed");
        assert_eq!(parsed.receiver_id, 1);
        assert_eq!(parsed.sender_id, 2);
        assert_eq!(parsed.sequence_number, 10);
        assert_eq!(parsed.payload_len, 4);
        assert_eq!(parsed.payload[0], 0xAA);
    }

    #[test]
    fn test_state_machine_transitions() {
        let mut sm = StateMachine::new();
        assert_eq!(sm.current_state, RastaState::Down);

        // Valid transition
        assert!(sm.transition(RastaState::Start));
        assert_eq!(sm.current_state, RastaState::Start);

        // Invalid transition: Down -> Up (must go through Start)
        let mut sm2 = StateMachine::new();
        assert!(!sm2.transition(RastaState::Up));
        assert_eq!(sm2.current_state, RastaState::Down);
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
        let config = RastaConfig {
            sender_id: 123,
            remote_id: 0,
            security_key: [0; 16],
            t_max: 2000,
            initial_seq: 0,
        };
        let mut conn = RastaConnection::new(
            SimpleMockTransport,
            SimpleMockTransport,
            timer,
            clock,
            config,
        );

        assert_eq!(conn.state_machine.current_state, RastaState::Down);
        conn.connect().expect("Connect failed");
        assert_eq!(conn.state_machine.current_state, RastaState::Start);
    }
}

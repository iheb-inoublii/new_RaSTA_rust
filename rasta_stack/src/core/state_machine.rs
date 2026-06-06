// RaSTA State Machine states (EN 50159)

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RastaState {
    Down,           // Initial state, connection closed
    Start,          // Connection request sent/received, waiting for handshake
    Up,             // Connection established, data can be sent
    Retransmission, // Connection in retransmission phase
    Closed,         // Disconnection initiated
}

pub struct StateMachine {
    pub current_state: RastaState,
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl StateMachine {
    pub fn new() -> Self {
        StateMachine {
            current_state: RastaState::Down,
        }
    }

    pub fn transition(&mut self, new_state: RastaState) -> bool {
        let allowed = match (self.current_state, new_state) {
            (RastaState::Down, RastaState::Start) => true,
            (RastaState::Start, RastaState::Up) => true,
            (RastaState::Start, RastaState::Down) => true,
            (RastaState::Up, RastaState::Retransmission) => true,
            (RastaState::Up, RastaState::Down) => true,
            (RastaState::Up, RastaState::Closed) => true,
            (RastaState::Retransmission, RastaState::Up) => true,
            (RastaState::Retransmission, RastaState::Down) => true,
            (RastaState::Closed, RastaState::Down) => true,
            // Self-transitions are generally allowed or ignored
            (s1, s2) if s1 == s2 => true,
            _ => false,
        };

        if allowed {
            self.current_state = new_state;
        }
        allowed
    }
}

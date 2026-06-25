// Connection state machine for the RaSTA service.

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RastaState {
    Closed,
    Down,
    Start,
    Up,
    RetransmissionRequested,
    RetransmissionRunning,
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
            current_state: RastaState::Closed,
        }
    }

    pub fn transition(&mut self, new_state: RastaState) -> bool {
        let allowed = match (self.current_state, new_state) {
            (RastaState::Closed, RastaState::Down) => true,
            (RastaState::Down, RastaState::Start) => true,
            (RastaState::Down, RastaState::Closed) => true,
            (RastaState::Start, RastaState::Up) => true,
            (RastaState::Start, RastaState::Closed) => true,
            (RastaState::Up, RastaState::RetransmissionRequested) => true,
            (RastaState::Up, RastaState::Closed) => true,
            (RastaState::RetransmissionRequested, RastaState::RetransmissionRunning) => true,
            (RastaState::RetransmissionRequested, RastaState::Closed) => true,
            (RastaState::RetransmissionRunning, RastaState::RetransmissionRequested) => true,
            (RastaState::RetransmissionRunning, RastaState::Up) => true,
            (RastaState::RetransmissionRunning, RastaState::Closed) => true,
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

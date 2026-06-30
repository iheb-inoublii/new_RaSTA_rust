use crate::connection::state_machine::RastaState;
use crate::connection::{ConnectionError, RastaConfig, RastaConnection, TimestampTraceEvent};
use crate::port::{RandomSource, Transport};
use crate::srl::DiagnosticEvent;
use crate::time::{MonotonicClock, ProtocolTimestampSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Down,
    Opening,
    Up,
    Retransmission,
    Closing,
}

impl From<RastaState> for ConnectionStatus {
    fn from(state: RastaState) -> Self {
        match state {
            RastaState::Closed => Self::Down,
            RastaState::Down => Self::Opening,
            RastaState::Start => Self::Opening,
            RastaState::Up => Self::Up,
            RastaState::RetransmissionRequested | RastaState::RetransmissionRunning => {
                Self::Retransmission
            }
        }
    }
}

pub struct RastaService<T1: Transport, T2: Transport, C: MonotonicClock + ProtocolTimestampSource> {
    connection: RastaConnection<T1, T2, C>,
}

pub type RastaApi<T1, T2, C> = RastaService<T1, T2, C>;

impl<T1: Transport, T2: Transport, C: MonotonicClock + ProtocolTimestampSource>
    RastaService<T1, T2, C>
{
    pub fn new(
        transport_a: T1,
        transport_b: T2,
        clock: C,
        config: RastaConfig,
    ) -> Result<Self, ConnectionError> {
        Ok(Self {
            connection: RastaConnection::try_new(transport_a, transport_b, clock, config)?,
        })
    }

    pub fn new_with_random<R: RandomSource>(
        transport_a: T1,
        transport_b: T2,
        clock: C,
        config: RastaConfig,
        random: &mut R,
    ) -> Result<Self, ConnectionError> {
        Ok(Self {
            connection: RastaConnection::try_new_with_random(
                transport_a,
                transport_b,
                clock,
                config,
                random,
            )?,
        })
    }

    pub fn open_connection(&mut self) -> Result<(), ConnectionError> {
        self.connection.connect()
    }

    pub fn close_connection(&mut self) -> Result<(), ConnectionError> {
        self.connection.disconnect()
    }

    pub fn send_data(&mut self, data: &[u8]) -> Result<(), ConnectionError> {
        if self.connection.state_machine.current_state != RastaState::Up {
            return Err(ConnectionError::StateTransitionInvalid);
        }
        self.connection.send_application_data(data)
    }

    pub fn poll(&mut self) -> Result<(), ConnectionError> {
        self.connection.process()
    }

    pub fn receive_data(&mut self, output: &mut [u8]) -> Result<usize, ConnectionError> {
        self.connection.receive_data(output)
    }

    pub fn has_received_data(&self) -> bool {
        self.connection.has_received_data()
    }

    pub fn take_diagnostic(&mut self) -> Option<DiagnosticEvent> {
        self.connection.take_diagnostic()
    }

    pub fn take_timestamp_trace(&mut self) -> Option<TimestampTraceEvent> {
        self.connection.take_timestamp_trace()
    }

    pub fn status(&self) -> ConnectionStatus {
        self.connection.state_machine.current_state.into()
    }
}

#[cfg(test)]
mod tests {
    use super::ConnectionStatus;
    use crate::connection::state_machine::RastaState;

    #[test]
    fn public_status_maps_final_closed_state_to_down() {
        assert_eq!(
            ConnectionStatus::from(RastaState::Closed),
            ConnectionStatus::Down
        );
        assert_eq!(
            ConnectionStatus::from(RastaState::Down),
            ConnectionStatus::Opening
        );
        assert_eq!(
            ConnectionStatus::from(RastaState::Start),
            ConnectionStatus::Opening
        );
        assert_eq!(ConnectionStatus::from(RastaState::Up), ConnectionStatus::Up);
        assert_eq!(
            ConnectionStatus::from(RastaState::RetransmissionRequested),
            ConnectionStatus::Retransmission
        );
        assert_eq!(
            ConnectionStatus::from(RastaState::RetransmissionRunning),
            ConnectionStatus::Retransmission
        );
    }
}

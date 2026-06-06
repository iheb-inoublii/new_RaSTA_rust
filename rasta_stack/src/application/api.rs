use crate::core::connection::{RastaConfig, RastaConnection};
use crate::platform::clock::Clock;
use crate::platform::timer::Timer;
use crate::platform::transport::Transport;

pub struct RastaApi<T1: Transport, T2: Transport, TimerCtx: Timer, C: Clock> {
    pub connection: RastaConnection<T1, T2, TimerCtx, C>,
}

impl<T1: Transport, T2: Transport, TimerCtx: Timer, C: Clock> RastaApi<T1, T2, TimerCtx, C> {
    pub fn new(
        transport_a: T1,
        transport_b: T2,
        timer: TimerCtx,
        clock: C,
        config: RastaConfig,
    ) -> Self {
        RastaApi {
            connection: RastaConnection::new(transport_a, transport_b, timer, clock, config),
        }
    }

    pub fn send_data(
        &mut self,
        data: &[u8],
    ) -> Result<(), crate::core::connection::ConnectionError> {
        if self.connection.state_machine.current_state != crate::core::state_machine::RastaState::Up
        {
            return Err(crate::core::connection::ConnectionError::StateTransitionInvalid);
        }
        self.connection
            .send_packet(crate::core::packet::PacketType::Data, data)
    }

    pub fn poll(&mut self) -> Result<(), crate::core::connection::ConnectionError> {
        self.connection.process()
    }
}

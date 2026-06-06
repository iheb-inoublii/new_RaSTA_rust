use crate::platform::transport::{Transport, TransportError};

pub struct TcpLinuxBackend;

impl Transport for TcpLinuxBackend {
    fn send(&mut self, _data: &[u8]) -> Result<(), TransportError> {
        Ok(())
    }
    fn receive(&mut self, _buffer: &mut [u8]) -> Result<usize, TransportError> {
        Ok(0)
    }
}

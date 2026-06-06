use crate::platform::transport::{Transport, TransportError};

// This would use Linux Sockets (libc) in a real project
pub struct UdpLinuxBackend;

impl Transport for UdpLinuxBackend {
    fn send(&mut self, _data: &[u8]) -> Result<(), TransportError> {
        Ok(())
    }
    fn receive(&mut self, _buffer: &mut [u8]) -> Result<usize, TransportError> {
        Ok(0)
    }
}

use std::io;
use std::net::UdpSocket;

use crate::platform::transport::{Transport, TransportError};

pub struct UdpSocketTransport {
    socket: UdpSocket,
}

impl UdpSocketTransport {
    pub fn new(local_addr: &str, remote_addr: &str) -> io::Result<Self> {
        let socket = UdpSocket::bind(local_addr)?;
        socket.connect(remote_addr)?;
        socket.set_nonblocking(true)?;
        Ok(Self { socket })
    }
}

impl Transport for UdpSocketTransport {
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        match self.socket.send(data) {
            Ok(sent) if sent == data.len() => Ok(()),
            Ok(_) | Err(_) => Err(TransportError::SendFailed),
        }
    }

    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError> {
        match self.socket.recv(buffer) {
            Ok(n) => Ok(n),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Ok(0),
            Err(_) => Err(TransportError::ReceiveFailed),
        }
    }
}

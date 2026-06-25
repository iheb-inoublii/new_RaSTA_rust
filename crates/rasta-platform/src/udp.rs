use std::io;
use std::net::UdpSocket;

use rasta_core::port::{Transport, TransportError};

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
            // A connected UDP socket can surface an ICMP "port unreachable" as
            // ConnectionRefused (or ConnectionReset on Windows).  It identifies
            // one lost datagram, not a permanent transport failure; RaSTA's
            // redundancy and timeout supervision decide how to react.
            Err(ref e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::ConnectionRefused
                        | io::ErrorKind::ConnectionReset
                ) =>
            {
                Ok(0)
            }
            Err(_) => Err(TransportError::ReceiveFailed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::UdpSocketTransport;
    use rasta_core::port::Transport;
    use std::net::UdpSocket;

    #[test]
    fn nonblocking_empty_receive_returns_zero() {
        let mut transport =
            UdpSocketTransport::new("127.0.0.1:0", "127.0.0.1:9").expect("bind UDP socket");
        let mut buffer = [0u8; 64];
        assert_eq!(transport.receive(&mut buffer).unwrap(), 0);
    }

    #[test]
    fn invalid_bind_address_fails() {
        assert!(UdpSocketTransport::new("256.256.256.256:1", "127.0.0.1:9").is_err());
    }

    #[test]
    fn sends_and_receives_over_loopback() {
        let reserved_receiver = UdpSocket::bind("127.0.0.1:0").expect("reserve receiver");
        let receiver_addr = reserved_receiver.local_addr().expect("receiver address");
        let reserved_sender = UdpSocket::bind("127.0.0.1:0").expect("reserve sender");
        let sender_addr = reserved_sender.local_addr().expect("sender address");
        drop(reserved_sender);
        drop(reserved_receiver);

        let mut tx = UdpSocketTransport::new(&sender_addr.to_string(), &receiver_addr.to_string())
            .expect("bind transport sender");
        let mut rx = UdpSocketTransport::new(&receiver_addr.to_string(), &sender_addr.to_string())
            .expect("bind transport receiver");

        tx.send(b"rasta").expect("send datagram");
        let mut buffer = [0u8; 16];
        for _ in 0..64 {
            let len = rx.receive(&mut buffer).expect("receive datagram");
            if len != 0 {
                assert_eq!(&buffer[..len], b"rasta");
                return;
            }
            std::thread::yield_now();
        }
        panic!("loopback datagram was not received");
    }
}

use rasta_core::port::{Transport, TransportError};

pub trait EmbeddedEthernetDriver {
    fn send_frame(&mut self, data: &[u8]) -> Result<(), TransportError>;
    fn receive_frame(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError>;
}

pub struct EmbeddedEthernetAdapter<D: EmbeddedEthernetDriver> {
    driver: D,
}

impl<D: EmbeddedEthernetDriver> EmbeddedEthernetAdapter<D> {
    pub fn new(driver: D) -> Self {
        Self { driver }
    }

    pub fn driver(&self) -> &D {
        &self.driver
    }

    pub fn driver_mut(&mut self) -> &mut D {
        &mut self.driver
    }
}

impl<D: EmbeddedEthernetDriver> Transport for EmbeddedEthernetAdapter<D> {
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.driver.send_frame(data)
    }

    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError> {
        self.driver.receive_frame(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::{EmbeddedEthernetAdapter, EmbeddedEthernetDriver};
    use rasta_core::port::{Transport, TransportError};

    #[derive(Default)]
    struct FakeDriver {
        sent: [u8; 16],
        sent_len: usize,
        rx: [u8; 16],
        rx_len: usize,
    }

    impl EmbeddedEthernetDriver for FakeDriver {
        fn send_frame(&mut self, data: &[u8]) -> Result<(), TransportError> {
            if data.len() > self.sent.len() {
                return Err(TransportError::BufferTooSmall);
            }
            self.sent[..data.len()].copy_from_slice(data);
            self.sent_len = data.len();
            Ok(())
        }

        fn receive_frame(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError> {
            if buffer.len() < self.rx_len {
                return Err(TransportError::BufferTooSmall);
            }
            buffer[..self.rx_len].copy_from_slice(&self.rx[..self.rx_len]);
            Ok(self.rx_len)
        }
    }

    #[test]
    fn delegates_send_and_receive_to_driver() {
        let mut driver = FakeDriver::default();
        driver.rx[..4].copy_from_slice(b"pong");
        driver.rx_len = 4;

        let mut adapter = EmbeddedEthernetAdapter::new(driver);
        adapter.send(b"ping").unwrap();
        assert_eq!(&adapter.driver().sent[..adapter.driver().sent_len], b"ping");

        let mut output = [0u8; 8];
        let len = adapter.receive(&mut output).unwrap();
        assert_eq!(&output[..len], b"pong");
    }
}

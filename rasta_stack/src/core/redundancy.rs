// RaSTA Redundancy Layer (EN 50159)
// Handles dual-channel communication, duplication, and duplicate discarding.

use crate::platform::transport::{Transport, TransportError};

pub struct RedundancyLayer<T1: Transport, T2: Transport> {
    transport_a: T1,
    transport_b: T2,
    tx_seq: u32,
    rx_seq: u32,
}

impl<T1: Transport, T2: Transport> RedundancyLayer<T1, T2> {
    pub const HEADER_SIZE: usize = 8;

    pub fn new(transport_a: T1, transport_b: T2) -> Self {
        RedundancyLayer {
            transport_a,
            transport_b,
            tx_seq: 0,
            rx_seq: 0,
        }
    }

    pub fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        let mut buffer = [0u8; 520];
        let total_len = data
            .len()
            .checked_add(Self::HEADER_SIZE)
            .ok_or(TransportError::BufferTooSmall)?;

        if total_len > buffer.len() {
            return Err(TransportError::BufferTooSmall);
        }

        buffer
            .get_mut(0..2)
            .ok_or(TransportError::BufferTooSmall)?
            .copy_from_slice(&(total_len as u16).to_be_bytes());
        buffer
            .get_mut(2..4)
            .ok_or(TransportError::BufferTooSmall)?
            .copy_from_slice(&0u16.to_be_bytes());
        buffer
            .get_mut(4..8)
            .ok_or(TransportError::BufferTooSmall)?
            .copy_from_slice(&self.tx_seq.to_be_bytes());

        let dst = buffer
            .get_mut(8..total_len)
            .ok_or(TransportError::BufferTooSmall)?;
        dst.copy_from_slice(data);

        self.tx_seq = self.tx_seq.wrapping_add(1);

        let res_a = self.transport_a.send(
            buffer
                .get(..total_len)
                .ok_or(TransportError::BufferTooSmall)?,
        );
        let res_b = self.transport_b.send(
            buffer
                .get(..total_len)
                .ok_or(TransportError::BufferTooSmall)?,
        );

        if res_a.is_err() && res_b.is_err() {
            return Err(TransportError::SendFailed);
        }
        Ok(())
    }

    pub fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError> {
        let mut temp_buffer = [0u8; 520];

        let read_res = self
            .transport_a
            .receive(&mut temp_buffer)
            .or_else(|_| self.transport_b.receive(&mut temp_buffer));

        let bytes_read = read_res?;

        if bytes_read > temp_buffer.len() {
            return Err(TransportError::BufferTooSmall);
        }

        if bytes_read < Self::HEADER_SIZE {
            return Ok(0);
        }

        let seq_bytes = temp_buffer
            .get(4..8)
            .ok_or(TransportError::BufferTooSmall)?;
        let r_seq = u32::from_be_bytes([
            *seq_bytes.first().ok_or(TransportError::BufferTooSmall)?,
            *seq_bytes.get(1).ok_or(TransportError::BufferTooSmall)?,
            *seq_bytes.get(2).ok_or(TransportError::BufferTooSmall)?,
            *seq_bytes.get(3).ok_or(TransportError::BufferTooSmall)?,
        ]);

        if r_seq.wrapping_sub(self.rx_seq) < 0x80000000 {
            self.rx_seq = r_seq.wrapping_add(1);
            let payload_len = bytes_read - Self::HEADER_SIZE;
            if payload_len <= buffer.len() {
                let src = temp_buffer
                    .get(Self::HEADER_SIZE..bytes_read)
                    .ok_or(TransportError::BufferTooSmall)?;
                let dst = buffer
                    .get_mut(..payload_len)
                    .ok_or(TransportError::BufferTooSmall)?;
                dst.copy_from_slice(src);
                return Ok(payload_len);
            }
        }

        Ok(0)
    }
}

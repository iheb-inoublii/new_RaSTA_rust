// Redundancy management for the Redundancy Layer.
//
// This layer is intentionally independent from UDP/TCP/Ethernet. It receives
// two objects that implement the portable Transport trait and exposes one
// logical channel to the Safety and Retransmission Layer above it.

use crate::platform::transport::{Transport, TransportError};
use crate::{config::RedundancyCrc, redundancy_crc};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RedundancyCheckCode {
    None,
    OptionB,
    OptionC,
    OptionD,
    OptionE,
}

#[derive(Clone, Copy, Debug)]
pub struct RedundancyConfig {
    pub check_code: RedundancyCheckCode,
    pub t_seq_ms: u32,
}

impl Default for RedundancyConfig {
    fn default() -> Self {
        Self {
            check_code: RedundancyCheckCode::OptionB,
            t_seq_ms: 100,
        }
    }
}

impl RedundancyConfig {
    fn check_code_len(&self) -> usize {
        match self.check_code {
            RedundancyCheckCode::None => 0,
            RedundancyCheckCode::OptionB | RedundancyCheckCode::OptionC => 4,
            RedundancyCheckCode::OptionD | RedundancyCheckCode::OptionE => 2,
        }
    }
}

pub struct RedundancyLayer<T1: Transport, T2: Transport> {
    transport_a: T1,
    transport_b: T2,
    config: RedundancyConfig,
    tx_seq: u32,
    rx_seq: u32,
    deferred: [Option<DeferredFrame>; 4],
}

#[derive(Clone, Copy)]
struct DeferredFrame {
    bytes: [u8; 520],
    len: usize,
    seq: u32,
    received_at_ms: u32,
}

impl<T1: Transport, T2: Transport> RedundancyLayer<T1, T2> {
    pub const HEADER_SIZE: usize = 8;

    pub fn new(transport_a: T1, transport_b: T2) -> Self {
        Self::with_config(transport_a, transport_b, RedundancyConfig::default())
    }

    pub fn with_config(transport_a: T1, transport_b: T2, config: RedundancyConfig) -> Self {
        RedundancyLayer {
            transport_a,
            transport_b,
            config,
            tx_seq: 0,
            rx_seq: 0,
            deferred: [None; 4],
        }
    }

    pub fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        let mut buffer = [0u8; 520];
        let total_len = data
            .len()
            .checked_add(Self::HEADER_SIZE)
            .and_then(|n| n.checked_add(self.config.check_code_len()))
            .ok_or(TransportError::BufferTooSmall)?;

        if total_len > buffer.len() {
            return Err(TransportError::BufferTooSmall);
        }

        buffer
            .get_mut(0..2)
            .ok_or(TransportError::BufferTooSmall)?
            .copy_from_slice(&(total_len as u16).to_le_bytes());
        buffer
            .get_mut(2..4)
            .ok_or(TransportError::BufferTooSmall)?
            .copy_from_slice(&0u16.to_le_bytes());
        buffer
            .get_mut(4..8)
            .ok_or(TransportError::BufferTooSmall)?
            .copy_from_slice(&self.tx_seq.to_le_bytes());

        let dst = buffer
            .get_mut(8..8 + data.len())
            .ok_or(TransportError::BufferTooSmall)?;
        dst.copy_from_slice(data);

        self.write_check_code(&mut buffer, 8 + data.len(), total_len)?;

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
        self.tx_seq = self.tx_seq.wrapping_add(1);
        Ok(())
    }

    pub fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError> {
        self.receive_at(buffer, 0)
    }

    pub fn receive_at(&mut self, buffer: &mut [u8], now_ms: u32) -> Result<usize, TransportError> {
        if let Some(length) = self.deliver_expired_deferred(buffer, now_ms)? {
            return Ok(length);
        }
        let mut temp_buffer = [0u8; 520];
        let mut saw_error = false;

        for channel in 0..2 {
            let read_res = if channel == 0 {
                self.transport_a.receive(&mut temp_buffer)
            } else {
                self.transport_b.receive(&mut temp_buffer)
            };

            match read_res {
                Ok(0) => {}
                Ok(bytes_read) => {
                    if let Some(len) =
                        self.accept_frame(&temp_buffer, bytes_read, buffer, now_ms)?
                    {
                        return Ok(len);
                    }
                }
                Err(TransportError::ReceiveFailed) => {
                    saw_error = true;
                }
                Err(e) => return Err(e),
            }
        }

        if saw_error {
            return Err(TransportError::ReceiveFailed);
        }
        Ok(0)
    }

    fn accept_frame(
        &mut self,
        frame: &[u8],
        bytes_read: usize,
        output: &mut [u8],
        now_ms: u32,
    ) -> Result<Option<usize>, TransportError> {
        let check_len = self.config.check_code_len();
        if bytes_read < Self::HEADER_SIZE + check_len {
            return Ok(None);
        }

        let declared_len = u16::from_le_bytes([
            *frame.first().ok_or(TransportError::BufferTooSmall)?,
            *frame.get(1).ok_or(TransportError::BufferTooSmall)?,
        ]) as usize;
        if declared_len != bytes_read {
            return Err(TransportError::InvalidFrame);
        }
        if !self.check_code_matches(frame, declared_len)? {
            return Err(TransportError::InvalidFrame);
        }

        let seq_bytes = frame.get(4..8).ok_or(TransportError::BufferTooSmall)?;
        let r_seq = u32::from_le_bytes([
            *seq_bytes.first().ok_or(TransportError::BufferTooSmall)?,
            *seq_bytes.get(1).ok_or(TransportError::BufferTooSmall)?,
            *seq_bytes.get(2).ok_or(TransportError::BufferTooSmall)?,
            *seq_bytes.get(3).ok_or(TransportError::BufferTooSmall)?,
        ]);

        if r_seq.wrapping_sub(self.rx_seq) >= 0x80000000 {
            return Ok(None);
        }
        if r_seq != self.rx_seq {
            if r_seq.wrapping_sub(self.rx_seq) > 40 {
                return Err(TransportError::SequenceViolation);
            }
            self.defer(frame, bytes_read, r_seq, now_ms)?;
            return Ok(None);
        }

        self.rx_seq = r_seq.wrapping_add(1);
        let payload_end = declared_len - check_len;
        let payload_len = payload_end - Self::HEADER_SIZE;
        if payload_len > output.len() {
            return Err(TransportError::BufferTooSmall);
        }
        let src = frame
            .get(Self::HEADER_SIZE..payload_end)
            .ok_or(TransportError::BufferTooSmall)?;
        let dst = output
            .get_mut(..payload_len)
            .ok_or(TransportError::BufferTooSmall)?;
        dst.copy_from_slice(src);
        Ok(Some(payload_len))
    }

    fn defer(
        &mut self,
        frame: &[u8],
        len: usize,
        seq: u32,
        received_at_ms: u32,
    ) -> Result<(), TransportError> {
        if self.deferred.iter().flatten().any(|entry| entry.seq == seq) {
            return Ok(());
        }
        let slot = self
            .deferred
            .iter_mut()
            .find(|slot| slot.is_none())
            .ok_or(TransportError::BufferTooSmall)?;
        let mut bytes = [0u8; 520];
        let source = frame.get(..len).ok_or(TransportError::BufferTooSmall)?;
        bytes
            .get_mut(..len)
            .ok_or(TransportError::BufferTooSmall)?
            .copy_from_slice(source);
        *slot = Some(DeferredFrame {
            bytes,
            len,
            seq,
            received_at_ms,
        });
        Ok(())
    }

    fn deliver_expired_deferred(
        &mut self,
        output: &mut [u8],
        now_ms: u32,
    ) -> Result<Option<usize>, TransportError> {
        let mut selected = None;
        for (index, entry) in self.deferred.iter().enumerate() {
            if let Some(frame) = entry
                && (frame.seq == self.rx_seq
                    || now_ms.wrapping_sub(frame.received_at_ms) >= self.config.t_seq_ms)
            {
                selected = Some(index);
                break;
            }
        }
        let Some(index) = selected else {
            return Ok(None);
        };
        let frame = self.deferred[index]
            .take()
            .ok_or(TransportError::InvalidFrame)?;
        self.rx_seq = frame.seq.wrapping_add(1);
        let check_len = self.config.check_code_len();
        let payload_end = frame
            .len
            .checked_sub(check_len)
            .ok_or(TransportError::InvalidFrame)?;
        let payload_len = payload_end
            .checked_sub(Self::HEADER_SIZE)
            .ok_or(TransportError::InvalidFrame)?;
        if payload_len > output.len() {
            return Err(TransportError::BufferTooSmall);
        }
        output
            .get_mut(..payload_len)
            .ok_or(TransportError::BufferTooSmall)?
            .copy_from_slice(
                frame
                    .bytes
                    .get(Self::HEADER_SIZE..payload_end)
                    .ok_or(TransportError::InvalidFrame)?,
            );
        Ok(Some(payload_len))
    }

    fn write_check_code(
        &self,
        frame: &mut [u8],
        check_start: usize,
        total_len: usize,
    ) -> Result<(), TransportError> {
        match self.config.check_code {
            RedundancyCheckCode::None => Ok(()),
            RedundancyCheckCode::OptionB
            | RedundancyCheckCode::OptionC
            | RedundancyCheckCode::OptionD
            | RedundancyCheckCode::OptionE => {
                let option = to_din_crc(self.config.check_code)?;
                let crc = redundancy_crc::calculate(
                    option,
                    frame
                        .get(..check_start)
                        .ok_or(TransportError::BufferTooSmall)?,
                );
                let destination = frame
                    .get_mut(check_start..total_len)
                    .ok_or(TransportError::BufferTooSmall)?;
                if redundancy_crc::check_code_len(option) == 2 {
                    destination.copy_from_slice(&(crc as u16).to_le_bytes());
                } else {
                    destination.copy_from_slice(&crc.to_le_bytes());
                }
                Ok(())
            }
        }
    }

    fn check_code_matches(&self, frame: &[u8], total_len: usize) -> Result<bool, TransportError> {
        let check_len = self.config.check_code_len();
        let check_start = total_len - check_len;
        match self.config.check_code {
            RedundancyCheckCode::None => Ok(true),
            RedundancyCheckCode::OptionB
            | RedundancyCheckCode::OptionC
            | RedundancyCheckCode::OptionD
            | RedundancyCheckCode::OptionE => {
                let option = to_din_crc(self.config.check_code)?;
                let expected = redundancy_crc::calculate(
                    option,
                    frame
                        .get(..check_start)
                        .ok_or(TransportError::BufferTooSmall)?,
                );
                let bytes = frame
                    .get(check_start..total_len)
                    .ok_or(TransportError::BufferTooSmall)?;
                if redundancy_crc::check_code_len(option) == 2 {
                    Ok(u16::from_le_bytes([
                        *bytes.first().ok_or(TransportError::BufferTooSmall)?,
                        *bytes.get(1).ok_or(TransportError::BufferTooSmall)?,
                    ]) == expected as u16)
                } else {
                    Ok(u32::from_le_bytes([
                        *bytes.first().ok_or(TransportError::BufferTooSmall)?,
                        *bytes.get(1).ok_or(TransportError::BufferTooSmall)?,
                        *bytes.get(2).ok_or(TransportError::BufferTooSmall)?,
                        *bytes.get(3).ok_or(TransportError::BufferTooSmall)?,
                    ]) == expected)
                }
            }
        }
    }
}

fn to_din_crc(check_code: RedundancyCheckCode) -> Result<RedundancyCrc, TransportError> {
    match check_code {
        RedundancyCheckCode::OptionB => Ok(RedundancyCrc::OptionB),
        RedundancyCheckCode::OptionC => Ok(RedundancyCrc::OptionC),
        RedundancyCheckCode::OptionD => Ok(RedundancyCrc::OptionD),
        RedundancyCheckCode::OptionE => Ok(RedundancyCrc::OptionE),
        RedundancyCheckCode::None => Err(TransportError::InvalidFrame),
    }
}

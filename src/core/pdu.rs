// PDU / packet handling for the Safety and Retransmission Layer.
//
// Compatibility-related fields are encoded little-endian. Payload bytes are
// opaque: the application layer decides their meaning and byte order.

use crate::core::safety_code::SafetyCodeConfig;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PacketType {
    ConnectionRequest = 6200,
    ConnectionResponse = 6201,
    RetransmissionRequest = 6212,
    RetransmissionResponse = 6213,
    DisconnectionRequest = 6216,
    Heartbeat = 6220,
    Data = 6240,
    RetransmissionData = 6241,
}

impl PacketType {
    pub fn from_u16(val: u16) -> Option<Self> {
        match val {
            6200 => Some(PacketType::ConnectionRequest),
            6201 => Some(PacketType::ConnectionResponse),
            6212 => Some(PacketType::RetransmissionRequest),
            6213 => Some(PacketType::RetransmissionResponse),
            6216 => Some(PacketType::DisconnectionRequest),
            6220 => Some(PacketType::Heartbeat),
            6240 => Some(PacketType::Data),
            6241 => Some(PacketType::RetransmissionData),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum PacketError {
    BufferTooSmall,
    InvalidType,
    ChecksumMismatch,
    InvalidLength,
    InvalidPayload,
    UnsupportedProtocolVersion,
}

#[derive(Clone)]
pub struct Packet {
    pub receiver_id: u32,
    pub sender_id: u32,
    pub sequence_number: u32,
    pub confirmed_sequence_number: u32,
    pub timestamp: u32,
    pub confirmed_timestamp: u32,
    pub packet_type: PacketType,
    // Using fixed size array to avoid dynamic memory (Vec)
    pub payload: [u8; 256],
    pub payload_len: usize,
}

impl Packet {
    pub const HEADER_SIZE: usize = 28;
    pub const MAX_PAYLOAD_SIZE: usize = 256;

    pub fn parse(buffer: &[u8], safety: &SafetyCodeConfig) -> Result<Packet, PacketError> {
        let safety_code_size = safety.len();
        if buffer.len() < Self::HEADER_SIZE + safety_code_size {
            return Err(PacketError::BufferTooSmall);
        }

        // 1. Message Length (2 bytes)
        let msg_len = u16::from_le_bytes([
            *buffer.first().ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(1).ok_or(PacketError::BufferTooSmall)?,
        ]);
        if (msg_len as usize) < Self::HEADER_SIZE + safety_code_size {
            return Err(PacketError::InvalidLength);
        }
        if buffer.len() < msg_len as usize {
            return Err(PacketError::BufferTooSmall);
        }
        if buffer.len() > msg_len as usize {
            return Err(PacketError::InvalidLength);
        }

        // 2. Message Type (2 bytes)
        let type_val = u16::from_le_bytes([
            *buffer.get(2).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(3).ok_or(PacketError::BufferTooSmall)?,
        ]);
        let packet_type = PacketType::from_u16(type_val).ok_or(PacketError::InvalidType)?;

        // 3. Receiver ID (4 bytes)
        let receiver_id = u32::from_le_bytes([
            *buffer.get(4).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(5).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(6).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(7).ok_or(PacketError::BufferTooSmall)?,
        ]);

        // 4. Sender ID (4 bytes)
        let sender_id = u32::from_le_bytes([
            *buffer.get(8).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(9).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(10).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(11).ok_or(PacketError::BufferTooSmall)?,
        ]);

        // 5. Sequence Number (4 bytes)
        let sequence_number = u32::from_le_bytes([
            *buffer.get(12).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(13).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(14).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(15).ok_or(PacketError::BufferTooSmall)?,
        ]);

        // 6. Confirmed Sequence Number (4 bytes)
        let confirmed_sequence_number = u32::from_le_bytes([
            *buffer.get(16).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(17).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(18).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(19).ok_or(PacketError::BufferTooSmall)?,
        ]);

        // 7. Timestamp (4 bytes)
        let timestamp = u32::from_le_bytes([
            *buffer.get(20).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(21).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(22).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(23).ok_or(PacketError::BufferTooSmall)?,
        ]);

        // 8. Confirmed Timestamp (4 bytes)
        let confirmed_timestamp = u32::from_le_bytes([
            *buffer.get(24).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(25).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(26).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(27).ok_or(PacketError::BufferTooSmall)?,
        ]);

        // 9. Safety Code (Checksum) - at the end of the PDU
        let safety_code_start = msg_len as usize - safety_code_size;
        let received_safety_code = buffer
            .get(safety_code_start..msg_len as usize)
            .ok_or(PacketError::BufferTooSmall)?;

        if safety_code_size > 0 {
            let calc_bytes = safety.calculate(
                buffer
                    .get(0..safety_code_start)
                    .ok_or(PacketError::BufferTooSmall)?,
            );
            let expected = calc_bytes
                .get(..safety_code_size)
                .ok_or(PacketError::InvalidLength)?;
            if received_safety_code != expected {
                return Err(PacketError::ChecksumMismatch);
            }
        }

        // 10. Payload
        let payload_len = safety_code_start - Self::HEADER_SIZE;
        if payload_len > Self::MAX_PAYLOAD_SIZE {
            return Err(PacketError::BufferTooSmall);
        }

        let mut payload = [0u8; Self::MAX_PAYLOAD_SIZE];
        if payload_len > 0 {
            let src = buffer
                .get(Self::HEADER_SIZE..safety_code_start)
                .ok_or(PacketError::BufferTooSmall)?;
            let dst = payload
                .get_mut(..payload_len)
                .ok_or(PacketError::BufferTooSmall)?;
            dst.copy_from_slice(src);
        }

        let packet = Packet {
            receiver_id,
            sender_id,
            sequence_number,
            confirmed_sequence_number,
            timestamp,
            confirmed_timestamp,
            packet_type,
            payload,
            payload_len,
        };
        packet.validate_payload_structure()?;
        Ok(packet)
    }

    pub fn serialize(
        &self,
        buffer: &mut [u8],
        safety: &SafetyCodeConfig,
    ) -> Result<usize, PacketError> {
        if self.payload_len > Self::MAX_PAYLOAD_SIZE {
            return Err(PacketError::InvalidLength);
        }
        self.validate_payload_structure()?;
        let safety_code_size = safety.len();
        let msg_len = Self::HEADER_SIZE + self.payload_len + safety_code_size;
        if buffer.len() < msg_len {
            return Err(PacketError::BufferTooSmall);
        }

        // 1. Message Length
        let len_bytes = (msg_len as u16).to_le_bytes();
        buffer
            .get_mut(0..2)
            .ok_or(PacketError::BufferTooSmall)?
            .copy_from_slice(&len_bytes);

        // 2. Message Type
        let type_bytes = (self.packet_type as u16).to_le_bytes();
        buffer
            .get_mut(2..4)
            .ok_or(PacketError::BufferTooSmall)?
            .copy_from_slice(&type_bytes);

        // 3. Receiver ID
        buffer
            .get_mut(4..8)
            .ok_or(PacketError::BufferTooSmall)?
            .copy_from_slice(&self.receiver_id.to_le_bytes());

        // 4. Sender ID
        buffer
            .get_mut(8..12)
            .ok_or(PacketError::BufferTooSmall)?
            .copy_from_slice(&self.sender_id.to_le_bytes());

        // 5. Sequence Number
        buffer
            .get_mut(12..16)
            .ok_or(PacketError::BufferTooSmall)?
            .copy_from_slice(&self.sequence_number.to_le_bytes());

        // 6. Confirmed Sequence Number
        buffer
            .get_mut(16..20)
            .ok_or(PacketError::BufferTooSmall)?
            .copy_from_slice(&self.confirmed_sequence_number.to_le_bytes());

        // 7. Timestamp
        buffer
            .get_mut(20..24)
            .ok_or(PacketError::BufferTooSmall)?
            .copy_from_slice(&self.timestamp.to_le_bytes());

        // 8. Confirmed Timestamp
        buffer
            .get_mut(24..28)
            .ok_or(PacketError::BufferTooSmall)?
            .copy_from_slice(&self.confirmed_timestamp.to_le_bytes());

        // 9. Payload
        if self.payload_len > 0 {
            let src = self
                .payload
                .get(..self.payload_len)
                .ok_or(PacketError::InvalidLength)?;
            let dst = buffer
                .get_mut(Self::HEADER_SIZE..Self::HEADER_SIZE + self.payload_len)
                .ok_or(PacketError::BufferTooSmall)?;
            dst.copy_from_slice(src);
        }

        // 10. Safety Code. The length is configured by the connection.
        let safety_code_start = Self::HEADER_SIZE + self.payload_len;
        if safety_code_size > 0 {
            let calc_bytes = safety.calculate(
                buffer
                    .get(0..safety_code_start)
                    .ok_or(PacketError::BufferTooSmall)?,
            );
            let src = calc_bytes
                .get(..safety_code_size)
                .ok_or(PacketError::InvalidLength)?;
            buffer
                .get_mut(safety_code_start..safety_code_start + safety_code_size)
                .ok_or(PacketError::BufferTooSmall)?
                .copy_from_slice(src);
        }

        Ok(msg_len)
    }

    fn validate_payload_structure(&self) -> Result<(), PacketError> {
        match self.packet_type {
            PacketType::ConnectionRequest | PacketType::ConnectionResponse => {
                if self.payload_len != 14 {
                    return Err(PacketError::InvalidPayload);
                }
                if self.payload.get(0..4) != Some(b"0303") {
                    return Err(PacketError::UnsupportedProtocolVersion);
                }
                if self.payload.get(6..14) != Some(&[0; 8]) {
                    return Err(PacketError::InvalidPayload);
                }
            }
            PacketType::RetransmissionRequest
            | PacketType::RetransmissionResponse
            | PacketType::Heartbeat => {
                if self.payload_len != 0 {
                    return Err(PacketError::InvalidPayload);
                }
            }
            PacketType::DisconnectionRequest => {
                if self.payload_len != 4 {
                    return Err(PacketError::InvalidPayload);
                }
            }
            PacketType::Data | PacketType::RetransmissionData => {}
        }
        Ok(())
    }
}

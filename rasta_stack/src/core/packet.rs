// Packet serialization and parsing according to RaSTA standard (EN 50159)
// Header size: 28 bytes
// Safety Code: 16 bytes (MD4)

use crate::core::safety_code::Md4;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PacketType {
    ConnectionRequest = 6200,
    ConnectionResponse = 6201,
    RetransmissionRequest = 6202,
    RetransmissionResponse = 6203,
    DisconnectionRequest = 6204,
    Heartbeat = 6205,
    Data = 6240,
    RetransmissionData = 6241,
}

impl PacketType {
    pub fn from_u16(val: u16) -> Option<Self> {
        match val {
            6200 => Some(PacketType::ConnectionRequest),
            6201 => Some(PacketType::ConnectionResponse),
            6202 => Some(PacketType::RetransmissionRequest),
            6203 => Some(PacketType::RetransmissionResponse),
            6204 => Some(PacketType::DisconnectionRequest),
            6205 => Some(PacketType::Heartbeat),
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
}

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
    pub const SAFETY_CODE_SIZE: usize = 16; // MD4 is 128 bits

    pub fn parse(buffer: &[u8], key: &[u8]) -> Result<Packet, PacketError> {
        if buffer.len() < Self::HEADER_SIZE + Self::SAFETY_CODE_SIZE {
            return Err(PacketError::BufferTooSmall);
        }

        // 1. Message Length (2 bytes)
        let msg_len = u16::from_be_bytes([
            *buffer.first().ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(1).ok_or(PacketError::BufferTooSmall)?,
        ]);
        if (msg_len as usize) < Self::HEADER_SIZE + Self::SAFETY_CODE_SIZE {
            return Err(PacketError::InvalidLength);
        }
        if buffer.len() < msg_len as usize {
            return Err(PacketError::BufferTooSmall);
        }

        // 2. Message Type (2 bytes)
        let type_val = u16::from_be_bytes([
            *buffer.get(2).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(3).ok_or(PacketError::BufferTooSmall)?,
        ]);
        let packet_type = PacketType::from_u16(type_val).ok_or(PacketError::InvalidType)?;

        // 3. Receiver ID (4 bytes)
        let receiver_id = u32::from_be_bytes([
            *buffer.get(4).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(5).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(6).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(7).ok_or(PacketError::BufferTooSmall)?,
        ]);

        // 4. Sender ID (4 bytes)
        let sender_id = u32::from_be_bytes([
            *buffer.get(8).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(9).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(10).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(11).ok_or(PacketError::BufferTooSmall)?,
        ]);

        // 5. Sequence Number (4 bytes)
        let sequence_number = u32::from_be_bytes([
            *buffer.get(12).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(13).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(14).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(15).ok_or(PacketError::BufferTooSmall)?,
        ]);

        // 6. Confirmed Sequence Number (4 bytes)
        let confirmed_sequence_number = u32::from_be_bytes([
            *buffer.get(16).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(17).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(18).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(19).ok_or(PacketError::BufferTooSmall)?,
        ]);

        // 7. Timestamp (4 bytes)
        let timestamp = u32::from_be_bytes([
            *buffer.get(20).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(21).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(22).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(23).ok_or(PacketError::BufferTooSmall)?,
        ]);

        // 8. Confirmed Timestamp (4 bytes)
        let confirmed_timestamp = u32::from_be_bytes([
            *buffer.get(24).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(25).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(26).ok_or(PacketError::BufferTooSmall)?,
            *buffer.get(27).ok_or(PacketError::BufferTooSmall)?,
        ]);

        // 9. Safety Code (Checksum) - at the end of the PDU
        let safety_code_start = msg_len as usize - Self::SAFETY_CODE_SIZE;
        let received_safety_code = buffer
            .get(safety_code_start..msg_len as usize)
            .ok_or(PacketError::BufferTooSmall)?;

        // MD4 Checksum: MD4(key | data)
        let mut md4 = Md4::new();
        md4.update(key);
        md4.update(
            buffer
                .get(0..safety_code_start)
                .ok_or(PacketError::BufferTooSmall)?,
        );
        let calc_bytes = md4.finalize();

        if received_safety_code != calc_bytes {
            return Err(PacketError::ChecksumMismatch);
        }

        // 10. Payload
        let payload_len = safety_code_start - Self::HEADER_SIZE;
        if payload_len > 256 {
            return Err(PacketError::BufferTooSmall);
        }

        let mut payload = [0u8; 256];
        if payload_len > 0 {
            let src = buffer
                .get(Self::HEADER_SIZE..safety_code_start)
                .ok_or(PacketError::BufferTooSmall)?;
            let dst = payload
                .get_mut(..payload_len)
                .ok_or(PacketError::BufferTooSmall)?;
            dst.copy_from_slice(src);
        }

        Ok(Packet {
            receiver_id,
            sender_id,
            sequence_number,
            confirmed_sequence_number,
            timestamp,
            confirmed_timestamp,
            packet_type,
            payload,
            payload_len,
        })
    }

    pub fn serialize(&self, buffer: &mut [u8], key: &[u8]) -> Result<usize, PacketError> {
        if self.payload_len > 256 {
            return Err(PacketError::InvalidLength);
        }
        let msg_len = Self::HEADER_SIZE + self.payload_len + Self::SAFETY_CODE_SIZE;
        if buffer.len() < msg_len {
            return Err(PacketError::BufferTooSmall);
        }

        // 1. Message Length
        let len_bytes = (msg_len as u16).to_be_bytes();
        buffer
            .get_mut(0..2)
            .ok_or(PacketError::BufferTooSmall)?
            .copy_from_slice(&len_bytes);

        // 2. Message Type
        let type_bytes = (self.packet_type as u16).to_be_bytes();
        buffer
            .get_mut(2..4)
            .ok_or(PacketError::BufferTooSmall)?
            .copy_from_slice(&type_bytes);

        // 3. Receiver ID
        buffer
            .get_mut(4..8)
            .ok_or(PacketError::BufferTooSmall)?
            .copy_from_slice(&self.receiver_id.to_be_bytes());

        // 4. Sender ID
        buffer
            .get_mut(8..12)
            .ok_or(PacketError::BufferTooSmall)?
            .copy_from_slice(&self.sender_id.to_be_bytes());

        // 5. Sequence Number
        buffer
            .get_mut(12..16)
            .ok_or(PacketError::BufferTooSmall)?
            .copy_from_slice(&self.sequence_number.to_be_bytes());

        // 6. Confirmed Sequence Number
        buffer
            .get_mut(16..20)
            .ok_or(PacketError::BufferTooSmall)?
            .copy_from_slice(&self.confirmed_sequence_number.to_be_bytes());

        // 7. Timestamp
        buffer
            .get_mut(20..24)
            .ok_or(PacketError::BufferTooSmall)?
            .copy_from_slice(&self.timestamp.to_be_bytes());

        // 8. Confirmed Timestamp
        buffer
            .get_mut(24..28)
            .ok_or(PacketError::BufferTooSmall)?
            .copy_from_slice(&self.confirmed_timestamp.to_be_bytes());

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

        // 10. Safety Code
        let safety_code_start = Self::HEADER_SIZE + self.payload_len;
        let mut md4 = Md4::new();
        md4.update(key);
        md4.update(
            buffer
                .get(0..safety_code_start)
                .ok_or(PacketError::BufferTooSmall)?,
        );
        let calc_bytes = md4.finalize();
        buffer
            .get_mut(safety_code_start..safety_code_start + Self::SAFETY_CODE_SIZE)
            .ok_or(PacketError::BufferTooSmall)?
            .copy_from_slice(&calc_bytes);

        Ok(msg_len)
    }
}

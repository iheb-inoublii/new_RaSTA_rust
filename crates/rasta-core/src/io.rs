#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketIoError {
    Truncated,
    BufferFull,
}

pub struct PacketReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> PacketReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    pub fn read_u16_le(&mut self) -> Result<u16, PacketIoError> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_u32_le(&mut self) -> Result<u32, PacketIoError> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn take(&mut self, len: usize) -> Result<&'a [u8], PacketIoError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(PacketIoError::Truncated)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(PacketIoError::Truncated)?;
        self.offset = end;
        Ok(value)
    }

    pub fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }
}

pub struct PacketWriter<'a> {
    bytes: &'a mut [u8],
    offset: usize,
}

impl<'a> PacketWriter<'a> {
    pub fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    pub fn write_u16_le(&mut self, value: u16) -> Result<(), PacketIoError> {
        self.write_bytes(&value.to_le_bytes())
    }

    pub fn write_u32_le(&mut self, value: u32) -> Result<(), PacketIoError> {
        self.write_bytes(&value.to_le_bytes())
    }

    pub fn write_bytes(&mut self, value: &[u8]) -> Result<(), PacketIoError> {
        let end = self
            .offset
            .checked_add(value.len())
            .ok_or(PacketIoError::BufferFull)?;
        let target = self
            .bytes
            .get_mut(self.offset..end)
            .ok_or(PacketIoError::BufferFull)?;
        target.copy_from_slice(value);
        self.offset = end;
        Ok(())
    }

    pub fn written(&self) -> usize {
        self.offset
    }
}

#[cfg(test)]
mod tests {
    use super::{PacketIoError, PacketReader, PacketWriter};

    #[test]
    fn reader_and_writer_are_checked() {
        let mut bytes = [0u8; 6];
        let mut writer = PacketWriter::new(&mut bytes);
        assert_eq!(writer.write_u16_le(0x1234), Ok(()));
        assert_eq!(writer.write_u32_le(0x89ab_cdef), Ok(()));
        assert_eq!(writer.write_bytes(&[1]), Err(PacketIoError::BufferFull));

        let mut reader = PacketReader::new(&bytes);
        assert_eq!(reader.read_u16_le(), Ok(0x1234));
        assert_eq!(reader.read_u32_le(), Ok(0x89ab_cdef));
        assert_eq!(reader.read_u16_le(), Err(PacketIoError::Truncated));
        assert_eq!(reader.remaining(), 0);
    }
}

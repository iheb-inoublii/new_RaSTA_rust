use crate::port::TransportError;

pub(crate) const HEADER_SIZE: usize = 8;
pub(crate) const MAX_FRAME_SIZE: usize = 520;

#[derive(Debug)]
pub(crate) struct FrameHeader {
    pub(crate) declared_len: usize,
    pub(crate) sequence: u32,
}

pub(crate) fn write_header(
    frame: &mut [u8],
    total_len: usize,
    sequence: u32,
) -> Result<(), TransportError> {
    frame
        .get_mut(0..2)
        .ok_or(TransportError::BufferTooSmall)?
        .copy_from_slice(&(total_len as u16).to_le_bytes());
    frame
        .get_mut(2..4)
        .ok_or(TransportError::BufferTooSmall)?
        .copy_from_slice(&0u16.to_le_bytes());
    frame
        .get_mut(4..8)
        .ok_or(TransportError::BufferTooSmall)?
        .copy_from_slice(&sequence.to_le_bytes());
    Ok(())
}

pub(crate) fn parse_header(frame: &[u8]) -> Result<FrameHeader, TransportError> {
    let declared_len = u16::from_le_bytes([
        *frame.first().ok_or(TransportError::BufferTooSmall)?,
        *frame.get(1).ok_or(TransportError::BufferTooSmall)?,
    ]) as usize;
    let sequence_bytes = frame.get(4..8).ok_or(TransportError::BufferTooSmall)?;
    let sequence = u32::from_le_bytes([
        *sequence_bytes
            .first()
            .ok_or(TransportError::BufferTooSmall)?,
        *sequence_bytes
            .get(1)
            .ok_or(TransportError::BufferTooSmall)?,
        *sequence_bytes
            .get(2)
            .ok_or(TransportError::BufferTooSmall)?,
        *sequence_bytes
            .get(3)
            .ok_or(TransportError::BufferTooSmall)?,
    ]);
    Ok(FrameHeader {
        declared_len,
        sequence,
    })
}

pub(crate) fn payload_range(
    total_len: usize,
    check_code_len: usize,
) -> Result<core::ops::Range<usize>, TransportError> {
    let payload_end = total_len
        .checked_sub(check_code_len)
        .ok_or(TransportError::InvalidFrame)?;
    if payload_end < HEADER_SIZE {
        return Err(TransportError::InvalidFrame);
    }
    Ok(HEADER_SIZE..payload_end)
}

#[cfg(test)]
mod tests {
    use super::{HEADER_SIZE, parse_header, payload_range, write_header};
    use crate::port::TransportError;

    #[test]
    fn encodes_and_decodes_little_endian_header() {
        let mut frame = [0u8; HEADER_SIZE];
        write_header(&mut frame, 0x1234, 0x89ab_cdef).unwrap();
        assert_eq!(&frame[..4], &[0x34, 0x12, 0, 0]);
        let header = parse_header(&frame).unwrap();
        assert_eq!(header.declared_len, 0x1234);
        assert_eq!(header.sequence, 0x89ab_cdef);
    }

    #[test]
    fn malformed_header_and_lengths_return_errors() {
        assert_eq!(
            parse_header(&[]).unwrap_err(),
            TransportError::BufferTooSmall
        );
        assert_eq!(payload_range(7, 0), Err(TransportError::InvalidFrame));
    }
}

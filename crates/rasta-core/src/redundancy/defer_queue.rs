use crate::port::TransportError;

use super::frame::MAX_FRAME_SIZE;

#[derive(Clone, Copy)]
pub(crate) struct DeferredFrame {
    pub(crate) bytes: [u8; MAX_FRAME_SIZE],
    pub(crate) len: usize,
    pub(crate) sequence: u32,
    pub(crate) received_at_ms: u32,
}

pub(crate) struct DeferQueue {
    entries: [Option<DeferredFrame>; 4],
}

impl DeferQueue {
    pub(crate) fn new() -> Self {
        Self { entries: [None; 4] }
    }

    pub(crate) fn insert(
        &mut self,
        frame: &[u8],
        len: usize,
        sequence: u32,
        received_at_ms: u32,
    ) -> Result<(), TransportError> {
        if self
            .entries
            .iter()
            .flatten()
            .any(|entry| entry.sequence == sequence)
        {
            return Ok(());
        }
        let slot = self
            .entries
            .iter_mut()
            .find(|slot| slot.is_none())
            .ok_or(TransportError::BufferTooSmall)?;
        let mut bytes = [0u8; MAX_FRAME_SIZE];
        let source = frame.get(..len).ok_or(TransportError::BufferTooSmall)?;
        bytes
            .get_mut(..len)
            .ok_or(TransportError::BufferTooSmall)?
            .copy_from_slice(source);
        *slot = Some(DeferredFrame {
            bytes,
            len,
            sequence,
            received_at_ms,
        });
        Ok(())
    }

    pub(crate) fn take_ready(
        &mut self,
        expected_sequence: u32,
        now_ms: u32,
        t_seq_ms: u32,
    ) -> Option<DeferredFrame> {
        let index = self.entries.iter().position(|entry| {
            entry.is_some_and(|frame| {
                frame.sequence == expected_sequence
                    || now_ms.wrapping_sub(frame.received_at_ms) >= t_seq_ms
            })
        })?;
        self.entries[index].take()
    }
}

#[cfg(test)]
mod tests {
    use super::DeferQueue;
    use crate::port::TransportError;

    #[test]
    fn retains_duplicates_once_and_reports_full_queue() {
        let mut queue = DeferQueue::new();
        let frame = [0u8; 8];
        assert_eq!(queue.insert(&frame, 8, 1, 0), Ok(()));
        assert_eq!(queue.insert(&frame, 8, 1, 0), Ok(()));
        for sequence in 2..=4 {
            assert_eq!(queue.insert(&frame, 8, sequence, 0), Ok(()));
        }
        assert_eq!(
            queue.insert(&frame, 8, 5, 0),
            Err(TransportError::BufferTooSmall)
        );
    }

    #[test]
    fn releases_expected_or_expired_entries_across_wrap() {
        let mut queue = DeferQueue::new();
        let mut frame = [0u8; 8];
        frame[0] = 1;
        queue.insert(&frame, 8, 2, u32::MAX - 5).unwrap();
        assert!(queue.take_ready(1, u32::MAX, 10).is_none());
        let released = queue.take_ready(1, 4, 10).unwrap();
        assert_eq!(released.sequence, 2);

        queue.insert(&frame, 8, 3, 0).unwrap();
        assert_eq!(queue.take_ready(3, 0, 100).unwrap().sequence, 3);
    }
}

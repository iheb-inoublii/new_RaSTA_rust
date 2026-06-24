use crate::port::TransportError;
use crate::time::{DurationMs, MonotonicInstant};

use super::frame::MAX_FRAME_SIZE;

#[derive(Clone, Copy)]
pub(crate) struct DeferredFrame {
    pub(crate) bytes: [u8; MAX_FRAME_SIZE],
    pub(crate) len: usize,
    pub(crate) sequence: u32,
    pub(crate) received_at: MonotonicInstant,
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
        received_at: MonotonicInstant,
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
            received_at,
        });
        Ok(())
    }

    pub(crate) fn take_ready(
        &mut self,
        expected_sequence: u32,
        now: MonotonicInstant,
        t_seq: DurationMs,
    ) -> Option<DeferredFrame> {
        let index = self.entries.iter().position(|entry| {
            entry.is_some_and(|frame| {
                frame.sequence == expected_sequence
                    || now.elapsed_since(frame.received_at).as_millis() >= t_seq.as_millis()
            })
        })?;
        self.entries[index].take()
    }
}

#[cfg(test)]
mod tests {
    use super::DeferQueue;
    use crate::port::TransportError;
    use crate::time::{DurationMs, MonotonicInstant};

    #[test]
    fn retains_duplicates_once_and_reports_full_queue() {
        let mut queue = DeferQueue::new();
        let frame = [0u8; 8];
        let now = MonotonicInstant::from_wrapping_millis(0);
        assert_eq!(queue.insert(&frame, 8, 1, now), Ok(()));
        assert_eq!(queue.insert(&frame, 8, 1, now), Ok(()));
        for sequence in 2..=4 {
            assert_eq!(queue.insert(&frame, 8, sequence, now), Ok(()));
        }
        assert_eq!(
            queue.insert(&frame, 8, 5, now),
            Err(TransportError::BufferTooSmall)
        );
    }

    #[test]
    fn releases_expected_or_expired_entries_across_wrap() {
        let mut queue = DeferQueue::new();
        let mut frame = [0u8; 8];
        frame[0] = 1;
        queue
            .insert(
                &frame,
                8,
                2,
                MonotonicInstant::from_wrapping_millis(u32::MAX - 5),
            )
            .unwrap();
        assert!(
            queue
                .take_ready(
                    1,
                    MonotonicInstant::from_wrapping_millis(u32::MAX),
                    DurationMs::from_millis(10),
                )
                .is_none()
        );
        let released = queue
            .take_ready(
                1,
                MonotonicInstant::from_wrapping_millis(4),
                DurationMs::from_millis(10),
            )
            .unwrap();
        assert_eq!(released.sequence, 2);

        queue
            .insert(&frame, 8, 3, MonotonicInstant::from_wrapping_millis(0))
            .unwrap();
        assert_eq!(
            queue
                .take_ready(
                    3,
                    MonotonicInstant::from_wrapping_millis(0),
                    DurationMs::from_millis(100),
                )
                .unwrap()
                .sequence,
            3
        );
    }
}

use crate::serial::HALF_RANGE;

pub(crate) enum ReceiveSequence {
    Expected,
    StaleOrDuplicate,
    Ahead,
    Violation,
}

pub(crate) struct RedundancySequence {
    tx: u32,
    rx: u32,
}

impl RedundancySequence {
    pub(crate) fn new() -> Self {
        Self { tx: 0, rx: 0 }
    }

    pub(crate) fn transmit(&self) -> u32 {
        self.tx
    }

    pub(crate) fn advance_transmit(&mut self) {
        self.tx = self.tx.wrapping_add(1);
    }

    pub(crate) fn classify_receive(&self, sequence: u32) -> ReceiveSequence {
        let distance = sequence.wrapping_sub(self.rx);
        if distance >= HALF_RANGE {
            ReceiveSequence::StaleOrDuplicate
        } else if sequence == self.rx {
            ReceiveSequence::Expected
        } else if distance > 40 {
            ReceiveSequence::Violation
        } else {
            ReceiveSequence::Ahead
        }
    }

    pub(crate) fn expected(&self) -> u32 {
        self.rx
    }

    pub(crate) fn accept(&mut self, sequence: u32) {
        self.rx = sequence.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::{ReceiveSequence, RedundancySequence};

    #[test]
    fn classifies_expected_duplicate_stale_and_ahead_sequences() {
        let mut sequence = RedundancySequence::new();
        assert!(matches!(
            sequence.classify_receive(0),
            ReceiveSequence::Expected
        ));
        sequence.accept(0);
        assert!(matches!(
            sequence.classify_receive(1),
            ReceiveSequence::Expected
        ));
        assert!(matches!(
            sequence.classify_receive(0),
            ReceiveSequence::StaleOrDuplicate
        ));
        assert!(matches!(
            sequence.classify_receive(2),
            ReceiveSequence::Ahead
        ));
        assert!(matches!(
            sequence.classify_receive(42),
            ReceiveSequence::Violation
        ));
    }

    #[test]
    fn wraps_and_keeps_half_range_stale() {
        let mut sequence = RedundancySequence::new();
        sequence.accept(u32::MAX);
        assert_eq!(sequence.expected(), 0);
        assert!(matches!(
            sequence.classify_receive(0),
            ReceiveSequence::Expected
        ));
        assert!(matches!(
            sequence.classify_receive(0x8000_0000),
            ReceiveSequence::StaleOrDuplicate
        ));
    }
}

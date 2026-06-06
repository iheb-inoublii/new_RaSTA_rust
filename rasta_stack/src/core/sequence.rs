// Sequence number handling for RaSTA
// Sequence numbers must be strictly sequential. Gaps trigger retransmission.

pub struct SequenceHandler {
    current_tx: u32,
    current_rx: u32,
}

#[derive(Debug, PartialEq)]
pub enum SequenceResult {
    Ok,
    Gap(u32),  // Received sequence is higher than expected (gap detected)
    Duplicate, // Received sequence is older than or equal to current_rx
}

impl Default for SequenceHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl SequenceHandler {
    pub fn new() -> Self {
        SequenceHandler {
            current_tx: 0,
            current_rx: 0,
        }
    }

    pub fn with_initial_tx(initial: u32) -> Self {
        SequenceHandler {
            current_tx: initial,
            current_rx: 0,
        }
    }

    pub fn next_tx(&mut self) -> u32 {
        let seq = self.current_tx;
        self.current_tx = self.current_tx.wrapping_add(1);
        seq
    }

    pub fn validate_rx(&mut self, received_seq: u32) -> SequenceResult {
        if received_seq == self.current_rx {
            self.current_rx = self.current_rx.wrapping_add(1);
            SequenceResult::Ok
        } else if received_seq.wrapping_sub(self.current_rx) < 0x80000000 {
            // received_seq > current_rx: we missed some packets
            SequenceResult::Gap(self.current_rx)
        } else {
            // received_seq < current_rx: old packet/duplicate
            SequenceResult::Duplicate
        }
    }

    pub fn expected_rx(&self) -> u32 {
        self.current_rx
    }

    pub fn confirmed_seq(&self) -> u32 {
        if self.current_rx == 0 {
            0
        } else {
            self.current_rx.wrapping_sub(1)
        }
    }

    pub fn last_received_seq(&self) -> Option<u32> {
        if self.current_rx == 0 {
            None
        } else {
            Some(self.current_rx.wrapping_sub(1))
        }
    }
}

// Retransmission logic with ordered retrieval
// RaSTA requires retransmitted packets to be in strict sequence.

use crate::core::packet::Packet;

pub struct RetransmissionBuffer {
    pub packets: [Option<Packet>; 16],
    // We track the oldest sequence number currently in the buffer
    oldest_seq: Option<u32>,
}

impl Default for RetransmissionBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl RetransmissionBuffer {
    pub fn new() -> Self {
        const NONE_PACKET: Option<Packet> = None;
        RetransmissionBuffer {
            packets: [NONE_PACKET; 16],
            oldest_seq: None,
        }
    }

    pub fn store(&mut self, packet: Packet) -> bool {
        for slot in self.packets.iter_mut() {
            if slot.is_none() {
                if self.oldest_seq.is_none() {
                    self.oldest_seq = Some(packet.sequence_number);
                }
                *slot = Some(packet);
                return true;
            }
        }
        false // Buffer full!
    }

    pub fn clear_up_to(&mut self, confirmed_seq: u32) {
        // Remove packets that the other side acknowledged (seq <= confirmed_seq)
        for slot in self.packets.iter_mut() {
            if slot
                .as_mut()
                .filter(|p| confirmed_seq.wrapping_sub(p.sequence_number) < 0x80000000)
                .is_some()
            {
                *slot = None;
            }
        }

        // Recalculate oldest_seq
        self.oldest_seq = None;
        for p in self.packets.iter().flatten() {
            match self.oldest_seq {
                None => self.oldest_seq = Some(p.sequence_number),
                Some(old) => {
                    if p.sequence_number.wrapping_sub(old) > 0x80000000 {
                        self.oldest_seq = Some(p.sequence_number);
                    }
                }
            }
        }
    }

    pub fn get_packet(&self, seq: u32) -> Option<&Packet> {
        self.packets
            .iter()
            .flatten()
            .find(|p| p.sequence_number == seq)
    }

    pub fn count(&self) -> usize {
        self.packets.iter().flatten().count()
    }
}

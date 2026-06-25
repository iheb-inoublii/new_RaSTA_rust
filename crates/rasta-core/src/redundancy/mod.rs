//! RaSTA Redundancy Layer framing, check codes, sequencing, and channels.

mod channel;
mod crc;
mod defer_queue;
mod frame;
mod sequence;

pub use channel::RedundancyLayer;
pub use crc::{RedundancyCrc, calculate, check_code_len};

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
    pub(crate) fn check_code_len(&self) -> usize {
        match self.check_code {
            RedundancyCheckCode::None => 0,
            RedundancyCheckCode::OptionB | RedundancyCheckCode::OptionC => 4,
            RedundancyCheckCode::OptionD | RedundancyCheckCode::OptionE => 2,
        }
    }

    pub(crate) fn crc_option(&self) -> Option<RedundancyCrc> {
        match self.check_code {
            RedundancyCheckCode::None => None,
            RedundancyCheckCode::OptionB => Some(RedundancyCrc::OptionB),
            RedundancyCheckCode::OptionC => Some(RedundancyCrc::OptionC),
            RedundancyCheckCode::OptionD => Some(RedundancyCrc::OptionD),
            RedundancyCheckCode::OptionE => Some(RedundancyCrc::OptionE),
        }
    }
}

//! DIN VDE V 0831-200:2015-06, clause 6.3.6 redundancy check codes.

use crate::config::RedundancyCrc;

pub fn check_code_len(option: RedundancyCrc) -> usize {
    match option {
        RedundancyCrc::OptionB | RedundancyCrc::OptionC => 4,
        RedundancyCrc::OptionD | RedundancyCrc::OptionE => 2,
    }
}

pub fn calculate(option: RedundancyCrc, data: &[u8]) -> u32 {
    match option {
        // Clause 6.3.6 option b: width 32, poly EE5B42FD, init 0,
        // refin/refout false, xorout 0, check("123456789") = 0E7C650A.
        RedundancyCrc::OptionB => crc32_normal(data, 0xee5b_42fd, 0),
        // Clause 6.3.6 option c: CRC-32C. The optimized reflected form uses
        // init FFFFFFFF, refin/refout true, xorout FFFFFFFF.
        // check("123456789") = E3069283.
        RedundancyCrc::OptionC => crc32_reflected(data, 0x82f6_3b78, 0xffff_ffff) ^ 0xffff_ffff,
        // Clause 6.3.6 option d: width 16, poly 1021, init 0,
        // refin/refout true, xorout 0, check("123456789") = 2189.
        RedundancyCrc::OptionD => crc16_reflected(data, 0x8408, 0) as u32,
        // Clause 6.3.6 option e: width 16, poly 8005, init 0,
        // refin/refout true, xorout 0, check("123456789") = BB3D.
        RedundancyCrc::OptionE => crc16_reflected(data, 0xa001, 0) as u32,
    }
}

fn crc32_normal(data: &[u8], polynomial: u32, initial: u32) -> u32 {
    let mut crc = initial;
    for &byte in data {
        crc ^= (byte as u32) << 24;
        for _ in 0..8 {
            crc = if crc & 0x8000_0000 != 0 {
                (crc << 1) ^ polynomial
            } else {
                crc << 1
            };
        }
    }
    crc
}

fn crc32_reflected(data: &[u8], polynomial: u32, initial: u32) -> u32 {
    let mut crc = initial;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ polynomial
            } else {
                crc >> 1
            };
        }
    }
    crc
}

fn crc16_reflected(data: &[u8], polynomial: u16, initial: u16) -> u16 {
    let mut crc = initial;
    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ polynomial
            } else {
                crc >> 1
            };
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::{RedundancyCrc, calculate};

    #[test]
    fn din_clause_6_3_6_known_answers() {
        let data = b"123456789";
        assert_eq!(calculate(RedundancyCrc::OptionB, data), 0x0e7c_650a);
        assert_eq!(calculate(RedundancyCrc::OptionC, data), 0xe306_9283);
        assert_eq!(calculate(RedundancyCrc::OptionD, data), 0x2189);
        assert_eq!(calculate(RedundancyCrc::OptionE, data), 0xbb3d);
    }
}

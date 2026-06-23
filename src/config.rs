//! DIN RaSTA 03.03 interoperability-test profile - non-production.
//!
//! Values in this module are test values only. A deployment must provide a
//! project-approved profile and safety case.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SafetyCodeLength {
    Md4Lower8,
    Md4Full16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RedundancyCrc {
    OptionB,
    OptionC,
    OptionD,
    OptionE,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileError {
    UnsupportedProtocolVersion,
    InvalidChannelCount,
    InvalidFlowControl,
    InvalidCapacity,
    InvalidTiming,
    InvalidPacketization,
    InvalidNetworkIdentifier,
    UnsafeMd4InitialValue,
}

#[derive(Clone, Copy, Debug)]
pub struct InteroperabilityProfile {
    pub protocol_version: [u8; 4],
    pub safety_code_length: SafetyCodeLength,
    pub redundancy_crc: RedundancyCrc,
    pub channel_count: u8,
    pub network_identifier: u32,
    pub md4_initial_value: [u8; 16],
    pub t_max_ms: u32,
    pub t_h_ms: u32,
    pub t_seq_ms: u32,
    pub n_send_max: usize,
    pub mwa: usize,
    pub defer_queue_capacity: usize,
    pub retransmission_capacity: usize,
    pub application_queue_capacity: usize,
    pub diagnostic_queue_capacity: usize,
    pub max_messages_per_packet: usize,
}

impl InteroperabilityProfile {
    pub const VERSION_03_03: [u8; 4] = *b"0303";
    pub const RFC_MD4_INITIAL_VALUE: [u8; 16] = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32,
        0x10,
    ];

    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.protocol_version != Self::VERSION_03_03 {
            return Err(ProfileError::UnsupportedProtocolVersion);
        }
        if self.channel_count != 2 {
            return Err(ProfileError::InvalidChannelCount);
        }
        if self.mwa == 0 || self.mwa >= self.n_send_max {
            return Err(ProfileError::InvalidFlowControl);
        }
        if self.retransmission_capacity < self.n_send_max || self.defer_queue_capacity != 4 {
            return Err(ProfileError::InvalidCapacity);
        }
        if self.t_h_ms == 0 || self.t_seq_ms == 0 || self.t_max_ms <= self.t_h_ms {
            return Err(ProfileError::InvalidTiming);
        }
        if self.max_messages_per_packet != 1 {
            return Err(ProfileError::InvalidPacketization);
        }
        if self.network_identifier == 0 {
            return Err(ProfileError::InvalidNetworkIdentifier);
        }
        if self.md4_initial_value == Self::RFC_MD4_INITIAL_VALUE
            || self.md4_initial_value == [0; 16]
        {
            return Err(ProfileError::UnsafeMd4InitialValue);
        }
        Ok(())
    }
}

pub const DIN_RASTA_03_03_INTEROPERABILITY_TEST_PROFILE: InteroperabilityProfile =
    InteroperabilityProfile {
        protocol_version: InteroperabilityProfile::VERSION_03_03,
        safety_code_length: SafetyCodeLength::Md4Lower8,
        redundancy_crc: RedundancyCrc::OptionB,
        channel_count: 2,
        network_identifier: 0x0000_0001,
        // A=0x67452302, B=0xEFCDAB98, C=0x98BADCFF, D=0x10325477, little-endian words.
        md4_initial_value: [
            0x02, 0x23, 0x45, 0x67, 0x98, 0xab, 0xcd, 0xef, 0xff, 0xdc, 0xba, 0x98, 0x77, 0x54,
            0x32, 0x10,
        ],
        t_max_ms: 1_800,
        t_h_ms: 300,
        t_seq_ms: 100,
        n_send_max: 20,
        mwa: 10,
        defer_queue_capacity: 4,
        retransmission_capacity: 20,
        application_queue_capacity: 20,
        diagnostic_queue_capacity: 16,
        max_messages_per_packet: 1,
    };

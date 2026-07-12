//! Platform-independent RaSTA configuration types and profile validation.

use core::fmt;

use crate::connection::safety_code::SafetyCodeConfig;
use crate::redundancy::{RedundancyConfig, RedundancyCrc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SafetyCodeLength {
    None,
    Md4Lower8,
    Md4Full16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimestampCompatibilityMode {
    StrictSynchronized,
    PeerRelative,
}

#[derive(Clone, Copy)]
pub struct RastaConfig {
    pub sender_id: u32,
    pub remote_id: u32,
    pub safety_code: SafetyCodeConfig,
    pub redundancy: RedundancyConfig,
    pub t_max: u32,
    pub initial_seq: u32,
    pub heartbeat_interval_ms: u32,
    pub n_send_max: u16,
    pub mwa: u16,
    pub allow_unsafe_no_checksums: bool,
    pub timestamp_compatibility: TimestampCompatibilityMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigError {
    UnsupportedProtocolVersion,
    InvalidChannelCount,
    InvalidFlowControl,
    InvalidCapacity,
    InvalidTiming,
    InvalidPacketization,
    InvalidNetworkIdentifier,
    UnsafeMd4InitialValue,
    UnsafeNoChecksumRequiresOptIn,
}

pub type ProfileError = ConfigError;
pub type InteroperabilityProfile = RastaProfile;

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedProtocolVersion => f.write_str("unsupported protocol version"),
            Self::InvalidChannelCount => f.write_str("invalid channel count"),
            Self::InvalidFlowControl => f.write_str("invalid flow control"),
            Self::InvalidCapacity => f.write_str("invalid capacity"),
            Self::InvalidTiming => f.write_str("invalid timing"),
            Self::InvalidPacketization => f.write_str("invalid packetization"),
            Self::InvalidNetworkIdentifier => f.write_str("invalid network identifier"),
            Self::UnsafeMd4InitialValue => f.write_str("unsafe md4 initial value"),
            Self::UnsafeNoChecksumRequiresOptIn => {
                f.write_str("unsafe no-checksum profile requires explicit opt-in")
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RastaProfile {
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
    pub timestamp_compatibility: TimestampCompatibilityMode,
}

impl RastaProfile {
    pub const VERSION_03_03: [u8; 4] = *b"0303";
    pub const RFC_MD4_INITIAL_VALUE: [u8; 16] = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32,
        0x10,
    ];

    pub const ACADEMIC_DEFAULT: Self = Self {
        protocol_version: Self::VERSION_03_03,
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
        timestamp_compatibility: TimestampCompatibilityMode::StrictSynchronized,
    };

    pub const LIBRASTA_LOCAL: Self = Self {
        protocol_version: Self::VERSION_03_03,
        safety_code_length: SafetyCodeLength::None,
        redundancy_crc: RedundancyCrc::OptionA,
        channel_count: 2,
        network_identifier: 1234,
        md4_initial_value: Self::RFC_MD4_INITIAL_VALUE,
        t_max_ms: 10_000,
        t_h_ms: 2_000,
        t_seq_ms: 50,
        n_send_max: 20,
        mwa: 10,
        defer_queue_capacity: 4,
        retransmission_capacity: 20,
        application_queue_capacity: 20,
        diagnostic_queue_capacity: 16,
        max_messages_per_packet: 1,
        timestamp_compatibility: TimestampCompatibilityMode::PeerRelative,
    };

    pub const SBB_LOCAL: Self = Self {
        protocol_version: Self::VERSION_03_03,
        safety_code_length: SafetyCodeLength::Md4Lower8,
        redundancy_crc: RedundancyCrc::OptionA,
        channel_count: 2,
        network_identifier: 123_456,
        md4_initial_value: Self::RFC_MD4_INITIAL_VALUE,
        t_max_ms: 750,
        t_h_ms: 300,
        t_seq_ms: 50,
        n_send_max: 20,
        mwa: 10,
        defer_queue_capacity: 4,
        retransmission_capacity: 20,
        application_queue_capacity: 20,
        diagnostic_queue_capacity: 16,
        max_messages_per_packet: 1,
        timestamp_compatibility: TimestampCompatibilityMode::PeerRelative,
    };

    pub fn academic_default() -> Result<Self, ConfigError> {
        let profile = Self::ACADEMIC_DEFAULT;
        profile.validate()?;
        Ok(profile)
    }

    pub fn librasta_local() -> Result<Self, ConfigError> {
        let profile = Self::LIBRASTA_LOCAL;
        profile.validate_allowing_unsafe_no_checksums()?;
        Ok(profile)
    }

    pub fn sbb_local() -> Result<Self, ConfigError> {
        let profile = Self::SBB_LOCAL;
        profile.validate_allowing_unsafe_no_checksums()?;
        Ok(profile)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        self.validate_common()?;
        self.validate_safe_checksums()
    }

    pub fn validate_allowing_unsafe_no_checksums(&self) -> Result<(), ConfigError> {
        self.validate_common()
    }

    fn validate_common(&self) -> Result<(), ConfigError> {
        if self.protocol_version != Self::VERSION_03_03 {
            return Err(ConfigError::UnsupportedProtocolVersion);
        }
        if self.channel_count != 2 {
            return Err(ConfigError::InvalidChannelCount);
        }
        if self.mwa == 0 || self.mwa >= self.n_send_max {
            return Err(ConfigError::InvalidFlowControl);
        }
        if self.retransmission_capacity < self.n_send_max || self.defer_queue_capacity != 4 {
            return Err(ConfigError::InvalidCapacity);
        }
        if self.t_h_ms == 0 || self.t_seq_ms == 0 || self.t_max_ms <= self.t_h_ms {
            return Err(ConfigError::InvalidTiming);
        }
        if self.max_messages_per_packet != 1 {
            return Err(ConfigError::InvalidPacketization);
        }
        if self.network_identifier == 0 {
            return Err(ConfigError::InvalidNetworkIdentifier);
        }
        if self.safety_code_length != SafetyCodeLength::None
            && (self.md4_initial_value == Self::RFC_MD4_INITIAL_VALUE
                || self.md4_initial_value == [0; 16])
            && *self != Self::SBB_LOCAL
        {
            return Err(ConfigError::UnsafeMd4InitialValue);
        }
        Ok(())
    }

    fn validate_safe_checksums(&self) -> Result<(), ConfigError> {
        if self.safety_code_length == SafetyCodeLength::None
            || self.redundancy_crc == RedundancyCrc::OptionA
        {
            return Err(ConfigError::UnsafeNoChecksumRequiresOptIn);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RastaProfileBuilder {
    profile: RastaProfile,
    allow_unsafe_no_checksums: bool,
}

impl RastaProfileBuilder {
    pub fn new() -> Self {
        Self {
            profile: RastaProfile::ACADEMIC_DEFAULT,
            allow_unsafe_no_checksums: false,
        }
    }

    pub fn from_profile(profile: RastaProfile) -> Self {
        Self {
            profile,
            allow_unsafe_no_checksums: false,
        }
    }

    pub fn allow_unsafe_no_checksums(mut self, allow: bool) -> Self {
        self.allow_unsafe_no_checksums = allow;
        self
    }

    pub fn safety_code_length(mut self, value: SafetyCodeLength) -> Self {
        self.profile.safety_code_length = value;
        self
    }

    pub fn redundancy_crc(mut self, value: RedundancyCrc) -> Self {
        self.profile.redundancy_crc = value;
        self
    }

    pub fn network_identifier(mut self, value: u32) -> Self {
        self.profile.network_identifier = value;
        self
    }

    pub fn md4_initial_value(mut self, value: [u8; 16]) -> Self {
        self.profile.md4_initial_value = value;
        self
    }

    pub fn timing(mut self, t_max_ms: u32, t_h_ms: u32, t_seq_ms: u32) -> Self {
        self.profile.t_max_ms = t_max_ms;
        self.profile.t_h_ms = t_h_ms;
        self.profile.t_seq_ms = t_seq_ms;
        self
    }

    pub fn flow_control(mut self, n_send_max: usize, mwa: usize) -> Self {
        self.profile.n_send_max = n_send_max;
        self.profile.mwa = mwa;
        self.profile.retransmission_capacity = n_send_max;
        self
    }

    pub fn timestamp_compatibility(mut self, value: TimestampCompatibilityMode) -> Self {
        self.profile.timestamp_compatibility = value;
        self
    }

    pub fn build(self) -> Result<RastaProfile, ConfigError> {
        if self.allow_unsafe_no_checksums {
            self.profile.validate_allowing_unsafe_no_checksums()?;
        } else {
            self.profile.validate()?;
        }
        Ok(self.profile)
    }
}

impl Default for RastaProfileBuilder {
    fn default() -> Self {
        Self::new()
    }
}

//! Academic interoperability-test profile — non-production.
//!
//! These values are for the runnable demonstration only. They are not approved
//! operational railway parameters.

use rasta_core::config::{InteroperabilityProfile, SafetyCodeLength};
use rasta_core::redundancy::RedundancyCrc;

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

#[cfg(test)]
mod tests {
    use super::DIN_RASTA_03_03_INTEROPERABILITY_TEST_PROFILE;
    use rasta_core::config::{InteroperabilityProfile, ProfileError};

    #[test]
    fn academic_profile_is_valid_and_values_are_unchanged() {
        let profile = DIN_RASTA_03_03_INTEROPERABILITY_TEST_PROFILE;
        assert_eq!(profile.protocol_version, *b"0303");
        assert_eq!(profile.network_identifier, 0x0000_0001);
        assert_eq!(profile.t_max_ms, 1_800);
        assert_eq!(profile.t_h_ms, 300);
        assert_eq!(profile.t_seq_ms, 100);
        assert_eq!(profile.n_send_max, 20);
        assert_eq!(profile.mwa, 10);
        assert!(profile.validate().is_ok());

        let mut invalid = profile;
        invalid.md4_initial_value = InteroperabilityProfile::RFC_MD4_INITIAL_VALUE;
        assert_eq!(invalid.validate(), Err(ProfileError::UnsafeMd4InitialValue));
    }
}

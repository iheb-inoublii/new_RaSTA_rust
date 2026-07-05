//! Application aliases for library-defined test profiles.
//!
//! These profiles are for runnable demonstrations and interoperability testing.
//! They are not approved operational railway parameters.

use rasta_core::config::RastaProfile;

pub const DIN_RASTA_03_03_INTEROPERABILITY_TEST_PROFILE: RastaProfile =
    RastaProfile::ACADEMIC_DEFAULT;
pub const LIBRASTA_LOCAL_PROFILE: RastaProfile = RastaProfile::LIBRASTA_LOCAL;

#[cfg(test)]
mod tests {
    use super::{DIN_RASTA_03_03_INTEROPERABILITY_TEST_PROFILE, LIBRASTA_LOCAL_PROFILE};
    use rasta_core::config::{
        ConfigError, InteroperabilityProfile, SafetyCodeLength, TimestampCompatibilityMode,
    };

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
        assert_eq!(
            profile.timestamp_compatibility,
            TimestampCompatibilityMode::StrictSynchronized
        );
        assert!(profile.validate().is_ok());

        let mut invalid = profile;
        invalid.md4_initial_value = InteroperabilityProfile::RFC_MD4_INITIAL_VALUE;
        assert_eq!(invalid.validate(), Err(ConfigError::UnsafeMd4InitialValue));
    }

    #[test]
    fn librasta_local_profile_matches_known_c_baseline() {
        let profile = LIBRASTA_LOCAL_PROFILE;
        assert_eq!(profile.protocol_version, *b"0303");
        assert_eq!(profile.network_identifier, 1234);
        assert_eq!(profile.safety_code_length, SafetyCodeLength::None);
        assert_eq!(
            profile.redundancy_crc,
            rasta_core::redundancy::RedundancyCrc::OptionA
        );
        assert_eq!(profile.t_max_ms, 10_000);
        assert_eq!(profile.t_h_ms, 2_000);
        assert_eq!(
            profile.timestamp_compatibility,
            TimestampCompatibilityMode::PeerRelative
        );
        assert!(profile.validate_allowing_unsafe_no_checksums().is_ok());
    }
}

# DIN RaSTA 03.03 traceability notes — non-production

This is an implementation-status aid for an academic interoperability-test
profile. It is not a compliance matrix, conformance claim, or safety case.
Clause references identify engineering areas only and do not reproduce DIN text.

## Status vocabulary

| Status | Meaning |
|---|---|
| Implemented and tested | Code exists and has directly relevant automated tests. |
| Implemented but incompletely tested | Code exists; branch/event coverage is incomplete. |
| Partially implemented | Some required behavior exists, but the implementation is incomplete. |
| Not implemented | No active implementation exists. |
| Project-configurable | Values must come from a project profile/safety case. |
| External interoperability pending | Local tests pass; independent-peer evidence is not yet available. |

## Traceability summary

| Area / requirement theme | Status | Implementation module / function | Test evidence |
|---|---|---|---|
| Academic profile validation and parameter typing | Implemented and tested; project-configurable for production | `rasta_core::config::{InteroperabilityProfile, RastaConfig}`, `apps/rasta-node/src/profile.rs` | `interoperability_profile_validation_reports_each_typed_error`, `academic_profile_is_valid_and_values_are_unchanged` |
| Fixed-size bounded storage | Implemented and tested | `queue::FixedQueue`, `connection::RastaConnection` buffers, redundancy defer queues | `queue::tests::preserves_order_and_reports_overflow`, `application_tx_queue_is_bounded_when_flow_control_blocks`, defer-queue tests |
| PDU wire layout, type values, payload rules | Implemented and tested | `connection::pdu::{Packet, PacketType}` | `test_packet_serialization`, `pdu_message_types_lengths_and_payload_rules_are_enforced`, `pdu_connection_version_and_max_payload_boundaries_are_enforced`, `pdu_parser_does_not_panic_on_malformed_input` |
| Safety code using MD4 and configured IV | Implemented and tested | `connection::safety_code::{Md4, SafetyCodeConfig}` | `test_md4_known_vectors`, `md4_safety_code_matches_din_annex_a_lower_half`, checksum rejection tests |
| Redundancy CRC options and byte order | Implemented and tested | `redundancy::crc`, `RedundancyCrc` | `din_clause_6_3_6_known_answers_and_lengths`, `writes_check_codes_in_little_endian_wire_order` |
| Redundant two-channel send/receive, duplicate suppression, and channel monitoring | Implemented and tested for current local behavior; adaptive policy remains conservative | `redundancy::channel::{RedundancyLayer,ChannelId,ChannelStatus}`, connection channel diagnostics | duplicate/channel status tests, one-channel send/receive failure tests, timeout/recovery tests, CRC/malformed-one-channel tests, two-endpoint test |
| Redundancy defer queue and `T_seq` expiry | Partially implemented; tested | `redundancy::{channel,defer_queue,sequence}` | defer queue tests, `defers_ahead_frame_then_releases_it_on_t_seq_expiry`, `releases_a_deferred_frame_after_the_missing_frame_arrives` |
| SRL six-state vocabulary and implemented transitions | Implemented and tested for current transition set | `connection::state_machine::{StateMachine, RastaState}` | `state_machine_all_implemented_transitions_and_rejections` |
| Complete DIN event/state matrix | Not implemented | N/A | Documented in `docs/state-event-test-matrix.md` as functional work required |
| Connection establishment | Partially implemented; tested locally | `connection::{connect, handle_packet}` | `test_connection_handshake_start`, `two_endpoint_two_channel_connection_and_data_interoperate` |
| Data transfer and packetization factor 1 | Partially implemented; tested | `connection::{send_application_data, receive_data}` | `two_endpoint_two_channel_connection_and_data_interoperate`, `test_application_receive_queue` |
| Flow control (`N_sendmax`, MWA) | Partially implemented; partially tested | `connection::{can_send_data, flush_application_tx}` | `application_tx_queue_is_bounded_when_flow_control_blocks`; full conformance pending |
| Sequence arithmetic and wraparound | Implemented and tested | `serial`, `connection::sequencing::SequenceHandler` | serial tests, `sequencing_duplicates_gaps_range_and_wraparound_are_classified` |
| Confirmed sequence handling | Implemented and tested for current local behavior | `connection::{validate_peer_confirmation,apply_valid_confirmation}`, `retransmission::clear_up_to`, `serial` | `confirmed_sequence_first_duplicate_single_cumulative_and_boundaries_release_exactly`, `confirmed_sequence_with_empty_retransmission_buffer_updates_ack_without_release`, `confirmed_sequence_initial_values_zero_one_max_and_before_max_are_not_sentinels`, `wraparound_confirmation_releases_only_confirmed_window_entries`, invalid confirmation tests |
| Retransmission request/response/data | Implemented and tested for deterministic sequence-gap recovery | `connection::{send_retransmission_request, send_retransmission_response, retransmit_from, handle_packet}`, `retransmission::RetransmissionBuffer` | `retransmission_request_uses_zero_payload_and_confirmed_sequence_point`, `sequence_gap_retransmission_recovers_lost_data_in_order`, `retransmit_from_validates_window_and_propagates_transport_failure` |
| Timestamp and heartbeat supervision | Implemented and tested for current local behavior | `time`, `connection::time_supervision`, `connection::heartbeat`, `connection::{validate_timeliness,apply_timeliness_decision}` | time tests, heartbeat tests, `time_supervision_preserves_exact_boundaries_and_wraparound`, `timestamp_validation_covers_future_boundary_and_half_range`, `confirmed_timestamp_validation_covers_progression_repeat_future_and_wrap`, `peer_silence_times_out_at_exact_t_max_and_sends_disconnect_once`, `valid_peer_heartbeat_restarts_deadline_but_sent_heartbeat_alone_does_not`, invalid timestamp tests, two-endpoint heartbeat loop |
| Diagnostics and SRL counters | Partially implemented; tested for implemented triggers | `srl::{DiagnosticEvent, SrlErrorCounters}`, `connection::record_diagnostic` | `bad_safety_code_is_rejected_and_counted_without_closing_connection`, `diagnostics_queue_overflow_is_counted_without_unrelated_counter_changes` |
| Concrete UDP platform adapter | Implemented and tested locally | `rasta_platform::udp::UdpSocketTransport` | UDP bind/empty receive/loopback tests; external interoperability pending |
| Standard clock adapter | Implemented and tested structurally | `rasta_platform::std_clock::StdClock` | standard-clock monotonic/protocol timestamp tests |
| Embedded Ethernet adapter trait | Implemented and tested with fake driver | `rasta_platform::embedded_ethernet` | fake-driver success and error propagation tests |
| Runnable node CLI and profile wiring | Implemented and tested for parsing/profile; local smoke exercised | `apps/rasta-node/src/main.rs`, `profile.rs` | CLI role/port tests, profile tests, prior local two-node smoke |
| Advanced per-channel quality/adaptive monitoring | Partially implemented | `redundancy::channel` | Minimal deterministic per-channel status, counters, timeout, and recovery are tested; statistical scoring/dynamic tuning remains project-specific |
| Independent implementation interoperability | External interoperability pending | N/A | Do not claim; future controlled test campaign required |

## Required project decisions

Operational network identifiers, MD4 initial values, endpoint addresses,
timing budgets, capacity values, acceptance criteria, hazard analysis, evidence
sets, and independent assessment remain project responsibilities. The shipped
node profile is explicitly academic and non-production.

## Verification recommendations

- Expand malformed retransmission-request and disconnect-path coverage.
- Independently review the DIN interpretation for which control PDU
  confirmation fields are acknowledgements versus request points.
- Add parser fuzzing in a separate phase if tooling is approved.
- Perform independent-peer interoperability only after deterministic local
  coverage and project requirements are stable.

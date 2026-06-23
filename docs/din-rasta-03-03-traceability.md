# DIN RaSTA 03.03 traceability notes — non-production

This is an implementation-status aid for an academic interoperability-test
profile. It is not a compliance matrix, a conformance claim, or a safety case.
Clause references identify the engineering area reviewed; they do not reproduce
the DIN text. Project-specific values and acceptance criteria require an
approved interface-control specification and safety case.

## Status vocabulary

| Status | Meaning |
|---|---|
| Implemented | Code exists and has a directly relevant automated test. |
| Partially implemented | Some behaviour exists, but coverage or required behaviour is incomplete. |
| Tested | Behaviour is covered by the named repository test; this is not conformance evidence. |
| Untested | Code exists but no directly relevant automated test was identified. |
| Planned | Identified work; not implemented. |
| Project-specific | A decision/value must be supplied by an approved project. |
| Not applicable | Not used by this test profile; it may still be relevant to another project. |

## Traceability summary

| Area | Status | Code location | Evidence / notes |
|---|---|---|---|
| Test-only profile, version `0303`, capacities, timings | Implemented; Project-specific for production | `config::InteroperabilityProfile`, `DIN_RASTA_03_03_INTEROPERABILITY_TEST_PROFILE` | `tests::din_interoperability_profile_is_valid_and_immutable_by_copy` validates the test profile only. |
| Fixed-size core buffers and queues | Implemented; Partially tested | `core::connection::RastaConnection`, `fixed_queue::FixedQueue` | `tests::fixed_queue_preserves_order_and_reports_overflow`; no formal resource-bound analysis. |
| Checked PDU read/write and control-PDU structure | Implemented; Tested | `core::pdu::Packet`, `packet_io::{PacketReader, PacketWriter}` | `tests::packet_reader_writer_are_checked`, `tests::din_control_pdus_enforce_exact_payload_rules`, `tests::pdu_parser_does_not_panic_on_malformed_input`. |
| MD4 lower-8 safety code using test IV | Implemented; Tested | `core::safety_code::SafetyCodeConfig` | `tests::md4_safety_code_matches_din_annex_a_lower_half`; test configuration only. |
| RL CRC options B/C/D/E | Implemented; Tested | `redundancy_crc::calculate` | `redundancy_crc::tests::din_clause_6_3_6_known_answers`. The executable selects option B. |
| RL two-channel duplicated send and duplicate suppression | Partially implemented; Tested | `core::redundancy_management::RedundancyLayer` | `tests::test_redundancy_discards_duplicate_channel_copy`, `tests::two_endpoint_two_channel_connection_and_data_interoperate`. No physical-channel quality model. |
| RL defer queue and `T_seq` | Partially implemented; Tested | `RedundancyLayer::{defer, deliver_expired_deferred}` | `tests::redundancy_defers_ahead_frame_until_missing_sequence_arrives`, `tests::redundancy_releases_deferred_frame_after_t_seq`. Capacity is fixed at four. |
| SRL connection establishment and `Up` transition | Partially implemented; Tested | `core::connection::{connect, handle_packet}` | `tests::test_connection_handshake_start`, `tests::two_endpoint_two_channel_connection_and_data_interoperate`. Independent-peer interoperability is untested. |
| Six-state SRL vocabulary | Partially implemented; Tested structurally | `core::connection_state_machine::{RastaState, StateMachine}` | `tests::test_state_machine_transitions`. Complete Table 18 event/state behaviour is planned. |
| Data transfer and one-message packetization | Partially implemented; Tested | `connection::{send_application_data, enqueue_application_data}` | `tests::two_endpoint_two_channel_connection_and_data_interoperate`, `tests::test_application_receive_queue`. |
| `N_sendmax`, MWA, bounded queues | Partially implemented; Partially tested | `connection::{can_send_data, flush_application_tx}` | `tests::application_tx_queue_is_bounded_when_flow_control_blocks`; no complete standards conformance suite. |
| Sequence arithmetic and wraparound helpers | Implemented; Tested | `serial`, `core::sequencing::SequenceHandler` | `tests::serial_number_arithmetic_handles_wraparound`, `tests::test_sequence_handler`. |
| Confirmed sequence number handling | Partially implemented; Untested at full matrix level | `connection::apply_confirmation` | Boundary and state/event coverage remains incomplete. |
| Timestamp/timeout supervision | Partially implemented; Partially tested | `connection::{apply_timeliness, start_timeliness_monitor}` | `tests::test_time_supervision` and extended heartbeat loop in `tests::two_endpoint_two_channel_connection_and_data_interoperate`; full timing diagnostics are planned. |
| Heartbeat and connection teardown | Partially implemented; Partially tested | `core::heartbeat::HeartbeatHandler`, `connection::{disconnect, send_disconnect}` | Local demo exercises these paths; no exhaustive automated event matrix. |
| Retransmission request/response/data | Partially implemented; Untested end-to-end | `connection::{retransmit_from, handle_packet}`, `core::retransmission::RetransmissionBuffer` | `tests::test_retransmission_buffer` only; end-to-end loss/recovery coverage is planned. |
| Diagnostics and SRL counters | Partially implemented; Untested for all conditions | `srl::{DiagnosticEvent, SrlErrorCounters}`, `connection::record_diagnostic` | `tests::bad_safety_code_is_rejected_and_counted_without_closing_connection` covers one case. |
| Per-channel quality and adaptive monitoring | Planned | — | No separate channel error/drop counters or standard-complete adaptive monitoring. |
| Desktop UDP adapter | Implemented; Manually exercised | `adapters::socket_transport::UdpSocketTransport`, `bin/rasta_node.rs` | Local loopback use is documented; Windows/Linux independent-peer interoperability remains untested. |

## Required project decisions

The following are deliberately not supplied by this repository: operational
network identifier, MD4 initial value, timing budget, capacity values, endpoint
configuration, threat/hazard analysis, acceptance criteria, evidence set, and
independent assessment. The constants in `src/config.rs` are test-only values.

## Planned verification

- Complete the event/state matrix with table-driven tests.
- Add end-to-end retransmission and confirmation boundary tests.
- Add per-channel diagnostics and tests for channel degradation.
- Test with an independently implemented peer and recorded, approved test
  vectors.
- Establish a project-specific requirements, verification, and safety-case
  baseline before considering operational use.

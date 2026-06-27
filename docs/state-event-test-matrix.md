# SRL state/event test matrix

This matrix describes currently implemented behavior only. It deliberately does
not invent DIN state/event rows that are not implemented.

## Structural state-machine transitions

Covered by `state_machine_all_implemented_transitions_and_rejections`.

| Current state | Input event / requested transition | Expected next state | Action / packet | Timers | Sequence effects | Diagnostic / counter | Coverage |
|---|---|---|---|---|---|---|---|
| `Closed` | `Down` | `Down` | none | none | none | none | Covered |
| `Closed` | self-transition | `Closed` | none | none | none | none | Covered |
| `Closed` | any other requested state | unchanged | none | none | none | none | Covered as rejected |
| `Down` | `Start` | `Start` | none in bare state machine | none | none | none | Covered |
| `Down` | `Closed` | `Closed` | none | none | none | none | Covered |
| `Down` | self-transition | `Down` | none | none | none | none | Covered |
| `Down` | any other requested state | unchanged | none | none | none | none | Covered as rejected |
| `Start` | `Up` | `Up` | none in bare state machine | none | none | none | Covered |
| `Start` | `Closed` | `Closed` | none | none | none | none | Covered |
| `Start` | self-transition | `Start` | none | none | none | none | Covered |
| `Start` | any other requested state | unchanged | none | none | none | none | Covered as rejected |
| `Up` | `RetransmissionRequested` | `RetransmissionRequested` | none in bare state machine | none | none | none | Covered |
| `Up` | `Closed` | `Closed` | none in bare state machine | none | none | none | Covered |
| `Up` | self-transition | `Up` | none | none | none | none | Covered |
| `Up` | any other requested state | unchanged | none | none | none | none | Covered as rejected |
| `RetransmissionRequested` | `RetransmissionRunning` | `RetransmissionRunning` | none in bare state machine | none | none | none | Covered |
| `RetransmissionRequested` | `Closed` | `Closed` | none | none | none | none | Covered |
| `RetransmissionRequested` | self-transition | `RetransmissionRequested` | none | none | none | none | Covered |
| `RetransmissionRequested` | any other requested state | unchanged | none | none | none | none | Covered as rejected |
| `RetransmissionRunning` | `RetransmissionRequested` | `RetransmissionRequested` | none | none | none | none | Covered |
| `RetransmissionRunning` | `Up` | `Up` | none | none | none | none | Covered |
| `RetransmissionRunning` | `Closed` | `Closed` | none | none | none | none | Covered |
| `RetransmissionRunning` | self-transition | `RetransmissionRunning` | none | none | none | none | Covered |
| `RetransmissionRunning` | any other requested state | unchanged | none | none | none | none | Covered as rejected |

## Connection-level implemented events

| Current state | Input event | Expected next state | Expected action / packet | Timers/deadlines | Sequence effects | Diagnostic / counter | Existing test | Coverage |
|---|---|---|---|---|---|---|---|---|
| `Closed` | `connect()` as lower-ID client | `Start` | sends `ConnectionRequest` | heartbeat and timeliness started | TX sequence advances | none | `test_connection_handshake_start`, `two_endpoint_two_channel_connection_and_data_interoperate` | Covered |
| `Closed` | `connect()` as higher-ID server | `Down` | waits for request | heartbeat started | none | none | `two_endpoint_two_channel_connection_and_data_interoperate` | Covered indirectly |
| `Down` | receive `ConnectionRequest` | `Start` | sends `ConnectionResponse` | timeliness started, heartbeat restarted | initial RX accepted | none | `two_endpoint_two_channel_connection_and_data_interoperate` | Covered indirectly |
| `Start` | client receives `ConnectionResponse` | `Up` | sends `Heartbeat` | heartbeat restarted | initial RX accepted, counters reset | none | `two_endpoint_two_channel_connection_and_data_interoperate` | Covered |
| `Start` | server receives `Heartbeat` | `Up` | none | heartbeat restarted | confirmation applied | counters reset | `two_endpoint_two_channel_connection_and_data_interoperate` | Covered |
| `Up` | send application data | `Up` | sends `Data` or queues payload | none | TX sequence advances, retransmission stores data | queue-full error possible | `two_endpoint_two_channel_connection_and_data_interoperate`, `application_tx_queue_is_bounded_when_flow_control_blocks` | Covered |
| `Up` | receive `Data` | `Up` | queues app data, heartbeat if MWA reached | heartbeat restarted | RX sequence advances | receive queue full possible | `test_application_receive_queue`, `two_endpoint_two_channel_connection_and_data_interoperate` | Covered |
| Any active state | one redundancy channel send/receive/validation failure while the other channel remains usable | unchanged | communication continues through remaining channel | channel status becomes `Degraded`; SRL timers unchanged by channel status alone | no duplicate app delivery; redundancy sequence continues | channel-supervision diagnostic on transition | `single_channel_send_failure_degrades_only_that_channel`, `receive_failure_on_one_channel_does_not_fail_remaining_channel`, `crc_failure_on_one_channel_still_accepts_valid_other_channel`, `single_redundancy_channel_send_failure_is_diagnostic_but_connection_continues` | Implemented and tested |
| Any active state | both redundancy sends fail or both receives fail | unchanged at redundancy layer; caller receives transport error | no false success | both channels become `Degraded` | no sequence advancement on failed receive | channel counters/status expose failure; connection action remains transport-error driven | `reports_send_failure_only_when_both_channels_fail`, `both_channel_receive_failures_report_failure` | Implemented and tested for current behavior |
| Redundancy channel | no valid receive before `t_seq_ms`, then valid frame arrives | channel status `Failed` then `Healthy` | no wire action | timeout and recovery tracked per channel using monotonic time | recovered duplicate is not delivered twice | transition counters updated | `channel_timeout_and_recovery_are_deterministic`, duplicate-channel tests | Implemented and tested |
| `Up` | receive valid peer confirmation on ACK-bearing PDU | unchanged or packet-specific state | releases retransmission entries cumulatively only after timestamp, confirmation, and RX sequence validation | heartbeat/timeliness refreshed only after validation | last peer confirmation advances or repeats; flow-control capacity may reopen | none | `confirmed_sequence_first_duplicate_single_cumulative_and_boundaries_release_exactly`, `invalid_confirmation_does_not_reopen_flow_control_but_valid_ack_does` | Implemented and tested |
| `Up` | receive invalid peer confirmation on ACK-bearing PDU | `Closed` | sends disconnection with existing protocol-sequence-error reason, returns `ProtocolViolation` | heartbeat and timeliness stopped; deadline not refreshed | RX sequence is not advanced; retransmission buffer and app delivery unchanged | confirmed-sequence diagnostic and counter | `invalid_confirmations_disconnect_without_releasing_or_delivering`, `confirmation_of_unsent_future_and_half_range_ambiguous_values_are_rejected` | Implemented and tested |
| `Up` | receive malformed safety code | `Up` | discard frame | none | none | safety counter and diagnostic increment; overflow counted | `bad_safety_code_is_rejected_and_counted_without_closing_connection`, `diagnostics_queue_overflow_is_counted_without_unrelated_counter_changes` | Covered |
| `Up` | heartbeat deadline reached | `Up` | sends `Heartbeat` | heartbeat restarted | TX sequence advances | none | `two_endpoint_two_channel_connection_and_data_interoperate` | Indirectly covered |
| `Up` | timeliness deadline reached | `Closed` | sends disconnection with incoming-message-timeout reason, returns `SafetyTimeout` | timeliness and heartbeat stopped | TX sequence may advance once for disconnect | connection-timeout diagnostic | `peer_silence_times_out_at_exact_t_max_and_sends_disconnect_once` | Implemented and tested |
| `Up` | receive too-old or too-far-future timestamp on time-supervised PDU | `Closed` | sends disconnection with incoming-message-timeout reason, returns `SafetyTimeout` | timeliness and heartbeat stopped; deadline not refreshed | RX sequence is not advanced | connection-timeout diagnostic | `invalid_remote_timestamp_rejects_packet_before_sequence_or_deadline_refresh`, timestamp classifier tests | Implemented and tested |
| `Up` | receive invalid confirmed timestamp on time-supervised PDU | `Closed` | sends disconnection with protocol-sequence-error reason, returns `SafetyTimeout` | timeliness and heartbeat stopped; deadline not refreshed | RX sequence is not advanced | confirmed-timestamp diagnostic | `invalid_confirmed_timestamp_rejects_packet_before_sequence_or_deadline_refresh`, confirmed timestamp classifier tests | Implemented and tested |
| `Up` | sequence gap | `RetransmissionRequested` | sends zero-payload `RetransmissionRequest` with request point in `confirmed_sequence` | heartbeat restarted | expected RX retained | sequence diagnostic and counter | `sequence_gap_retransmission_recovers_lost_data_in_order` | Implemented and tested |
| `Up` | receive valid `RetransmissionRequest` | `Up` | treats `confirmed_sequence` as the retransmission request point, sends `RetransmissionResponse`, retransmits retained packets as `RetransmissionData`, then sends `Heartbeat` | heartbeat state unchanged except normal outbound packets | retransmitted packets keep original SRL sequence numbers; request point does not release the sender's retransmission buffer; TX sequence is not advanced by retransmitted data | none | `sequence_gap_retransmission_recovers_lost_data_in_order`, `retransmission_request_point_is_not_processed_as_cumulative_ack`, `retransmit_from_validates_window_and_propagates_transport_failure` | Implemented and tested |
| `Up` | invalid retransmission request outside retained window | `Closed` via disconnect path | sends disconnection with existing retransmission-unavailable reason where possible | timeliness stopped | retained buffer unchanged | retransmission-failure diagnostic | `retransmit_from_validates_window_and_propagates_transport_failure` covers selection; connection-level invalid request remains partial | Partially implemented |
| `RetransmissionRequested` | receive `RetransmissionResponse` | `RetransmissionRunning` | none | heartbeat restarted after confirmation validation | expected RX is retained so missing retransmitted sequence can fill the gap; invalid peer confirmation closes before transition | confirmed-sequence diagnostic/counter on invalid confirmation | `sequence_gap_retransmission_recovers_lost_data_in_order`, `invalid_confirmation_in_retransmission_state_does_not_transition_or_release` | Implemented and tested |
| `RetransmissionRunning` | receive `RetransmissionData` | `RetransmissionRunning` until terminating heartbeat | queues app data after validation | heartbeat restarted | RX sequence advances using original packet sequence; valid peer confirmation releases retained local data | none | `sequence_gap_retransmission_recovers_lost_data_in_order`, `retransmission_data_confirmation_is_classified_and_releases_retained_packets` | Implemented and tested |
| `RetransmissionRunning` | receive heartbeat after retransmission range | `Up` | none | heartbeat restarted | confirms retransmission range complete under current behavior | none | `sequence_gap_retransmission_recovers_lost_data_in_order` | Implemented and tested |
| Any non-`Closed` | `disconnect()` | `Closed` | sends `DisconnectionRequest` | timeliness stopped | TX sequence may advance | none | local node smoke/manual | Indirectly covered |
| Any state | complete DIN Table 18 event matrix | N/A | N/A | N/A | N/A | N/A | N/A | Not implemented — functional work required |

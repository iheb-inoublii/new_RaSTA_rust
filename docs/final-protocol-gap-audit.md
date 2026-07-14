# Final RaSTA protocol gap audit — updated interoperability status

This audit is documentation-only. It does not claim DIN compliance,
certification readiness, full conformance, or independent assessment. It reviews the
current Rust workspace against the project’s stated RaSTA 03.03 intent and the
implementation evidence available in this repository.

The DIN VDE V 0831-200:2015-06 / RaSTA 03.03 document is treated as the
normative authority. Exact clause interpretations that are not directly
confirmed by repository evidence are marked `Unverified interpretation` rather
than inferred from memory.

## Executive summary

The workspace is compile/test ready. Controlled interoperability evidence now
exists against the SBB RaSTA stack through the local wrapper, including native
and Docker/Podman five-round Ping/Pong completion. The
protocol core is `no_std`, dependency-free, safe Rust, fixed-memory, and has
deterministic tests for framing, safety code, redundancy, sequencing,
retransmission, timeliness, confirmed sequence validation, and conservative
per-channel monitoring.

It is not certification-ready or ready for broad interoperability claims.
Several DIN table/event interpretations remain unverified, the controlled SBB
tests are not an independent assessment, and project-specific profile parameters
still need external review.

Readiness decision:

```text
Controlled SBB interoperability passed under recorded test conditions; broader
conformance and readiness are not established
```

## Critical blockers

No compile/test blockers were found. The following block broad interoperability
or certification claims:

- Exact DIN event/state table coverage is incomplete and not independently
  traced row-by-row.
- Control-PDU `confirmed_sequence_number` semantics are partly project
  interpreted, especially `RetransmissionRequest`.
- Per-channel monitoring is conservative and project-specific; `T_seq` is used
  as the academic observation window.
- No independent peer interoperability evidence exists.
- The academic profile values are explicitly non-production.
- Safety code is error detection / safety-code behavior, not cybersecurity
  authentication.

## Requirement-by-requirement matrix

Allowed status values used below:

- `Implemented and directly tested`
- `Implemented but indirectly tested`
- `Partially implemented`
- `Implemented with project-specific assumption`
- `Not implemented`
- `Unverified interpretation`
- `Controlled interoperability evidence`
- `Not applicable to selected profile`

| Requirement / clause | Expected behavior | Implementation module | Implementation symbol | Test name | Status | Risk | Required next action |
|---|---|---|---|---|---|---|---|
| Workspace separation | Protocol core is platform-independent; adapters and app live outside core | `rasta-core`, `rasta-platform`, `apps/rasta-node` | crate layout | workspace validation commands | Implemented and directly tested | Low | Preserve dependency direction during future changes. |
| Core platform independence | No production `std`, heap, OS sockets, synchronization, or unsafe in `rasta-core` | `crates/rasta-core/src/lib.rs`, port traits | `#![no_std]`, `Transport`, clocks | grep validation | Implemented and directly tested | Low | Keep grep in CI before interop branches. |
| Academic profile, not production | Profile values are selected for test only | `apps/rasta-node/src/profile.rs` | `ACADEMIC_PROFILE` | `academic_profile_is_valid_and_values_are_unchanged` | Implemented with project-specific assumption | High | Obtain project/safety-case profile before production claims. |
| Protocol version | Connection setup payload starts with ASCII `0303` | `connection/pdu.rs`, `connection/mod.rs` | `validate_payload_structure`, `connection_payload`, `apply_connection_payload` | `pdu_message_types_lengths_and_payload_rules_are_enforced`, profile test | Implemented and directly tested | Medium | Verify exact standard representation with independent peer. |
| SRL PDU type values | Numeric types are fixed as 6200, 6201, 6212, 6213, 6216, 6220, 6240, 6241 | `connection/pdu.rs` | `PacketType` | PDU tests | Implemented and directly tested | Medium | Cross-check values against DIN table before interop. |
| SRL header layout | 28-byte header: length, type, receiver, sender, seq, confirmed seq, timestamp, confirmed timestamp | `connection/pdu.rs` | `Packet::{parse,serialize}` | `test_packet_serialization` | Implemented and directly tested | Medium | Validate with peer trace capture. |
| SRL field byte order | SRL numeric fields are little-endian | `connection/pdu.rs` | `to_le_bytes`, `from_le_bytes` | `test_packet_serialization` | Implemented and directly tested | Medium | Confirm exact endian requirement from standard text. |
| SRL safety-code position | Safety code is appended after payload and covers PDU before safety code | `connection/pdu.rs`, `connection/safety_code.rs` | `Packet::serialize`, `Packet::parse`, `SafetyCodeConfig::calculate` | packet/safety-code tests | Implemented and directly tested | Medium | Validate with independent known-answer vector using selected IV. |
| RL header layout | 8-byte redundancy header: declared length, reserve, redundancy sequence | `redundancy/frame.rs` | `write_header`, `parse_header`, `payload_range` | frame tests | Implemented and directly tested | Medium | Verify declared-length inclusion rules against external implementation. |
| RL reserve field | Reserve is encoded as zero on send and accepted non-zero on receive | `redundancy/frame.rs`, `redundancy/channel.rs` | `write_header`, `parse_header` | `accepts_non_zero_reserve_and_discards_duplicate_channel_copy` | Implemented and directly tested | Low | Confirm reserve-field tolerance is acceptable for interop. |
| RL check-code position | Redundancy check code is appended after RL payload | `redundancy/channel.rs` | `write_check_code`, `check_code_matches` | `writes_check_codes_in_little_endian_wire_order` | Implemented and directly tested | Medium | Verify with peer traces. |
| Malformed/trailing bytes | Parser rejects trailing bytes and malformed declared lengths | `connection/pdu.rs`, `redundancy/frame.rs` | `Packet::parse`, `parse_header` | PDU malformed tests, frame tests | Implemented and directly tested | Low | Add fuzzing later if approved. |
| Maximum SRL payload | Payload storage is fixed at 256 bytes | `connection/pdu.rs` | `Packet::MAX_PAYLOAD_SIZE` | max payload tests | Implemented and directly tested | Medium | Confirm profile packetization factor and max size externally. |
| Packetization | Application data is one message per PDU with 2-byte app length prefix | `connection/mod.rs` | `send_application_data`, `enqueue_application_data` | application queue/data tests | Implemented with project-specific assumption | Medium | Verify app payload convention with peer; DIN packetization interpretation remains partial. |
| ConnectionRequest | Allowed in `Down`; payload length 14; version + `N_sendmax` + zeros | `connection/mod.rs`, `connection/pdu.rs` | `handle_packet`, `apply_connection_payload` | two-endpoint test, PDU tests | Implemented but indirectly tested | Medium | Add explicit duplicate/repeated setup tests before broad interop. |
| ConnectionResponse | Client `Start` -> `Up`; payload mirrors connection payload | `connection/mod.rs` | `handle_packet` | two-endpoint test | Implemented but indirectly tested | Medium | Verify all invalid setup cases from standard table. |
| Data | In `Up` and retransmission states; sequence checked, app data queued | `connection/mod.rs` | `send_application_data`, `enqueue_application_data`, `handle_packet` | `test_application_receive_queue`, two-endpoint test | Implemented and directly tested | Medium | Verify payload length prefix convention externally. |
| Heartbeat | Zero payload, used for ACK/timeliness and state completion | `connection/mod.rs`, `connection/heartbeat.rs` | `HeartbeatHandler`, `handle_packet` | heartbeat and timeliness tests | Implemented and directly tested | Medium | Verify behavior in every DIN state/event row. |
| RetransmissionRequest | Zero payload; request point encoded in `confirmed_sequence_number` | `connection/mod.rs` | `send_retransmission_request`, `handle_packet` | `retransmission_request_uses_zero_payload_and_confirmed_sequence_point`, recovery test | Implemented with project-specific assumption | High | Confirm that `confirmed_sequence_number` is not also cumulative ACK for this PDU. |
| RetransmissionResponse | Zero payload; moves `RetransmissionRequested` to `RetransmissionRunning` | `connection/mod.rs` | `send_retransmission_response`, `handle_packet` | recovery tests | Implemented but indirectly tested | Medium | Verify expected sequence-number semantics against standard. |
| RetransmissionData | Resent packet payload and original SRL sequence are preserved, packet type changed | `connection/mod.rs` | `send_retransmission_data`, `retransmit_from` | `sequence_gap_retransmission_recovers_lost_data_in_order`, confirmation-after-retransmission test | Unverified interpretation | High | Check standard text/peer behavior for whether original sequence and transformed type are exact. |
| DisconnectionRequest | Payload length 4, reason code at bytes 2..4 little-endian | `connection/mod.rs`, `connection/pdu.rs`, `srl.rs` | `send_disconnect`, `DisconnectReason` | disconnect reason tests, timeout test | Implemented and directly tested | Medium | Verify payload reserved bytes and reason mapping externally. |
| Six-state vocabulary | Closed, Down, Start, Up, RetransmissionRequested, RetransmissionRunning | `connection/state_machine.rs`, `srl.rs` | `RastaState`, `SrlState` | state-machine tests | Implemented and directly tested | Medium | Complete standard event table trace before compliance claims. |
| Complete state/event matrix | Every standard row should have defined action/state/timer behavior | `connection/mod.rs`, docs | `handle_packet`, `process` | state matrix docs | Partially implemented | High | Build clause-by-clause table from DIN text; do not infer. |
| Active/passive setup | Lower sender ID actively opens; higher ID waits | `connection/mod.rs` | `connect` | handshake/two-endpoint tests | Implemented and directly tested | Medium | Confirm role rule against selected interop scenario. |
| Random initial sequence | Optional random source initializes TX sequence | `connection/mod.rs` | `try_new_with_random` | `connection_uses_injected_random_initial_sequence` | Implemented and directly tested | Medium | Use approved random source for real integration. |
| ID validation | Receiver/sender IDs validated outside `Down`; setup accepts receiver 0 or local ID | `connection/mod.rs` | `handle_packet` | indirectly covered | Implemented but indirectly tested | Medium | Add direct negative ID tests before broad interop. |
| TX sequence | Wraparound-safe next TX sequence | `connection/sequencing.rs` | `SequenceHandler::next_tx` | sequence tests | Implemented and directly tested | Low | Preserve serial arithmetic. |
| RX expected sequence | Expected RX advances only on accepted sequence | `connection/sequencing.rs`, `connection/mod.rs` | `validate_rx`, `handle_packet` | sequencing/retransmission tests | Implemented and directly tested | Medium | Check DIN behavior for duplicates in all states. |
| Local confirmation sent to peer | Outbound PDUs carry last received SRL sequence or default zero | `connection/mod.rs` | `send_packet`, `send_packet_with_fields` | indirect tests | Implemented but indirectly tested | High | Initial default zero is an interop risk; verify expected sentinel/initial ACK value. |
| Peer cumulative confirmation | ACK-bearing PDUs validate peer confirmation before buffer release | `connection/mod.rs` | `validate_peer_confirmation`, `apply_valid_confirmation` | confirmed-sequence tests | Implemented and directly tested | Medium | Verify ACK-bearing PDU set with standard. |
| Retransmission request point | Uses `confirmed_sequence_number = missing_sequence - 1` | `connection/mod.rs` | `send_retransmission_request` | retransmission request tests | Implemented with project-specific assumption | High | Confirm special field meaning against DIN. |
| RL redundancy sequence | Separate RL sequence, expected/duplicate/ahead/violation classification | `redundancy/sequence.rs`, `redundancy/channel.rs` | `RedundancySequence` | redundancy sequence/channel tests | Implemented and directly tested | Medium | Validate violation window (`>40`) with standard/profile. |
| Protocol timestamps | Separate protocol timestamp source from monotonic deadlines; platform `StdClock` maps wire timestamps to a shared epoch sampled once at startup and advanced monotonically | `time.rs`, `connection/time_supervision.rs`, `rasta-platform/src/std_clock.rs` | `ProtocolTimestamp`, `TimeSupervisor`, `StdClock` | time/timeliness tests, unequal local-origin tests | Implemented with project-specific assumption | High | Confirm DIN timestamp origin and synchronization accuracy requirements. |
| Confirmed timestamps | Validated against previous confirmed timestamp and local time | `connection/time_supervision.rs`, `connection/mod.rs` | `validate_confirmed_timestamp`, `validate_timeliness` | confirmed timestamp tests | Implemented and directly tested | High | Confirm exact DIN confirmed-timestamp formula and future tolerance. |
| Gap detection | Sequence gap in `Up` requests retransmission and retains expected RX | `connection/mod.rs` | `handle_packet`, `send_retransmission_request` | recovery test | Implemented and directly tested | Medium | Add multi-gap edge cases with independent traces. |
| Retransmission timeout | Timeout behavior during retransmission-specific states | `connection/mod.rs`, timers | global `T_max` only | timeliness tests | Partially implemented | High | Determine whether DIN has retransmission-specific timeout/event rows. |
| Multiple missing packets | Recovery retransmits contiguous retained packets up to gap | `connection/mod.rs` | `retransmit_from` | recovery test | Partially implemented | Medium | Add explicit multi-consecutive missing packet matrix. |
| Duplicate retransmitted packets | Duplicate/stale logic should avoid duplicate app delivery | `connection/sequencing.rs`, `connection/mod.rs` | `validate_rx`, `enqueue_application_data` | indirect tests | Partially implemented | Medium | Add direct duplicate retransmission tests. |
| `T_h` heartbeat interval | Heartbeat due after configured interval | `connection/heartbeat.rs` | `HeartbeatHandler` | heartbeat tests | Implemented and directly tested | Medium | Verify timing tolerance with external peer. |
| `T_max` timeliness | Peer silence at exact deadline disconnects with incoming-message-timeout | `connection/mod.rs` | `process`, `disconnect_with_reason` | `peer_silence_times_out_at_exact_t_max_and_sends_disconnect_once` | Implemented and directly tested | Medium | Confirm exact-deadline behavior with standard. |
| Remote timestamp validation | Reject too-old/future timestamps on time-supervised PDUs | `connection/time_supervision.rs`, `connection/mod.rs` | `validate`, `validate_timeliness` | invalid timestamp tests | Implemented and directly tested | High | Future tolerance is project-specific until DIN interpretation confirmed. |
| Heartbeat in retransmission states | Heartbeat may terminate retransmission states to `Up` | `connection/mod.rs` | `handle_packet` | recovery test | Implemented but indirectly tested | High | Verify against complete standard event table. |
| MWA | Sends heartbeat/ACK after receiving `mwa` data messages | `connection/mod.rs` | `received_since_ack`, `mwa` branch | indirect coverage only | Partially implemented | High | Add direct end-to-end MWA threshold test and verify ACK PDU type. |
| `N_sendmax` | Blocks/suspends data when retransmission buffer reaches min local/remote limit | `connection/mod.rs` | `can_send_data`, `send_application_data`, `flush_application_tx` | flow-control tests | Implemented and directly tested | Medium | Verify remote negotiation and queue behavior with peer. |
| Flow-control failure | Behavior if app queue full or flow-control exceeded | `connection/mod.rs` | app TX queue, `BufferFull` | queue tests | Partially implemented | Medium | Map to standard diagnostics/disconnects if required. |
| User disconnect | Sends DisconnectionRequest reason 0, closes, stops timers | `connection/mod.rs`, `srl.rs` | `disconnect`, `DisconnectReason::UserRequest` | reason-code test, indirect smoke | Implemented but indirectly tested | Medium | Add direct frame payload test. |
| Protocol version error | Unsupported setup version is rejected | `connection/pdu.rs`, `connection/mod.rs` | `UnsupportedProtocolVersion`, `ProtocolVersionError` | PDU tests | Implemented but indirectly tested | Medium | Verify outgoing disconnect behavior for setup parse failures. |
| Sequence error | Sequence error diagnostic/counter and retransmission where applicable | `connection/mod.rs` | `SequenceError`, `send_retransmission_request` | sequence/retransmission tests | Implemented and directly tested | Medium | Complete non-`Up` state behavior matrix. |
| Confirmed sequence error | Diagnostic/counter, protocol sequence disconnect | `connection/mod.rs` | `ConfirmedSequenceError`, `disconnect_with_error` | invalid confirmation tests | Implemented and directly tested | Medium | Confirm exact wire reason for confirmed-sequence error. |
| Timestamp error | Incoming message timeout or protocol sequence error depending timestamp kind | `connection/mod.rs` | `handle_time_supervision_error` | timeliness tests | Implemented with project-specific assumption | High | Confirm disconnect reason split. |
| Retransmission unavailable | Reason code 7 on invalid request outside retained window | `connection/mod.rs`, `srl.rs` | `RetransmissionUnavailable` | retransmit validation test | Implemented and directly tested | Medium | Add wire payload assertion. |
| Channel failure | Channel diagnostics emitted; no wire disconnect code invented | `redundancy/channel.rs`, `connection/mod.rs` | `ChannelSupervisionFailure` | channel tests | Implemented with project-specific assumption | High | Decide if channel failure should affect SRL state for interop profile. |
| Malformed message | Malformed SRL parse increments diagnostic; many malformed frames are ignored/continued | `connection/mod.rs`, `pdu.rs` | `MalformedMessage`, parser errors | malformed tests | Implemented but indirectly tested | Medium | Map every parser error to DIN diagnostic/action. |
| Safety code MD4-low8 | Default selected safety code is lower 8 bytes of MD4 digest | `connection/safety_code.rs`, app profile | `SafetyCodeConfig::md4_low8` | MD4 tests, profile tests | Implemented and directly tested | Medium | Verify lower-half interpretation with standard; current code uses first 8 digest bytes. |
| Safety code MD4-full16 | Mode exists but selected profile uses low8 | `connection/safety_code.rs` | `SafetyCodeMode::Md4Full16` | indirect via code only | Not applicable to selected profile | Low | Add tests if profile changes. |
| Non-standard MD4 IV/network separation | Test profile uses non-standard IV; standard IV rejected by profile validation | `config.rs`, `apps/rasta-node/src/profile.rs` | `InteroperabilityProfile`, `ACADEMIC_PROFILE` | profile tests | Implemented with project-specific assumption | High | Replace with approved project value for real deployment. |
| Redundancy CRC options b/c/d/e | Algorithms and lengths implemented | `redundancy/crc.rs` | `RedundancyCrc`, `calculate` | CRC known-answer tests | Implemented and directly tested | Medium | Confirm test vectors are independent from implementation; option B selected. |
| Redundancy defer queue | Ahead frames deferred, released on missing arrival or `T_seq` expiry | `redundancy/defer_queue.rs`, `channel.rs` | `DeferQueue`, `receive_at` | defer tests | Implemented and directly tested | Medium | Verify queue capacity and expiry semantics with standard. |
| Duplicate copies | Duplicate valid copy refreshes channel health but delivers SRL payload once | `redundancy/channel.rs`, `sequence.rs` | `ReceiveSequence::StaleOrDuplicate` | duplicate/channel tests | Implemented and directly tested | Medium | Confirm health refresh rule is acceptable for selected interop. |
| Per-channel monitoring | Unknown/Healthy/Degraded/Failed fixed states and counters | `redundancy/channel.rs` | `ChannelStatus`, `ChannelCounters` | channel tests | Implemented with project-specific assumption | High | Consider making profile-configurable if external peer expects no monitoring diagnostics. |
| One-channel degraded operation | Remaining channel continues communication | `redundancy/channel.rs`, `connection/mod.rs` | `send_at`, `receive_at` | one-channel tests | Implemented and directly tested | Medium | Validate behavior against peer network fault tests. |
| Both-channel failure | Both send or both receive failures return transport error; no wire code invented | `redundancy/channel.rs` | `send_at`, `receive_at` | both-channel tests | Implemented with project-specific assumption | Medium | Decide SRL action on persistent dual-channel failure. |
| Diagnostics queue | Fixed 16-event queue, overflow counted | `queue.rs`, `connection/mod.rs` | `FixedQueue<DiagnosticEvent,16>` | diagnostics overflow test | Implemented and directly tested | Low | Add external observability API if needed. |
| Error counters | Fixed SRL counters for safety/address/type/sequence/confirmed sequence | `srl.rs`, `connection/mod.rs` | `SrlErrorCounters`, `record_diagnostic` | counter tests | Partially implemented | Medium | Address counter not currently wired; map all standard counters. |
| Channel counters | Fixed per-channel counters saturating on increment | `redundancy/channel.rs` | `ChannelCounters` | channel tests | Implemented but indirectly tested | Low | Add explicit saturation test if required. |
| SBB-stack interoperability through local wrapper | Controlled peer evidence required | `endpoint`, `sbb-local`, SBB wrapper | public endpoint and wrapper runtime | native and Docker/Podman five-round Ping/Pong evidence | Controlled interoperability evidence | Medium | Keep claims limited to the recorded configuration; add robustness/fault scenarios before broader claims. |

## Message-type audit

| Message type | Allowed state/action observed | Payload | Sequence semantics | Confirmed sequence semantics | Timestamp semantics | Status | Risk / note |
|---|---|---|---|---|---|---|---|
| `ConnectionRequest` | Accepted only in `Down`; starts timeliness and sends response | 14 bytes: `0303`, `N_sendmax`, 8 zeros | Initializes RX after acceptance | Not treated as cumulative ACK | Not time-supervised | Implemented but indirectly tested | Duplicate/repeated request behavior needs explicit interop review. |
| `ConnectionResponse` | Accepted by client in `Start`; transitions to `Up`, sends heartbeat | same connection payload | Initializes RX | Treated as ACK-bearing by current code | Not time-supervised | Implemented but indirectly tested | ACK semantics during setup require standard confirmation. |
| `Data` | Accepted in `Up`/retransmission states; app delivery after validation | app length prefix + bytes | Strict expected sequence | Cumulative ACK | Time-supervised | Implemented and directly tested | Packetization prefix is project convention. |
| `Heartbeat` | Completes server setup; ACK/timeliness traffic | zero | Strict expected sequence outside setup | Cumulative ACK | Time-supervised | Implemented and directly tested | Full event table behavior unverified. |
| `RetransmissionRequest` | In `Up`, validates retained request point and retransmits | zero | Strict expected sequence | Special request point, not cumulative ACK | Not time-supervised | Implemented with project-specific assumption | High interop risk if peer treats field differently. |
| `RetransmissionResponse` | Only in `RetransmissionRequested`; transitions to `RetransmissionRunning` | zero | Excluded from normal RX validation | Cumulative ACK-bearing by current code | Not time-supervised | Unverified interpretation | Verify sequence and ACK semantics. |
| `RetransmissionData` | App-delivered as retransmitted original data | original payload bytes | Uses original SRL sequence | Cumulative ACK | Time-supervised | Unverified interpretation | Verify packet type/sequence preservation against standard. |
| `DisconnectionRequest` | Closes connection from active states | 4 bytes, reason code at bytes 2..4 | Excluded from normal RX validation | Cumulative ACK-bearing by current code | Not time-supervised | Implemented but indirectly tested | Confirm ACK semantics and reserved bytes. |

## Six-state audit

The bare transition graph is implemented and directly tested. The complete DIN
state/event/action table is not fully implemented or traced. Current
connection-level behavior is summarized in `docs/state-event-test-matrix.md`.

High-risk state/event gaps:

- repeated or duplicate setup messages;
- invalid control PDUs in every state;
- retransmission-specific timeout rows;
- disconnection behavior for every malformed/sequence/timestamp cause;
- service/application notifications beyond simple data queues;
- full behavior for `RetransmissionResponse` and `RetransmissionData`.

## Sequencing and acknowledgement separation

The implementation keeps these concepts separate in code:

- TX sequence: `SequenceHandler::next_tx`;
- RX expected sequence: `SequenceHandler::validate_rx`, `expected_rx`;
- local confirmation sent to peer: `SequenceHandler::last_received_seq`;
- peer cumulative confirmation: `last_confirmed_by_peer`;
- retransmission request point: `RetransmissionRequest.confirmed_sequence_number`;
- RL redundancy sequence: `RedundancySequence`;
- protocol timestamps: `ProtocolTimestamp`;
- confirmed protocol timestamps: `confirmed_timestamp_reference`.

Remaining risks:

- default local confirmation before any received packet is `0` in many outbound
  paths, while confirmation validation uses `initial_tx_sequence - 1`; verify
  the expected initial wire value.
- half-range ambiguity is tested for serial helpers, but not all connection
  paths have direct boundary tests.
- exact ACK-bearing status of all control PDUs requires standard review.

## Fixed-memory audit

Major fixed buffers per `RastaConnection`:

- RX SRL buffer: 512 bytes;
- TX SRL buffer: 512 bytes;
- application RX buffer: `20 * 256 = 5120` bytes plus 20 `usize` lengths;
- application TX buffer: `20 * 256 = 5120` bytes plus 20 `usize` lengths;
- retransmission buffer: 20 optional `Packet` entries; each `Packet` stores
  fixed 256-byte payload plus header fields;
- diagnostics queue: 16 fixed `DiagnosticEvent` entries;
- redundancy defer queue: fixed in redundancy layer;
- redundancy layer frame scratch buffers are stack-local `[u8; 520]` during
  send/receive.

All loops are bounded by fixed queue lengths, channel count 2, retransmission
capacity, or the `process()` receive loop limit of 32. No production heap use or
`unsafe` was found.

Exact `size_of::<RastaConnection<...>>()` was not added because this audit phase
does not modify source code and the concrete test transports are private test
fixtures. A future temporary measurement harness can report exact sizes without
changing production code.

## Platform and application audit

- `rasta-core` owns protocol logic and only depends on port traits.
- `rasta-platform::udp::UdpSocketTransport` implements `Transport` using
  nonblocking UDP sockets; no channel-health policy exists in platform code.
- `rasta-platform::std_clock::StdClock` provides process-local monotonic
  scheduling instants and shared-epoch protocol timestamps. The epoch is sampled
  from system time at clock construction and then advanced with `Instant`, so
  later wall-clock adjustments do not move protocol timestamps backward.
- `apps/rasta-node` owns CLI, profile wiring, two UDP transports, and node
  lifecycle.
- The root `rasta_stack` compatibility facade has been removed; active code
  lives in `crates/rasta-core`, `crates/rasta-platform`, and `apps/rasta-node`.

## Diagnostics, counters, and disconnect inventory

Diagnostics:

- `SafetyCodeError`: checksum mismatch; safety counter increments.
- `RedundancyCheckError`: defined, no clear active trigger found.
- `SequenceError`: sequence gap/range; sequence counter increments.
- `ConfirmedSequenceError`: invalid peer confirmation; confirmed sequence
  counter increments.
- `ConfirmedTimestampError`: invalid confirmed timestamp.
- `ProtocolVersionError`: unsupported version parsing path.
- `MalformedMessage`: parser malformed type/length/payload.
- `UnexpectedMessage`: invalid packet for current state.
- `RetransmissionFailure`: unavailable retransmission request.
- `FlowControlEvent`: defined, no clear active trigger found.
- `DeferQueueOverflow`: defined, no clear active connection trigger found.
- `ChannelSupervisionFailure`: emitted on channel degraded/failed transition.
- `ConnectionTimeout`: peer silence or timestamp age.

Disconnect reasons:

- 0 user request;
- 2 unexpected message for state;
- 3 sequence error during connection establishment;
- 4 incoming message timeout;
- 5 service not allowed in state;
- 6 protocol version error;
- 7 retransmission unavailable;
- 8 protocol sequence error;
- unknown code round-trips.

Gaps:

- address counter is defined but not wired in observed paths;
- several diagnostic variants are present but not tied to direct tests;
- standard-required diagnostic mapping is not fully audited clause-by-clause.

## Test profile audit

| Profile item | Current value | Classification |
|---|---:|---|
| Protocol version | ASCII `0303` | standard-fixed / externally verify |
| Network identifier | `0x0000_0001` | test-only / project-specific |
| MD4 IV | `[02 23 45 67 98 ab cd ef ff dc ba 98 77 54 32 10]` | test-only / project-specific |
| Safety-code length | MD4 lower 8 bytes | profile-selected |
| CRC option | Option B | profile-selected |
| `T_max` | 1800 ms | test-only / project-specific |
| `T_h` | 300 ms | test-only / project-specific |
| `T_seq` | 100 ms | test-only / project-specific |
| `N_sendmax` | 20 | profile-selected / project-specific |
| MWA | 10 | profile-selected / project-specific |
| Defer queue capacity | 4 | test-only / project-specific |
| Retransmission capacity | 20 | test-only / project-specific |
| Application queue capacity | 20 | test-only / project-specific |
| Diagnostic queue capacity | 16 | test-only / project-specific |
| Packetization limit | 1 | profile-selected |
| Channel count | 2 | profile-selected |
| Port mapping | node A local 12000/12001 remote 12002/12003; node B inverted | test-only |

## Test evidence classification

The 95 tests include:

- independent known-answer tests: RFC MD4 vectors; CRC check values where values
  are embedded as constants;
- implementation-derived expected-result tests: many protocol behavior tests
  based on current code semantics;
- unit tests: PDU, MD4, CRC, serial arithmetic, redundancy frame/sequence/defer
  queue/channel, time, heartbeat;
- integration tests: two in-memory endpoint connection/data/retransmission
  tests;
- same-stack tests: all two-node Rust tests use this implementation on both
  sides;
- platform smoke tests: UDP bind/empty receive/loopback tests;
- controlled SBB interoperability tests: native handshake/heartbeat and
  five-round Ping/Pong passed; Docker/Podman reproduction passed.

Two Rust nodes using this same stack are not independent interoperability
evidence.

Additional tests recommended before making any broader interoperability claim:

1. Clause-by-clause state/event table tests from the DIN document.
2. Independent trace vectors for all PDU and RL frame layouts.
3. Peer-produced safety-code and redundancy-CRC vectors using the academic
   profile values.
4. Explicit MWA threshold and ACK PDU tests.
5. Duplicate/repeated connection-establishment tests.
6. Direct wire payload tests for each disconnect reason.
7. RetransmissionData behavior comparison with an independent implementation.
8. Negative tests for all invalid control PDUs in all states.
9. Long-running peer-silence/recovery/channel-failure controlled tests.

## Validation results

The following validation commands were run after creating this audit:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check -p rasta-core --no-default-features
cargo build -p rasta-node --release
cargo tree -p rasta-core
git diff --check
git grep -n "TODO\|FIXME\|unimplemented!\|todo!\|panic!" -- crates apps
git grep -n "Vec<\|String\|Box<\|unsafe\|std::\|rasta_platform" -- crates/rasta-core
```

Results are summarized in the final response for this audit turn.

## Ordered action plan

1. Build a DIN-derived state/event matrix with explicit clause references and
   compare every row against `handle_packet` and `process`.
2. Resolve high-risk interpretation questions:
   `RetransmissionRequest.confirmed_sequence_number`,
   `RetransmissionResponse` sequence/ACK semantics, `RetransmissionData`
   preservation/type behavior, initial confirmation wire value, timestamp
   formulas, and per-channel monitoring policy.
3. Generate or obtain independent wire vectors for SRL PDUs, RL frames, MD4
   safety code with profile IV, and CRC option B.
4. Add missing direct tests for MWA, disconnect payloads, repeated setup
   messages, malformed control PDUs, and retransmission edge cases.
5. Extend the completed controlled SBB campaign with packet capture, robustness,
   loss, retransmission, and fault scenarios where project requirements demand it.
6. Feed observed mismatches back into the requirement matrix before any broader
   testing.

## Final readiness decision

Compile/test readiness: established for the current workspace.

Same-stack functional readiness: established for the implemented local behavior
covered by the workspace test suite.

Controlled SBB interoperability: passed for the recorded native and
Docker/Podman handshake, heartbeat, and five-round Ping/Pong scenarios.

Broader interoperability readiness: not established beyond the recorded test
configuration.

Certification readiness: not established.

Final decision:

```text
Controlled SBB interoperability evidence exists; production, certification, and
full DIN conformance readiness are not established
```

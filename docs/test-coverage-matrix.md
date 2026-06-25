# Test coverage matrix

This matrix is a structural coverage audit for the current repository. It is
not a conformance claim and does not replace DIN RaSTA validation.

`cargo llvm-cov` was checked and is not installed in this environment, so this
audit maps tests to modules and requirements manually.

## Baseline

Baseline before this phase:

| Package | Tests |
|---|---:|
| `rasta-core` | 46 |
| `rasta-platform` | 3 |
| `rasta-node` | 1 |
| `rasta_stack` | 0 |
| Total | 50 |

## Module matrix

| Module | Important API | Direct tests | Integration / indirect tests | Success path | Failure path | Boundary / wraparound | Status | Missing tests / notes |
|---|---|---|---|---|---|---|---|---|
| `rasta-core/src/config.rs` | `RastaConfig`, `InteroperabilityProfile::validate` | `interoperability_profile_validation_is_value_based`, `interoperability_profile_validation_reports_each_typed_error`, node profile test | Node startup construction | Covered | Covered | N/A | Covered | Production profile selection is project-specific. |
| `rasta-core/src/io.rs` | `PacketReader`, `PacketWriter` | `io::tests::reader_and_writer_are_checked` | PDU tests | Covered | Covered | Partially covered | Partially covered | Multi-field malformed streams could be expanded. |
| `rasta-core/src/queue.rs` | `FixedQueue` | `queue::tests::preserves_order_and_reports_overflow` | Diagnostics queue tests | Covered | Covered | Capacity boundary covered | Covered | None for current behavior. |
| `rasta-core/src/serial.rs` | serial comparison helpers | serial unit tests | sequencing/redundancy tests | Covered | Covered | Covered | Covered | Half-range ambiguity documented by tests. |
| `rasta-core/src/time.rs` | typed durations/instants/timestamps | time unit tests | heartbeat/time supervision tests | Covered | Covered | Covered | Covered | None for current helpers. |
| `rasta-core/src/srl.rs` | disconnect reasons, diagnostics, counters | disconnect reason tests, diagnostics tests | connection tests | Covered | Partially covered | N/A | Partially covered | Not every diagnostic kind has a trigger path. |
| `rasta-core/src/service.rs` | `RastaService`, `ConnectionStatus` | none direct | two-endpoint connection test, node smoke/manual | Indirectly covered | Partially covered | N/A | Indirectly covered | Direct facade error-path tests remain useful. |
| `connection/mod.rs` | `RastaConnection` | many protocol tests | two-endpoint test | Covered | Covered for implemented confirmation/timeliness failures | Covered for sequence, confirmation, and timeliness wraparound | Partially covered | Sequence-gap retransmission recovery, confirmed-sequence validation, and timeliness timeout paths are tested; full DIN event matrix remains incomplete. |
| `connection/pdu.rs` | `Packet`, `PacketType`, parser/serializer | packet serialization, control PDU, malformed loop, PDU boundary tests | connection tests | Covered | Covered | Max payload covered | Covered | Future fuzzing recommended. |
| `connection/state_machine.rs` | `StateMachine`, `RastaState` | exhaustive implemented-transition test | connection tests | Covered | Covered | N/A | Covered for implemented transitions | DIN-complete event table is not implemented. |
| `connection/sequencing.rs` | `SequenceHandler` | sequence tests, wrap/duplicate/gap tests | connection tests | Covered | Covered | Covered | Covered for current behavior | Sentinel behavior after RX wrap is documented, not fixed. |
| `connection/retransmission.rs` | `RetransmissionBuffer` | buffer/capacity/confirmation/window tests | retransmission and flow-control acknowledgement tests | Covered | Covered | Covered | Covered for current behavior | Additional malformed request/disconnect combinations can be expanded. |
| `connection/safety_code.rs` | `Md4`, `SafetyCodeConfig` | MD4 vectors, DIN lower-half answer | PDU checksum tests | Covered | Covered | N/A | Covered | No alternate algorithm; MD4 behavior intentionally preserved. |
| `connection/heartbeat.rs` | `HeartbeatHandler` | heartbeat restart/stop/wrap test | connection heartbeat loop | Covered | Covered | Covered | Covered | None for current helper. |
| `connection/time_supervision.rs` | `TimeSupervisor` | exact boundary tests, timestamp classifier tests | connection timeliness tests | Covered | Covered | Covered | Covered for current behavior | Remote timestamp, confirmed timestamp, exact `T_max`, and wraparound paths are covered. |
| `redundancy/channel.rs` | `RedundancyLayer` | channel tests | two-endpoint test | Covered | Covered | Covered | Partially covered | Per-channel quality monitoring not implemented. |
| `redundancy/frame.rs` | frame encode/decode | frame tests | channel tests | Covered | Covered | Boundary malformed lengths covered | Covered | None for current frame format. |
| `redundancy/sequence.rs` | redundancy sequence classifier | sequence tests | channel tests | Covered | Covered | Covered | Covered | None for current behavior. |
| `redundancy/defer_queue.rs` | fixed defer queue | defer queue tests | channel tests | Covered | Covered | Covered | Covered | None for current behavior. |
| `redundancy/crc.rs` | CRC options | known-answer and empty-data tests | channel tests | Covered | Covered | N/A | Covered | Independent vector set could be expanded. |
| `rasta-platform/src/udp.rs` | `UdpSocketTransport` | bind failure, empty receive, loopback send/receive | node smoke/manual | Covered | Partially covered | Dynamic ports covered | Partially covered | Small receive-buffer OS behavior remains platform-dependent. |
| `rasta-platform/src/std_clock.rs` | `StdClock` | monotonic/protocol timestamp structural tests | node tests/manual | Covered | N/A | N/A | Covered | No scheduler-dependent sleeps used. |
| `rasta-platform/src/embedded_ethernet.rs` | adapter and driver trait | fake-driver tests | none | Covered | Covered | Buffer boundary covered | Covered | Real hardware integration is external. |
| `apps/rasta-node/src/profile.rs` | demo profile constant | profile value/validation test | node startup | Covered | Covered | N/A | Covered | Profile remains non-production. |
| `apps/rasta-node/src/main.rs` | CLI role/port mapping, main loop | CLI parse/role tests | bounded manual smoke | Partially covered | Covered for parse errors | Port inversion covered | Partially covered | Main loop is not unit-tested to avoid indefinite processes. |

## Modules covered only indirectly

- `rasta-core/src/service.rs`
- Several private `connection/mod.rs` paths such as confirmation and timeliness
  application are covered through deterministic connection tests, not exhaustive
  direct unit tests.
  Confirmation coverage includes first, duplicate, cumulative, empty-buffer,
  invalid, flow-control, retransmission, and wraparound cases.

## Modules with no direct production-unit coverage

No active production module is completely untested. Remaining gaps are branch
coverage gaps rather than total absence of coverage.

## Future recommendations

- Add more deterministic retransmission failure/disconnect scenarios.
- Add a table-driven SRL event/action test harness.
- Add parser fuzzing in a separate phase with explicit tooling approval.
- Add independent-peer interoperability testing only after local deterministic
  coverage and requirements status are stable.

# DIN RaSTA 03.03 interoperability-test profile - non-production

This repository is being rewritten against DIN VDE V 0831-200 (VDE V 0831-200):2015-06. The values in `InteroperabilityProfile` are test values only; they are not a safety case or a certified deployment profile.

## Replacement map

| Existing module | Status | Replacement responsibility |
|---|---|---|
| `core/connection.rs` | Incompatible | Six-state SRL event matrix, timers, flow control, diagnostics |
| `core/connection_state_machine.rs` | Partially replaced | Active six-state SRL vocabulary; complete Table 18 transition behavior remains pending |
| `core/retransmission.rs` | Incompatible | 20-entry encoded-SRL-packet retention and confirmation validation |
| `core/redundancy_management.rs` | Incompatible | RL state, two channels, defer queue, `T_seq`, channel diagnosis |
| `core/safety_code.rs` | Retain after validation | DIN MD4-8/MD4-16 test vectors and immutable profile IV |
| `core/pdu.rs` | Retain only as reference | Fixed reader/writer and type-specific DIN 03.03 PDU validation |
| `platform/transport.rs` | Retain | Platform-independent physical-channel abstraction |
| `platform/clock.rs` and `timer.rs` | Replace interface | Monotonic clock and deterministic timer polling |
| `adapters/embedded_ethernet.rs` | Retain | Physical transport adapter pattern |

## Compatibility requirements

| DIN clause | Requirement | Planned code/tests |
|---|---|---|
| 5.3, 5.4 | Exact SRL PDU fields and type-specific payload lengths | `packet/srl.rs`, malformed-PDU tests |
| 5.5.2-5.5.7 | Ordered receive checks, serial checks, confirmations, timestamps, timers | `srl/transition.rs`, diagnostics |
| 5.5.9-5.5.10 | Flow control and packetization | `srl/flow_control.rs`, profile validation |
| 5.6 | Six-state event-state matrix | state/event matrix tests |
| 6.3.6 | CRC options b/c/d/e | `safety/redundancy_crc.rs` known-answer tests |
| 6.6 | RL sequencing, defer queue and `T_seq` | `redundancy/defer_queue.rs` |
| 7-8 | Project/profile parameters | immutable `InteroperabilityProfile` |

## Incremental implementation status

- Complete: immutable test profile and validation; fixed queue; serial arithmetic; checked reader/writer; DIN CRC options b-e with published check values; typed SRL states, diagnostic events, disconnect reasons, and random-source PAL; control-PDU structure validation; DIN packetization factor one; bounded flow-control TX/RX queues; RL defer queue with `T_seq`; two-channel end-to-end in-memory connection/data test.
- Pending: complete Table 18 state/event behavior, per-channel quality diagnostics, all DIN timing diagnostics, and the full traceability test suite.

## Source-based assumptions

- The DIN document defines version bytes as ASCII `"0303"`; no integer wire field `0x0303` is used.
- The RaSTA network identifier is a project value with a one-to-one allocation to the MD4 initial value (4.5, 8.1). The DIN PDU and MD4 coverage clauses do not add it as an MD4 input field.
- DIN allows CRC options a-e. This profile selects b; the implementations of b-e must nevertheless match clause 6.3.6 exactly.
- The supplied document does not specify a safe production value for any test parameter. All profile values remain labelled non-production.

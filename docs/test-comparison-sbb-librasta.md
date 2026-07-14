# Test Comparison: SBB and librasta Style Coverage

This matrix records expected coverage areas from established RaSTA implementations and maps them to the current Rust test/spec foundation.

## SBB Baseline Status

The SBB RaSTA stack has a documented local baseline investigation in `docs/sbb-baseline-investigation.md`. The SBB repository configured and built successfully after installing `libgmock-dev`, and its CTest suite passed with `24/24` tests passing.

No ready UDP client/server demo executable was found in the original SBB build
output. A local SBB adapter/wrapper was therefore added to implement the
transport integration interfaces.

The controlled campaign subsequently passed native SBB-to-SBB five-round
Ping/Pong, native Rust-to-SBB handshake/heartbeat and five-round Ping/Pong, and
Docker/Podman reproduction. See the
[final interop summary](final-interop-summary.md). This evidence is limited to
the recorded configuration and is not a broad conformance claim.

| Test area | What librasta/SBB-style tests usually cover | Current Rust coverage if known | Gap | Planned Rust test/spec file |
| --- | --- | --- | --- | --- |
| Profile/config validation | Valid and invalid protocol parameters, timing, flow control, checksums | Core validation tests and profile builder tests | Add external profile evidence as it is found | `tests/specs/unit/profile-config-validation.md` |
| Packet encoding/decoding | Wire layout, PDU type fields, malformed input | Unit tests for parser, serializer, librasta captured frames | Expand fixture catalog | `tests/specs/unit/packet-encoding-decoding.md` |
| Sequence numbering | Initial sequence, accepted receive sequence, wrap behavior | Core sequence tests | Add cross-implementation captures | `tests/specs/unit/sequence-numbering.md` |
| Confirmed sequence handling | ACK advancement, stale/invalid confirmations | Core confirmation tests | Add librasta/SBB trace comparison | `tests/specs/unit/confirmed-sequence-handling.md` |
| Retransmission | Buffer capacity, resend request/running paths | Core retransmission tests | Longer scenario evidence | `tests/specs/unit/retransmission-buffer.md` |
| Time supervision | Timestamp validity, timeout, wraparound | Core time supervision tests and librasta timestamp compatibility | Add SBB timing baseline | `tests/specs/unit/time-supervision.md` |
| Heartbeat | Periodic heartbeat and timeout behavior | Core heartbeat tests plus 40-second live librasta result | Automate long-running heartbeat scenarios | `tests/specs/unit/heartbeat-handling.md` |
| Redundancy | Two-channel framing, CRC, duplicate/loss behavior | Core redundancy tests | Dockerized multi-channel scenarios | `tests/specs/unit/redundancy-layer.md` |
| Safety code / CRC | MD4 and redundancy CRC behavior | Core checksum tests | Add SBB fixture vectors | `tests/specs/unit/safety-code-checksum-crc.md` |
| State machine | Connection open, up, retransmission, close | Core state transition tests | Scenario-level documentation | `tests/specs/unit/state-machine-transitions.md` |
| Error handling | Invalid config, malformed packets, diagnostics | Core error/diagnostic tests | Map errors to external expectations | `tests/specs/unit/error-handling.md` |
| Transport behavior | Transport trait send/receive errors | Mock transport tests | Keep transport refactor out of this phase | `tests/specs/unit/transport-trait-behavior.md` |
| Rust-to-Rust | Handshake, data, ping-pong, disconnect | In-memory and app-level tests exist partially | Add ping-pong and use-case apps later | `tests/specs/integration/rust-to-rust-ping-pong.md` |
| Rust-to-librasta | Handshake, data, heartbeat, ping-pong | Working local profile and 40-second result documented | Automate in CI or Docker later | `tests/specs/interoperability/rust-to-librasta-40-second-heartbeat.md` |
| Rust-to-SBB profile | SBB profile matching, wire lengths, role/port mapping | `RastaProfile::sbb_local()`, CLI selection, and RedL length tests | Expand fixture catalog if needed | `tests/specs/interoperability/rust-to-sbb-preparation.md` |
| Rust-to-SBB live tests | Handshake, heartbeat, data, ping-pong | Native handshake/heartbeat and five-round Ping/Pong passed | Add loss/retransmission/fault scenarios before broader claims | `tests/specs/interoperability/rust-to-sbb-handshake.md` |
| Docker/Podman | Repeatable multi-process tests | Rust tests, wrapper build/tests, and live five-round Rust-to-SBB scenario passed | CI automation remains optional future work | `tests/specs/interoperability/docker-interop.md` |

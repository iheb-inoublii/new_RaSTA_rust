# Controlled interoperability harness

This directory contains the material and local wrapper used for controlled
interoperability testing against the SBB RaSTA stack.

Controlled interoperability evidence exists against the SBB RaSTA stack via the
local SBB wrapper, under the recorded test configuration only. Two instances of
this Rust implementation are not independent interoperability evidence.

## Peer implementation

| Field | Recorded value |
|---|---|
| Implementation name | SBB RaSTA stack via local wrapper |
| Repository/source location | External local checkout, mounted via `SBB_HOST_ROOT` for Docker/Podman |
| Language | C/C++ |
| Supported RaSTA version | Protocol version `0303` in the inspected/observed SBB wrapper configuration; not a broad conformance claim |
| Build instructions | See [sbb-wrapper/](sbb-wrapper/) and [Docker interop](../docs/docker-interop.md) |
| Transport mapping | POSIX UDP wrapper with two redundancy channels |
| Licence | External SBB repository; no licence is restated or inferred here |
| Configuration mechanism | Wrapper/profile configuration matching `sbb-local` |
| One/two channel support | Two channels in the captured tests |
| Safety-code options | SBB-compatible local test configuration; see the [final interop summary](../docs/final-interop-summary.md) |
| Redundancy CRC options | SBB-compatible local test configuration; see the [final interop summary](../docs/final-interop-summary.md) |

These entries describe the inspected peer and captured test setup. They do not
assert capabilities or conformance beyond that setup.

## Final status

- Native SBB-to-SBB Ping/Pong 5 rounds: passed.
- Native Rust-to-SBB handshake/heartbeat: passed.
- Native Rust-to-SBB Ping/Pong 5 rounds: passed.
- Docker/Podman Rust tests: passed.
- Docker/Podman SBB wrapper build/tests: passed.
- Docker/Podman Rust-to-SBB 5-round Ping/Pong: passed.

Commands, profiles, and the evidence scope are recorded in the
[final interop summary](../docs/final-interop-summary.md). Container setup and
reproduction instructions are in [Docker interop](../docs/docker-interop.md).

## Safety and scope

This is controlled test evidence only. It is not certification, production
readiness, an independent safety assessment, or proof of full DIN conformance.
Operational use requires project-specific configuration management,
verification, validation, a safety case, and independent assessment.

`ChannelSupervisionFailure` diagnostics can appear during SBB interoperability
runs, but they did not prevent successful five-round Rust-to-SBB Ping/Pong
completion in the captured native and Docker/Podman evidence.

The `academic`, `librasta-local`, and `sbb-local` profiles are test profiles.
Their identifiers, timing values, safety-code settings, CRC settings, and ports
must not be treated as approved operational railway parameters.

## Interoperability resources

- [SBB wrapper](sbb-wrapper/) — POSIX UDP, RedL/SafRetL bridge, and Ping/Pong runtime
- [Profile comparison](profile-comparison.md) — Rust/peer configuration comparison
- [Test plan](test-plan.md) — phased controlled test procedure
- [Packet capture](packet-capture.md) — Wireshark/tcpdump capture guidance
- [Final interop summary](../docs/final-interop-summary.md) — final native and container status
- [Docker interop](../docs/docker-interop.md) — Docker/Podman build and reproduction workflow

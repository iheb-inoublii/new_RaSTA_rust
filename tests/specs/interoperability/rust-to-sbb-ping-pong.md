# Rust-to-SBB Ping/Pong

## Objective

Verify repeated bidirectional application payload exchange between Rust active
and SBB passive endpoints.

## Test setup

Use the `sbb-local` profile, two UDP redundancy channels, and five paced rounds.
Rust uses local ports `7100/7101`; SBB uses local ports `7000/7001`.

## Expected result

Rust sends `Ping(1)` through `Ping(5)`, SBB returns the matching ordered Pong
messages, and both peers report success before clean disconnection.

## Actual result

**PASS.** Native and Docker/Podman runs completed all five rounds:

- Rust: `sent_pings=5 received_pongs=5 success=true`.
- SBB: `received_pings=5 sent_pongs=5 success=true`.

`ChannelSupervisionFailure` diagnostics can appear during the run but did not
prevent successful completion.

## Evidence

See the [completed result](../../../interop/results/sbb-rust-ping-pong-5-rounds.md)
and [Docker/Podman test record](docker-interop.md).

## Scope

Loss, retransmission, and fault-injection phases were not run as part of this
result. The pass is controlled test evidence only, not certification, production
readiness, or proof of full DIN conformance.

# Rust-to-SBB handshake

## Objective

Verify that a Rust active endpoint establishes a connection and exchanges
heartbeats with an SBB passive endpoint through the local wrapper.

## Test setup

Run Rust active and SBB passive with `--profile sbb-local`, network ID `123456`,
active sender `0x61`, passive sender `0x62`, `t_max = 750 ms`, `t_h = 300 ms`,
`t_seq = 50 ms`, lower MD4 safety, and RedL option A/no check code.

## Expected result

The peers reach `Up` and exchange heartbeat frames using the inspected profile
and observed RedL datagram lengths: ConnectionRequest `58`, Heartbeat `44`, and
Disconnect `48` bytes.

## Actual result

**PASS.** The native Rust-to-SBB handshake and heartbeat exchange completed
under the recorded controlled configuration. The connection remained available
for application Ping/Pong testing.

## Evidence

See the [completed five-round result](../../../interop/results/sbb-rust-ping-pong-5-rounds.md)
and [final interop summary](../../../docs/final-interop-summary.md).

## Scope

This is controlled test evidence only, not certification, production readiness,
an independent assessment, or proof of full DIN conformance.

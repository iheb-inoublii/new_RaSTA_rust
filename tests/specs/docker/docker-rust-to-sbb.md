# Docker/Podman Rust-to-SBB

## Objective

Reproduce the native five-round Rust-to-SBB Ping/Pong scenario in the controlled
container environment.

## Test setup

- Rust active container: `172.28.0.10`.
- SBB passive wrapper container: `172.28.0.20`.
- Profile: `sbb-local`.
- Two UDP redundancy channels.
- Five paced Ping/Pong rounds.
- External SBB checkout mounted through `SBB_HOST_ROOT`.

## Test steps

1. Run the Rust workspace test service.
2. Build and test the SBB wrapper service.
3. Start the live compose profile.
4. Confirm both peers report five successful rounds.

## Actual result

**PASS.** Docker/Podman Rust tests, SBB wrapper build/tests, and the live
five-round Rust-to-SBB scenario passed.

## Evidence

See [Docker interop](../../../docs/docker-interop.md), the
[completed result](../../../interop/results/sbb-rust-ping-pong-5-rounds.md), and
the detailed [container interoperability spec](../interoperability/docker-interop.md).

## Scope

The result reproduces the recorded controlled configuration. It does not
establish certification, production readiness, an independent assessment, or
full DIN conformance.

# Rust-to-SBB Interoperability Plan

This document records the original preparation plan and baseline values. The
planned campaign has since completed: native SBB-to-SBB five-round Ping/Pong,
native Rust-to-SBB handshake/heartbeat and five-round Ping/Pong, and the
Docker/Podman reproduction all passed. See the
[final interop summary](final-interop-summary.md) and
[completed result](../interop/results/sbb-rust-ping-pong-5-rounds.md).

Those results are controlled test evidence only, not certification, production
readiness, an independent assessment, or proof of full DIN conformance.

## SBB baseline values

| Field | Value |
| --- | --- |
| network ID | `123456` |
| active/client sender ID | `0x61` |
| active/client receiver ID | `0x62` |
| passive/server sender ID | `0x62` |
| passive/server receiver ID | `0x61` |
| `t_max` | `750 ms` |
| `t_h` | `300 ms` |
| `t_seq` | `50 ms` |
| safety code | lower MD4 |
| RedL check code | type A / no check code |
| RedL header | `8` bytes |
| SR header | `28` bytes |
| ConnectionRequest | `50` byte SR PDU, `58` byte RedL datagram |
| Heartbeat | `36` byte SR PDU, `44` byte RedL datagram |
| Disconnect | `40` byte SR PDU, `48` byte RedL datagram |

## Rust preparation

Rust now exposes `RastaProfile::sbb_local()` and the `rasta-node` CLI accepts `--profile sbb-local`. The profile is explicit opt-in because it uses SBB-compatible interoperability settings, including RedL option A/no check code.

Default `rasta-node` role mapping for `--profile sbb-local`:

| Rust role | Local ports | Remote ports | Sender ID | Receiver ID |
| --- | --- | --- | --- | --- |
| `A` active/client | `7100`, `7101` | `7000`, `7001` | `0x61` | `0x62` |
| `B` passive/server | `7000`, `7001` | `7100`, `7101` | `0x62` | `0x61` |

The first live test should run Rust active against SBB passive.

## Original planned live test

Start SBB passive:

```sh
./interop/sbb-wrapper/build/sbb-rasta-wrapper passive 127.0.0.1 --rounds 3 --trace --run-seconds 30
```

Start Rust active:

```sh
cargo run -p rasta-node -- A 127.0.0.1 --profile sbb-local --trace-wire --run-seconds 30
```

Expected evidence before claiming success:

- Rust sends ConnectionRequest datagrams of length `58`.
- SBB accepts the request and both sides reach `Up`.
- Heartbeat datagrams match the SBB-observed length `44`.
- Disconnect datagrams match the SBB-observed length `48`.
- No timeout, malformed-frame diagnostic, or SBB `rasys_FatalError` occurs.

## Original open points and final disposition

The live test subsequently verified timestamp compatibility and response
handling sufficiently for the recorded handshake/heartbeat and five-round
Ping/Pong scenario. Docker/Podman reproduction also passed. Broader sequence,
loss, retransmission, and fault-injection coverage remains outside this evidence
scope. `ChannelSupervisionFailure` diagnostics can appear during SBB runs but
did not prevent successful five-round completion.

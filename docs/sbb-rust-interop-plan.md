# Rust-to-SBB Interoperability Plan

Step 8H verified the SBB wrapper SBB-to-SBB baseline: passive reaches `Up`, receives heartbeat, exits cleanly after the smoke condition, and active reaches `Up` then closes cleanly. Step 8I prepares the Rust side only. It does not claim Rust-to-SBB interoperability.

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

## Planned live test

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

## Open points

The live test still needs to verify timestamp compatibility, sequence behavior, and exact SBB response handling. Docker and SBB ping-pong remain future work.

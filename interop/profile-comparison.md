# Profile comparison

This comparison records values inspected for the controlled local test setups.
It is not a broad statement about every configuration supported by the peer
implementations and does not establish conformance.

## `sbb-local` captured profile

| Parameter | Rust `sbb-local` value | Observed SBB wrapper value | Recorded result |
|---|---|---|---|
| RaSTA version | ASCII `0303` | `0303` in inspected wrapper configuration | Matched for captured test |
| active sender / receiver | `0x61` / `0x62` | `0x61` / `0x62` | Matched |
| passive sender / receiver | `0x62` / `0x61` | `0x62` / `0x61` | Matched |
| network identifier | `123456` | `123456` | Matched |
| safety code | Lower MD4 with SBB-observed RFC MD4 initial value | SBB-compatible lower MD4 test configuration | Matched for captured test |
| redundancy check code | Option A / no RedL check code | Option A / no RedL check code | Matched for captured test |
| redundancy channels | 2 | 2 | Matched |
| active ports | Local `7100/7101`, remote `7000/7001` | Remote peer of passive `7000/7001` | Matched |
| passive ports | Local `7000/7001`, remote `7100/7101` | Local `7000/7001`, remote `7100/7101` | Matched |
| `T_max` | `750 ms` | `750 ms` | Matched |
| `T_h` | `300 ms` | `300 ms` | Matched |
| `T_seq` | `50 ms` | `50 ms` | Matched |
| `N_sendmax` / MWA | `20` / `10` | Inspected local wrapper/profile configuration | Matched for captured test |
| timestamp mode | Peer-relative compatibility | Observed live exchange accepted | Passed captured handshake/heartbeat |

The native and Docker/Podman runs completed five Ping/Pong rounds with this
configuration. See the
[completed result](results/sbb-rust-ping-pong-5-rounds.md) and
[final interop summary](../docs/final-interop-summary.md). This controlled result
is not certification, production readiness, or proof of full DIN conformance.

## `librasta-local` opt-in profile

This profile is a local C librasta wire-compatibility preset:

```bash
cargo run -p rasta-node --release -- A 127.0.0.1 --profile librasta-local --trace-wire
```

It uses client ID `0x60`, server ID `0x61`, client ports `9998/9999`, server
ports `8888/8889`, RaSTA version `0303`, network ID `1234`, `T_h = 2000 ms`,
`T_max = 10000 ms`, SR checksum length NONE, and redundancy TYPE_A with zero
CRC bytes. These unsafe/no-checksum settings require explicit opt-in and are
test-only.

The librasta profile and evidence are separate from the final SBB campaign. No
result for one peer should be generalized to another implementation or profile.

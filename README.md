
## Project status

This is an academic/development Rust implementation. It provides a fixed-capacity, `no_std`-capable core and a
two-UDP-channel desktop demonstration. It is useful for learning, development,
and controlled interoperability experiments.

It is **not** production-ready, certified, certification-ready, fully
DIN-compliant, or proven interoperable with another RaSTA implementation.
Several requirements remain incomplete; see [Known limitations](#known-limitations)
and the [traceability notes](docs/din-rasta-03-03-traceability.md).

Production parameters must come from an approved project profile, an
interface-control specification, and a safety case. The code and its current
configuration have not been independently assessed or certified.

## Safety disclaimer

The executable configuration is a test-only interoperability profile. Do not
reuse its identifiers, timing values, MD4 initial value, or ports as operational
railway parameters. A deployment needs project-specific parameter management,
configuration control, verification, validation, and independent assessment.

## Architecture

```text
Railway signalling application
        │
RaSTA service API                 src/application/service_interface.rs
        │
SRL and redundancy logic          src/core/
        │
Platform traits                   src/platform/
        │
Desktop / embedded adapters       src/adapters/
```

The core uses fixed-size arrays and generic platform traits. The `std`-only
UDP demo is deliberately outside the `no_std` core.

## Build

The desktop demonstration requires Rust with the `std` feature:

```bash
cargo build --release --features std --bin rasta_node
```

The binary interface is:

```text
rasta_node <A|B> <remote_ip>
```

`A` uses local UDP ports 5000 and 6000 and sends to the peer's 5001 and 6001.
`B` uses local UDP ports 5001 and 6001 and sends to the peer's 5000 and 6000.

## Test

The following commands were run successfully against this repository:

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo check --no-default-features --lib
```

The current suite contains unit and in-memory two-channel tests. It is not a
substitute for conformance, robustness, hardware-in-the-loop, or independent
interoperability testing.

## Run two local nodes

Use two terminals. Start B first:

```bash
cargo run --release --features std --bin rasta_node -- B 127.0.0.1
```

Then start A:

```bash
cargo run --release --features std --bin rasta_node -- A 127.0.0.1
```

The demo prints the channel endpoints, reaches `Up`, sends `Hello from A`, and
then A disconnects after a short demonstration interval.

## Run Windows and Linux nodes

Build the same revision on both hosts. Identify the IP address that each host
actually uses on the VMware network; on multihomed hosts it may differ from a
Wi-Fi or host-only adapter address.

Start B on Kali/Linux first, replacing the placeholders with the peer's actual
source address:

```bash
cargo run --release --features std --bin rasta_node -- B <windows-source-ip>
```

Start A on Windows:

```powershell
cargo run --release --features std --bin rasta_node -- A <kali-source-ip>
```

Allow inbound UDP 5000 and 6000 on Windows, and inbound UDP 5001 and 6001 on
Linux/Kali. Do not run `nc` on either RaSTA port while the demo is running.

The UDP adapter connects each socket to the configured peer endpoint and will
discard datagrams from a different source address or port. To verify the actual
source address on Kali:

```bash
sudo tcpdump -n -i any 'udp and (port 5000 or port 5001 or port 6000 or port 6001)'
```

You can also inspect listeners with `ss -lunp` on Linux or `netstat -ano -p udp`
on Windows. Ping proves only IP routing; it does not prove both required UDP
channels or endpoint/source-address matching.

## Configuration

[`src/config.rs`](src/config.rs) defines the clearly labelled
`DIN_RASTA_03_03_INTEROPERABILITY_TEST_PROFILE`. It is not a production profile.
The current test configuration uses:

| Parameter | Test-only value |
|---|---:|
| Protocol version | ASCII `0303` |
| RaSTA network identifier | `1` |
| MD4 initial value | Non-standard test-only value (defined in source) |
| Safety code | Lower 8 bytes of MD4 |
| Redundancy check code | CRC option B |
| Channels | 2 |
| `T_max` / heartbeat / `T_seq` | 1800 ms / 300 ms / 100 ms |
| `N_sendmax` / MWA | 20 / 10 |
| Defer / retransmission / application / diagnostic capacity | 4 / 20 / 20 / 16 |
| Packetization factor | 1 message per packet |

The network identifier and MD4 value are deliberately non-production test
values. They are not secrets and must not be interpreted as approved operational
parameters.

## Known limitations

- The six-state vocabulary exists, but the complete DIN Table 18 event/state
  matrix is not implemented or exhaustively tested.
- Per-channel quality metrics, adaptive channel monitoring, and the full
  timing-diagnostic model are incomplete.
- Retransmission, confirmations, flow control, packetization, and timestamp
  handling have focused tests, but not a full standard conformance test suite.
- The test suite includes an in-memory two-channel peer test; it does not prove
  compatibility with another independent RaSTA implementation or platform.
- The core avoids dynamic allocation and has malformed-PDU no-panic testing,
  but this is not a formal whole-program no-panic proof.
- The desktop binary is an interoperability aid, not a production networking or
  deployment interface. Its fixed identifiers and example payload are test-only.

## Repository structure

```text
Cargo.toml                         crate configuration
README.md                          project status and safe-use guidance
docs/development-guide.md          learning/development guide
docs/din-rasta-03-03-traceability.md  implementation-status traceability
src/                               library and desktop demonstration
```


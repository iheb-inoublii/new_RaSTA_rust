# SBB/Rust five-round Ping/Pong result

## Result metadata

| Field | Recorded value |
|---|---|
| Status | **PASS** |
| Test date | Recorded during final project validation, July 2026 |
| Rust repository commit | `a321b7be58abe79ecfb303eaa8a1bdf7baea27c0` (`a321b7b docs: align final interop documentation`) |
| Peer implementation | SBB RaSTA stack via local wrapper |
| Peer revision | Not recorded; `SBB_HOST_ROOT` was not available during this documentation polish |
| Host OS / environment | Windows development host; native interop captured in Linux/Kali and container reproduction in Docker/Podman; exact versions not fully recorded |
| Packet capture | Not recorded |
| Profile | `sbb-local` |
| Evidence scope | Controlled native and Docker/Podman test configuration |
| Evidence logs | [Final interop summary](../../docs/final-interop-summary.md) and [Docker/Podman reproduction](../../docs/docker-interop.md) |

## Network topology

- Rust endpoint: active.
- SBB wrapper endpoint: passive.
- Transport: POSIX UDP with two redundancy channels.
- Rust local ports: `7100` and `7101`.
- SBB local ports: `7000` and `7001`.

## Commands

SBB passive:

```sh
./build/sbb-rasta-wrapper passive 127.0.0.1 \
  --rounds 5 --trace --run-seconds 30 \
  --channel0-local 7000 --channel0-remote 7100 \
  --channel1-local 7001 --channel1-remote 7101
```

Rust active:

```sh
cargo run -p ping-pong-node -- active 127.0.0.1 \
  --profile sbb-local \
  --rounds 5 \
  --trace-wire \
  --run-seconds 30 \
  --ping-delay-ms 300 \
  --channel-0-local-port 7100 \
  --channel-0-remote-port 7000 \
  --channel-1-local-port 7101 \
  --channel-1-remote-port 7001
```

## Phase results

| Phase | Status | Recorded scope |
|---|---|---|
| Build validation | PASS | Rust workspace and SBB wrapper build/tests passed |
| Network reachability | PASS | Both UDP redundancy-channel paths exchanged traffic |
| Redundancy-layer framing | PASS | The recorded SBB-compatible two-channel configuration exchanged frames |
| Connection establishment | PASS | Rust active and SBB passive reached `Up` |
| Data transfer | PASS | Five ordered Ping/Pong rounds completed |
| Heartbeat and idle operation | PASS | Handshake/heartbeat evidence passed |
| Clean disconnection | PASS | Recorded run completed successfully |
| Loss/retransmission/fault phases | NOT RUN / OUT OF SCOPE | No broad robustness or fault-injection claim |

## Evidence summary

- Rust reached `Up`.
- Rust sent `Ping(1)` through `Ping(5)`.
- Rust received `Pong(1)` through `Pong(5)`.
- SBB received `Ping(1)` through `Ping(5)`.
- SBB sent `Pong(1)` through `Pong(5)`.
- Rust active summary: `sent_pings=5 received_pongs=5 success=true`.
- SBB passive summary: `received_pings=5 sent_pongs=5 success=true`.

`ChannelSupervisionFailure` diagnostics can appear during SBB interoperability
runs, but they did not prevent successful five-round Rust-to-SBB Ping/Pong
completion.

## Conclusion

**PASS** under the controlled recorded test conditions only. This result is not
certification, production readiness, an independent safety assessment, or proof
of full DIN conformance.

See the [final interop summary](../../docs/final-interop-summary.md) and
[Docker/Podman reproduction guide](../../docs/docker-interop.md) for the wider
recorded status and container commands.

# SBB wrapper plan

## Objective
Define the documentation-only plan for a small SBB wrapper executable that can later be used for Rust-to-SBB interoperability testing.

## Related requirement
Step 8B SBB wrapper design documentation and integration plan.

## Preconditions
The Step 8A SBB baseline investigation exists. SBB builds with CMake/Ninja, CTest passes `24/24`, no ready UDP client/server demo endpoint was found, and SBB requires integrator-provided adapter and transport functions.

## Test setup
No executable test is run in this planning step. Review `docs/sbb-wrapper-design.md`, `docs/sbb-baseline-investigation.md`, and this formal spec for completeness.

## Proposed commands
Future wrapper commands proposed by the plan:

```sh
sbb-rasta-wrapper active <remote_ip> --rounds 10 --run-seconds 30 --trace
sbb-rasta-wrapper passive <remote_ip> --rounds 10 --run-seconds 30 --trace
```

## Expected result
The design documents explain why a wrapper is needed, which SBB APIs and adapter functions must be integrated, how active/passive roles should work, how UDP transport channels should be mapped, and which trace evidence must be collected.

## Evidence to collect
- role and selected connection IDs
- local and remote UDP ports
- connection state changes
- TX/RX RedL frame lengths
- SafRetL message type or return code when accessible
- Ping/Pong counters
- adapter, transport, and SBB return codes
- packet lengths needed before any Rust-to-SBB profile or interoperability claim

## Postconditions
No Rust protocol behavior is changed. No Docker setup or Rust-to-SBB interoperability claim is made.

## Step 8F status
- Skeleton compile with real SBB: passed with `SBB_ROOT=/home/iheb/Desktop/sbb-investigation/sbb-rasta-stack`.
- Real SBB libraries linked: `librasta_common.a`, `librasta_redundancy.a`, and `librasta_safety_retransmission.a`.
- Wrapper smoke tests: passed for payload codec, UDP transport, RedL bridge, transport notification, SafRetL smoke, and wrapper help.

## Step 8I status
- SBB-to-SBB `Up` baseline: passed before this step.
- Previous gap: passive stopped after `Up` and heartbeat, so active Ping messages were not answered with Pong replies.
- Runtime change: passive remains alive after `Up`, decodes Ping counters, sends matching Pong counters, and exits successfully only after all requested rounds are answered.
- Active runtime change: active waits for all expected Pong counters before reporting success.
- Rust-to-SBB interoperability: pending; no success claim is made.

## Step 8J status
- SBB-to-SBB Ping/Pong: passed.
- Passive received `Ping(1)..Ping(5)` and sent `Pong(1)..Pong(5)`.
- Passive summary: `received_pings=5 sent_pongs=5 success=true`.
- Active received `Pong(1)..Pong(5)`.
- Active summary: `sent_pings=5 received_pongs=5 success=true`.
- Rust-to-SBB interoperability: pending; no success claim is made.

## Step 8K status
- SBB-to-SBB Ping/Pong: passed.
- Rust-to-SBB connection establishment: passed.
- Rust-to-SBB heartbeat exchange: passed.
- Rust-to-SBB application Ping/Pong: pending.
- Docker: pending.

Evidence summary:

- Rust active sent `6200` ConnectionRequest length `58` on both channels.
- Rust active received `6201` ConnectionResponse length `58`.
- Rust active transitioned `Opening -> Up`.
- Rust and SBB exchanged `6220` Heartbeat frames of length `44`.
- SBB passive reached `state=Up` and later observed `Closed after Up`.
- Full application data interoperability remains pending.

## Step 8L status
- `ping-pong-node --profile sbb-local`: runnable.
- SBB-local ping-pong active defaults: Rust local `7100/7101`, SBB remote `7000/7001`, IDs `0x61 -> 0x62`.
- SBB-local ping-pong passive defaults: Rust local `7000/7001`, remote `7100/7101`, IDs `0x62 -> 0x61`.
- Explicit channel port overrides are available.
- Rust-to-SBB Ping/Pong success: pending live Kali evidence.
- Docker: pending.

## Step 8M status
- SBB-to-SBB Ping/Pong 5 rounds: passed.
- Rust-to-SBB handshake/heartbeat: passed.
- Rust-to-SBB Ping/Pong 2 rounds: passed.
- Rust-to-SBB Ping/Pong 5 rounds: unstable / pending.
- Docker: pending.

Evidence summary:

- Rust active `ping-pong-node` used `--profile sbb-local`.
- Rust transitioned `Opening -> Up`.
- Rust sent `Ping(1)` and received `Pong(1)`.
- Rust sent `Ping(2)` and received `Pong(2)`.
- Rust completed two ping-pong rounds and started graceful disconnect.
- SBB passive received `Ping(1)` and `Ping(2)`.
- SBB passive sent `Pong(1)` and `Pong(2)`.
- SBB passive summary: `received_pings=2 sent_pongs=2 success=true`.

## Step 8N status
- SBB-to-SBB Ping/Pong 5 rounds: passed.
- Rust-to-SBB handshake/heartbeat: passed.
- Rust-to-SBB Ping/Pong 2 rounds: passed.
- Rust-to-SBB Ping/Pong 5 rounds: paced run pending live Kali evidence.
- Docker: pending.

Preparation summary:

- `ping-pong-node --profile sbb-local` defaults to a `300 ms` inter-ping delay.
- `--ping-delay-ms N` allows live-test override.
- Academic and `librasta-local` profiles keep a `0 ms` default.
- Rust active sends Ping `N+1` only after Pong `N` is decoded and the delay has elapsed.
- Rust active prints `active summary: sent_pings=N received_pongs=M success=true/false`.

## Automation status
Documentation/spec review only. Later steps add wrapper source, wrapper build tests, SBB-to-SBB baseline tests, and Rust-to-SBB preparation tests.

## Open points
- Observe timestamp behavior live.
- Run the paced Rust-to-SBB application Ping/Pong test for five rounds before claiming five-round success.

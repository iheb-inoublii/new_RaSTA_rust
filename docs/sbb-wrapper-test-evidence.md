# SBB Wrapper Test Evidence

## Environment

Wrapper path:

```text
/home/iheb/Desktop/new_RaSTA_rust/interop/sbb-wrapper
```

SBB checkout:

```text
/home/iheb/Desktop/sbb-investigation/sbb-rasta-stack
```

## Commands

Configure:

```sh
cmake -S. -B build -G Ninja -DSBB_ROOT=/home/iheb/Desktop/sbb-investigation/sbb-rasta-stack
```

Build:

```sh
cmake --build build --verbose
```

Smoke checks:

```sh
./ping_pong_payload_test
./udp_transport_test
./sbb_adapter_bridge_test
./sbb_transport_notification_test
./sbb_safretl_smoke_test
./sbb-rasta-wrapper --help
```

## Expected Result

The wrapper configures with a real `SBB_ROOT`, links SBB static libraries, builds `sbb-rasta-wrapper`, and passes wrapper smoke tests without claiming Rust-to-SBB interoperability.

## Actual Result

Configure printed:

```text
-- SBB_ROOT=/home/iheb/Desktop/sbb-investigation/sbb-rasta-stack
```

Build completed successfully and linked:

- `sbb-rasta-common/librasta_common.a`
- `sbb-rasta-redundancy/librasta_redundancy.a`
- `sbb-rasta-safety-retransmission/librasta_safety_retransmission.a`

Executable built:

- `build/sbb-rasta-wrapper`

Smoke checks passed:

- `ping_pong_payload_test`
- `udp_transport_test`
- `sbb_adapter_bridge_test`
- `sbb_transport_notification_test`
- `sbb_safretl_smoke_test`
- `sbb-rasta-wrapper --help`

## Step 8I Runtime Update

The SBB-to-SBB wrapper baseline reaches `Up`, but the passive side previously exited after observing `Up` and heartbeat. That made the active side's `Ping(1)..Ping(N)` sends visible without corresponding Pong replies.

Step 8I changes the runtime so passive stays alive after `Up`, reads Ping payloads, sends matching Pong payloads, and exits successfully only after answering the requested number of rounds. Active now exits successfully only after receiving every expected Pong.

Expected summary lines after a successful two-process run:

```text
[sbb-wrapper] active summary: sent_pings=N received_pongs=N success=true
[sbb-wrapper] passive summary: received_pings=N sent_pongs=N success=true
```

## Step 8J: SBB-to-SBB Ping/Pong Runtime Success

Kali commands:

```sh
./build/sbb-rasta-wrapper passive 127.0.0.1 --rounds 5 --trace --run-seconds 30
./build/sbb-rasta-wrapper active 127.0.0.1 --rounds 5 --trace --run-seconds 30
```

Filtered passive result:

```text
passive received Ping(1)
passive sent Pong(1)
passive received Ping(2)
passive sent Pong(2)
passive received Ping(3)
passive sent Pong(3)
passive received Ping(4)
passive sent Pong(4)
passive received Ping(5)
passive sent Pong(5)
passive Ping/Pong success condition reached
passive summary: received_pings=5 sent_pongs=5 success=true
```

Filtered active result:

```text
active received Pong(1)
active received Pong(2)
active received Pong(3)
active received Pong(4)
active received Pong(5)
active Ping/Pong success condition reached
active summary: sent_pings=5 received_pongs=5 success=true
```

Interpretation: SBB-wrapper-to-SBB-wrapper Ping/Pong works for five ordered application rounds over an established SBB RaSTA connection. This is not Rust-to-SBB interoperability.

## Step 8K: Rust-to-SBB Live Baseline

Kali SBB passive command:

```sh
./build/sbb-rasta-wrapper passive 127.0.0.1 \
  --rounds 5 --trace --run-seconds 30 \
  --channel0-local 7000 --channel0-remote 7100 \
  --channel1-local 7001 --channel1-remote 7101
```

Kali Rust active command:

```sh
cargo run -p rasta-node -- A 127.0.0.1 \
  --profile sbb-local \
  --trace-wire \
  --run-seconds 30 \
  --channel-0-local-port 7100 \
  --channel-0-remote-port 7000 \
  --channel-1-local-port 7101 \
  --channel-1-remote-port 7001
```

Observed Rust evidence:

- `Starting node A`
- `Local ID: 97`
- `Remote ID: 98`
- `Profile: SbbLocal`
- `Channel A: 0.0.0.0:7100 -> 127.0.0.1:7000`
- `Channel B: 0.0.0.0:7101 -> 127.0.0.1:7001`
- wire TX `6200` ConnectionRequest length `58` on both channels
- wire RX `6201` ConnectionResponse length `58`
- state transition `Opening -> Up`
- wire TX/RX `6220` Heartbeat length `44`
- wire RX `6216` Disconnect length `48`
- state transition `Up -> Down`

Observed SBB evidence:

- channel 0 local `7000`, remote `7100`
- channel 1 local `7001`, remote `7101`
- `srapi_GetConnectionState state=Up`
- received RedL frame `sr_type=0x184c(Heartbeat)`
- UDP send channel 0 length `44`
- UDP send channel 1 length `44`
- later `Closed after Up`

Status:

- SBB-to-SBB Ping/Pong: passed.
- Rust-to-SBB connection establishment: passed.
- Rust-to-SBB heartbeat exchange: passed.
- Rust-to-SBB application Ping/Pong: pending at Step 8K; passed for five rounds in Step 8O.
- Docker/Podman reproduction: passed later in Step 9B.

## Step 8L: Rust Ping-Pong Node Preparation

`ping-pong-node` can now be run with `--profile sbb-local` and explicit channel
port overrides. This prepares the Rust-to-SBB application Ping/Pong live test
without claiming success yet.

SBB passive command:

```sh
./build/sbb-rasta-wrapper passive 127.0.0.1 \
  --rounds 5 --trace --run-seconds 30 \
  --channel0-local 7000 --channel0-remote 7100 \
  --channel1-local 7001 --channel1-remote 7101
```

Rust active command:

```sh
cargo run -p ping-pong-node -- active 127.0.0.1 \
  --profile sbb-local \
  --rounds 5 \
  --trace-wire \
  --run-seconds 30 \
  --channel-0-local-port 7100 \
  --channel-0-remote-port 7000 \
  --channel-1-local-port 7101 \
  --channel-1-remote-port 7001
```

Status:

- Rust-to-SBB handshake/heartbeat: passed.
- Rust-to-SBB Ping/Pong: runnable.
- Rust-to-SBB Ping/Pong success: pending at Step 8L; passed for five rounds in Step 8O.

## Step 8M: Rust-to-SBB Ping/Pong 2-Round Success

Kali SBB passive command:

```sh
./build/sbb-rasta-wrapper passive 127.0.0.1 \
  --rounds 2 --trace --run-seconds 20 \
  --channel0-local 7000 --channel0-remote 7100 \
  --channel1-local 7001 --channel1-remote 7101
```

Kali Rust active command:

```sh
cargo run -p ping-pong-node -- active 127.0.0.1 \
  --profile sbb-local \
  --rounds 2 \
  --trace-wire \
  --run-seconds 20 \
  --channel-0-local-port 7100 \
  --channel-0-remote-port 7000 \
  --channel-1-local-port 7101 \
  --channel-1-remote-port 7001
```

Observed Rust evidence:

```text
Starting ping-pong-node Active
Profile: SbbLocal
Channel A: 0.0.0.0:7100 -> 127.0.0.1:7000
Channel B: 0.0.0.0:7101 -> 127.0.0.1:7001
State transition: Opening -> Up
Ping(1) sent
Pong(1) received
Ping(2) sent
Pong(2) received
Completed 2 ping-pong rounds
Graceful disconnect...
```

Observed SBB evidence:

```text
received Ping(1)
sent Pong(1)
received Ping(2)
sent Pong(2)
passive summary: received_pings=2 sent_pongs=2 success=true
```

Status:

- SBB-to-SBB Ping/Pong 5 rounds: passed.
- Rust-to-SBB handshake/heartbeat: passed.
- Rust-to-SBB Ping/Pong 2 rounds: passed.
- Rust-to-SBB Ping/Pong 5 rounds: passed in Step 8O.
- Docker/Podman reproduction: passed later in Step 9B.

## Step 8N: Rust-to-SBB 5-Round Pacing Preparation

At Step 8N, the five-round Rust-to-SBB Ping/Pong run remained pending. The
previous five-round attempt was unstable after two Ping/Pong exchanges and Rust
reported channel supervision diagnostics.

Step 8N changes only the Rust `ping-pong-node` test driver behavior:

- `--profile sbb-local` uses a `300 ms` default inter-ping delay.
- `--ping-delay-ms N` can override that delay.
- Academic and `librasta-local` keep a `0 ms` default.
- Active mode sends the next Ping only after the previous Pong is decoded and
  the delay has elapsed.
- Active mode prints `active summary: sent_pings=N received_pongs=M success=true/false`.

Suggested next Kali command for Rust active:

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

Status after Step 8N: Rust-to-SBB 5-round Ping/Pong was pending until the
paced run was verified live in Kali in Step 8O.

## Step 8O: Rust-to-SBB Ping/Pong 5-Round Success

The paced Step 8N command was verified live in Kali with SBB wrapper passive
and Rust `ping-pong-node` active.

SBB passive command:

```sh
./build/sbb-rasta-wrapper passive 127.0.0.1 \
  --rounds 5 --trace --run-seconds 30 \
  --channel0-local 7000 --channel0-remote 7100 \
  --channel1-local 7001 --channel1-remote 7101
```

Rust active command:

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

Observed SBB evidence:

```text
received Ping(5)
sent Pong(5)
passive Ping/Pong success condition reached
passive summary: received_pings=5 sent_pongs=5 success=true
```

Observed Rust evidence:

```text
State transition: Opening -> Up
Ping(1) sent / Pong(1) received
Ping(2) sent / Pong(2) received
Ping(3) sent / Pong(3) received
Ping(4) sent / Pong(4) received
Ping(5) sent / Pong(5) received
Completed 5 ping-pong rounds
active summary: sent_pings=5 received_pongs=5 success=true
```

ChannelSupervisionFailure diagnostics were observed during the run, but they
did not prevent the five ordered Ping/Pong rounds from completing successfully.

Status:

- SBB-to-SBB Ping/Pong 5 rounds: passed.
- Rust-to-SBB handshake/heartbeat: passed.
- Rust-to-SBB Ping/Pong 2 rounds: passed.
- Rust-to-SBB Ping/Pong 5 rounds: passed.
- Docker/Podman Rust-to-SBB 5-round Ping/Pong: passed in Step 9B.

## Limitations

This evidence proves Rust-to-SBB application Ping/Pong for five paced rounds.
Step 9B also proves Docker/Podman reproduction of the same five-round
Rust-to-SBB Ping/Pong scenario. Neither step modifies Rust protocol behavior or
Rust applications.

## Step 9B: Docker/Podman Rust-to-SBB Success

Docker/Podman validation passed for:

```sh
docker compose -f docker/interop/docker-compose.yml run --rm rust-test
docker compose -f docker/interop/docker-compose.yml run --rm sbb-wrapper-build
docker compose -f docker/interop/docker-compose.yml --profile live up --build
```

Observed live evidence:

```text
sbb-passive received Ping(5)
sbb-passive sent Pong(5)
passive Ping/Pong success condition reached
passive summary: received_pings=5 sent_pongs=5 success=true
rust-active Pong(5) received
rust-active Completed 5 ping-pong rounds
active summary: sent_pings=5 received_pongs=5 success=true
```

Earlier Docker/Podman build issue: native `interop/sbb-wrapper/build` cache
caused a CMake path mismatch inside `/workspace`. The workaround used was
`rm -rf interop/sbb-wrapper/build`. Add `.dockerignore` later to exclude build
artifacts permanently.

Status:

- Native SBB-to-SBB Ping/Pong 5 rounds: passed.
- Native Rust-to-SBB handshake/heartbeat: passed.
- Native Rust-to-SBB Ping/Pong 5 rounds: passed.
- Docker/Podman Rust tests: passed.
- Docker/Podman SBB wrapper build/tests: passed.
- Docker/Podman Rust-to-SBB 5-round Ping/Pong: passed.

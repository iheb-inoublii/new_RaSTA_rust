# Rust-to-SBB live baseline

## Objective
Record the first live Rust-to-SBB baseline evidence for connection establishment and heartbeat exchange.

## Related requirement
Rust-to-SBB interoperability preparation after the SBB wrapper baseline and SBB-to-SBB Ping/Pong runtime success.

## Preconditions
- SBB wrapper builds against the real SBB stack.
- SBB-wrapper-to-SBB-wrapper Ping/Pong has passed.
- Rust `rasta-node` supports the opt-in `sbb-local` profile.
- No Docker setup is required.

## Test setup
Run SBB wrapper as passive and Rust `rasta-node` as active on loopback with two UDP channels.

## Test data
- SBB passive channel 0: local `7000`, remote `7100`
- SBB passive channel 1: local `7001`, remote `7101`
- Rust active channel 0: local `7100`, remote `7000`
- Rust active channel 1: local `7101`, remote `7001`
- Rust profile: `sbb-local`
- Expected ConnReq RedL length: `58`
- Expected ConnResp RedL length: `58`
- Expected Heartbeat RedL length: `44`
- Expected Disconnect RedL length: `48`

## Test steps
1. Start SBB passive:

   ```sh
   ./build/sbb-rasta-wrapper passive 127.0.0.1 \
     --rounds 5 --trace --run-seconds 30 \
     --channel0-local 7000 --channel0-remote 7100 \
     --channel1-local 7001 --channel1-remote 7101
   ```

2. Start Rust active:

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

## Expected result
Rust and SBB establish a RaSTA connection and exchange heartbeat frames. Application Ping/Pong is not required for this baseline.

## Actual result
Rust evidence:

- `Starting node A`
- `Local ID: 97`
- `Remote ID: 98`
- `Profile: SbbLocal`
- channel A `0.0.0.0:7100 -> 127.0.0.1:7000`
- channel B `0.0.0.0:7101 -> 127.0.0.1:7001`
- wire TX `6200` ConnectionRequest length `58` on both channels
- wire RX `6201` ConnectionResponse length `58`
- state transition `Opening -> Up`
- wire TX/RX `6220` Heartbeat length `44`
- wire RX `6216` Disconnect length `48`
- state transition `Up -> Down`

SBB evidence:

- channel 0 local `7000`, remote `7100`
- channel 1 local `7001`, remote `7101`
- `srapi_GetConnectionState state=Up`
- received RedL frame `sr_type=0x184c(Heartbeat)`
- UDP send channel 0 length `44`
- UDP send channel 1 length `44`
- later `Closed after Up`

## Postconditions
- SBB-to-SBB Ping/Pong: passed.
- Rust-to-SBB connection establishment: passed.
- Rust-to-SBB heartbeat exchange: passed.
- Rust-to-SBB application Ping/Pong 2 rounds: passed.
- Rust-to-SBB application Ping/Pong 5 rounds: passed.
- Docker/Podman Rust-to-SBB 5-round Ping/Pong: passed.

## Step 8L runnable Ping-Pong command
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

Expected evidence: Rust sends ordered `Ping(1)..Ping(5)`, SBB replies with
ordered `Pong(1)..Pong(5)`, and both sides complete successfully.

## Step 8M actual Ping-Pong evidence
SBB passive:

```sh
./build/sbb-rasta-wrapper passive 127.0.0.1 \
  --rounds 2 --trace --run-seconds 20 \
  --channel0-local 7000 --channel0-remote 7100 \
  --channel1-local 7001 --channel1-remote 7101
```

Rust active:

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

Observed Rust result:

- `Starting ping-pong-node Active`
- `Profile: SbbLocal`
- channel A `0.0.0.0:7100 -> 127.0.0.1:7000`
- channel B `0.0.0.0:7101 -> 127.0.0.1:7001`
- state transition `Opening -> Up`
- `Ping(1) sent`
- `Pong(1) received`
- `Ping(2) sent`
- `Pong(2) received`
- `Completed 2 ping-pong rounds`
- `Graceful disconnect...`

Observed SBB result:

- `received Ping(1)`
- `sent Pong(1)`
- `received Ping(2)`
- `sent Pong(2)`
- `passive summary: received_pings=2 sent_pongs=2 success=true`

Status:

- SBB-to-SBB Ping/Pong 5 rounds: passed.
- Rust-to-SBB handshake/heartbeat: passed.
- Rust-to-SBB Ping/Pong 2 rounds: passed.
- Rust-to-SBB Ping/Pong 5 rounds: passed.
- Docker/Podman Rust-to-SBB Ping/Pong 5 rounds: passed in Step 9B.

## Step 9B Docker/Podman evidence
Docker/Podman Rust tests passed:

```sh
docker compose -f docker/interop/docker-compose.yml run --rm rust-test
```

Docker/Podman SBB wrapper build/tests passed:

```sh
docker compose -f docker/interop/docker-compose.yml run --rm sbb-wrapper-build
```

Docker/Podman live interop passed:

```sh
docker compose -f docker/interop/docker-compose.yml --profile live up --build
```

Observed live result:

- `sbb-passive received Ping(5)`
- `sbb-passive sent Pong(5)`
- `passive Ping/Pong success condition reached`
- `passive summary: received_pings=5 sent_pongs=5 success=true`
- `rust-active Pong(5) received`
- `rust-active Completed 5 ping-pong rounds`
- `active summary: sent_pings=5 received_pongs=5 success=true`

Earlier Docker/Podman build issue: native `interop/sbb-wrapper/build` cache
caused a CMake path mismatch inside `/workspace`. The workaround was
`rm -rf interop/sbb-wrapper/build`. Add `.dockerignore` later to exclude build
artifacts.

Status:

- Native SBB-to-SBB Ping/Pong 5 rounds: passed.
- Native Rust-to-SBB handshake/heartbeat: passed.
- Native Rust-to-SBB Ping/Pong 5 rounds: passed.
- Docker/Podman Rust tests: passed.
- Docker/Podman SBB wrapper build/tests: passed.
- Docker/Podman Rust-to-SBB 5-round Ping/Pong: passed.

## Step 8N pacing preparation
The first five-round Rust-to-SBB Ping/Pong attempt was unstable after two
rounds. Step 8N adds pacing to Rust active `ping-pong-node` without changing
protocol behavior:

- `--profile sbb-local` defaults to `--ping-delay-ms 300`.
- Academic and `librasta-local` default to `0 ms`.
- Active sends the next Ping only after the previous Pong is decoded and the
  configured delay has elapsed.
- Active prints `active summary: sent_pings=N received_pongs=M success=true/false`.

## Step 8O actual 5-round Ping-Pong evidence
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

Observed SBB result:

- `received Ping(5)`
- `sent Pong(5)`
- `passive Ping/Pong success condition reached`
- `passive summary: received_pings=5 sent_pongs=5 success=true`

Observed Rust result:

- state transition `Opening -> Up`
- `Ping(1)` sent and `Pong(1)` received
- `Ping(2)` sent and `Pong(2)` received
- `Ping(3)` sent and `Pong(3)` received
- `Ping(4)` sent and `Pong(4)` received
- `Ping(5)` sent and `Pong(5)` received
- `Completed 5 ping-pong rounds`
- `active summary: sent_pings=5 received_pongs=5 success=true`

`ChannelSupervisionFailure` diagnostics were observed during the run, but they
did not prevent the five-round application exchange from completing.

Status:

- SBB-to-SBB Ping/Pong 5 rounds: passed.
- Rust-to-SBB handshake/heartbeat: passed.
- Rust-to-SBB Ping/Pong 2 rounds: passed.
- Rust-to-SBB Ping/Pong 5 rounds: passed.
- Docker: pending at this step; passed in Step 9B.

## Evidence
Kali Rust and SBB wrapper logs.

## Automation status
Manual live test. Not yet automated.

## Open points
- Investigate the observed ChannelSupervisionFailure diagnostics.
- Add `.dockerignore` to exclude native build artifacts from Docker/Podman contexts.

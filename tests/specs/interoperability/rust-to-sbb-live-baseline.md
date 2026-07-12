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
- Rust-to-SBB application Ping/Pong: pending.
- Docker: pending.

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
  --channel-0-local-port 7100 \
  --channel-0-remote-port 7000 \
  --channel-1-local-port 7101 \
  --channel-1-remote-port 7001
```

Expected future evidence: Rust sends ordered `Ping(1)..Ping(5)`, SBB replies
with ordered `Pong(1)..Pong(5)`, and both sides complete successfully. This is
pending until captured in Kali.

## Evidence
Kali Rust and SBB wrapper logs.

## Automation status
Manual live test. Not yet automated.

## Open points
- Verify Rust-to-SBB application Ping/Pong.
- Keep Docker pending until the non-Docker live path is stable.
- Do not claim full Rust-to-SBB application interoperability until application data Ping/Pong passes.

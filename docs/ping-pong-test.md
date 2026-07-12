# Rust-to-Rust Ping-Pong Test

The ping-pong test exercises repeated bidirectional application data over an established RaSTA connection. It is intentionally simple and reusable for future interoperability work.

## Purpose

The scenario verifies that two Rust endpoints can exchange ordered request/response application messages while normal RaSTA polling, heartbeat handling, sequence supervision, and graceful disconnect continue.

## Message Flow

```text
active endpoint              passive endpoint
      | Ping(1)                      |
      | ---------------------------> |
      | Pong(1)                      |
      | <--------------------------- |
      | Ping(2)                      |
      | ---------------------------> |
      | Pong(2)                      |
      | <--------------------------- |
      | ...                          |
      | Ping(N)                      |
      | ---------------------------> |
      | Pong(N)                      |
      | <--------------------------- |
      | graceful disconnect          |
      | ---------------------------> |
```

Messages use the fixed-format `ApplicationMessage::Ping { counter }` and `ApplicationMessage::Pong { counter }` payloads.

## Difference From Signal/Interlocking

The signal/interlocking example demonstrates a domain-flavored controller workflow with SignalStatus and MovementAuthority messages.

The ping-pong test is a protocol exercise: it focuses on repeated ordered bidirectional data and is designed to map later to Rust-to-librasta and Rust-to-SBB ping-pong scenarios.

## Automated Test

The automated Rust-to-Rust test is:

```text
rust_to_rust_ping_pong_repeats_bidirectional_messages_and_disconnects_cleanly
```

It uses in-memory transports and the public `RastaEndpoint` API. It verifies ordered counters, no malformed messages, no timeout diagnostics, heartbeat activity, and graceful disconnect.

## Demo App

Start the passive node:

```powershell
cargo run -p ping-pong-node -- passive 127.0.0.1 --rounds 10 --run-seconds 30
```

Start the active node in another terminal:

```powershell
cargo run -p ping-pong-node -- active 127.0.0.1 --rounds 10 --run-seconds 30
```

Optional trace output:

```powershell
cargo run -p ping-pong-node -- passive 127.0.0.1 --rounds 10 --trace
cargo run -p ping-pong-node -- active 127.0.0.1 --rounds 10 --trace
```

## Rust-to-SBB Ping-Pong Preparation

Rust-to-SBB connection establishment and heartbeat exchange have passed with
`rasta-node --profile sbb-local`. Step 8L adds the same `--profile sbb-local`
selection to `ping-pong-node` so the application Ping/Pong live test is
runnable. Step 8M captured Rust-to-SBB Ping/Pong success for two rounds. A
five-round Rust-to-SBB Ping/Pong run passed in Step 8O with active-side pacing.

Start SBB passive:

```sh
./build/sbb-rasta-wrapper passive 127.0.0.1 \
  --rounds 5 --trace --run-seconds 30 \
  --channel0-local 7000 --channel0-remote 7100 \
  --channel1-local 7001 --channel1-remote 7101
```

Start Rust active `ping-pong-node`:

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

For the proven Step 8M live run, use `--rounds 2` and `--run-seconds 20` on
both sides. For the Step 8O five-round run, use the command above with
`--ping-delay-ms 300`.

Step 8N adds conservative active-side pacing for `ping-pong-node` when
`--profile sbb-local` is selected. The default inter-ping delay is `300 ms`,
and it can be overridden with `--ping-delay-ms N`. Academic and
`librasta-local` runs keep the existing fast default of `0 ms`. Active mode
only sends the next Ping after the previous Pong has been decoded, polling has
continued through the main loop, and the configured delay has elapsed.

Step 8O captured the paced five-round Rust-to-SBB live evidence. Rust sent and
received `Ping(1)..Ping(5)` / `Pong(1)..Pong(5)`, SBB passive reported
`received_pings=5 sent_pongs=5 success=true`, and Rust active reported
`sent_pings=5 received_pongs=5 success=true`. ChannelSupervisionFailure
diagnostics were observed during the run, but they did not prevent completion.

## Expected Output

The active node logs:

```text
Ping(1) sent
Pong(1) received
...
Completed 10 ping-pong rounds
Graceful disconnect...
```

The passive node logs:

```text
Ping(1) received
Pong(1) sent
...
```

## Future Interoperability

The payload format is intentionally fixed and compact so the same scenario can later be reused for:

- Rust to librasta
- Rust to SBB

Rust-to-SBB connection and heartbeat are proven. Rust-to-SBB application
Ping/Pong is proven for two rounds and for five paced rounds. Docker remains
pending.

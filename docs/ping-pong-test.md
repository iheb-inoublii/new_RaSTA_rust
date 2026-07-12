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

This step does not implement those interoperability tests yet.

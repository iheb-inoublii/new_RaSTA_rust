# Signal / Interlocking Example

This example demonstrates two simple object controllers communicating over the public RaSTA endpoint API.

The controllers are intentionally small:

- `signal-controller` acts as the active endpoint.
- `interlocking-controller` acts as the passive endpoint.
- both use `RastaEndpoint`, UDP transports from `rasta-platform`, and fixed-format application messages from `rasta-core::application`.

No protocol internals, packet structs, redundancy internals, retransmission buffers, or state-machine internals are accessed by the applications.

## Message Flow

```text
signal-controller            interlocking-controller
        |  SignalStatus(Red)              |
        | ------------------------------> |
        |  MovementAuthority(false)       |
        | <------------------------------ |
        |  SignalStatus(GreenRequested)   |
        | ------------------------------> |
        |  MovementAuthority(true)        |
        | <------------------------------ |
        |  Ping(1)                        |
        | ------------------------------> |
        |  Pong(1)                        |
        | <------------------------------ |
        |  Ping(2)                        |
        | ------------------------------> |
        |  Pong(2)                        |
        | <------------------------------ |
```

Heartbeats continue through the normal RaSTA endpoint polling loop.

## Run

Start the interlocking first:

```powershell
cargo run -p interlocking-controller -- 127.0.0.1 --run-seconds 20
```

Start the signal controller in a second terminal:

```powershell
cargo run -p signal-controller -- 127.0.0.1 --run-seconds 20
```

Optional tracing:

```powershell
cargo run -p interlocking-controller -- 127.0.0.1 --trace
cargo run -p signal-controller -- 127.0.0.1 --trace
```

The examples also accept `--profile academic` and `--profile librasta-local`; the default is `academic`.

## Expected Output

The signal logs status messages it sends and authority/pong messages it receives.

The interlocking logs status/ping messages it receives and authority/pong messages it sends.

Both applications log RaSTA state transitions and perform a graceful disconnect after the selected run duration.

## Public API Validation

The examples validate that the public API is sufficient for a bidirectional use case:

- construct an endpoint from profile/config and two transports
- connect and poll
- send and receive application data
- drain diagnostics and trace events
- close gracefully

Application payloads are fixed-size encoded with no heap allocation in `rasta-core`.

# Public API

The application-facing API is `rasta_core::endpoint::RastaEndpoint`. It wraps the lower protocol layers so applications do not need to handle packet encoding, redundancy internals, retransmission buffers, state machine details, or queue plumbing directly.

## Minimal Endpoint Example

```rust
use rasta_core::endpoint::RastaEndpoint;
use rasta_core::endpoint::config_from_profile;
use rasta_core::config::RastaProfile;

let config = config_from_profile(
    local_id,
    remote_id,
    RastaProfile::academic_default()?,
    false,
)?;

let mut endpoint = RastaEndpoint::from_config(
    channel_0_transport,
    channel_1_transport,
    clock,
    config,
)?;

endpoint.connect()?;

loop {
    endpoint.poll()?;

    if endpoint.has_received_data() {
        let mut data = [0u8; 256];
        let len = endpoint.receive(&mut data)?;
        // Use data[..len].
    }
}
```

## Builder API

`RastaEndpointBuilder` accepts the two transport channels and clock first, then IDs and either a full `RastaConfig` or a profile-derived config.

```rust
use rasta_core::endpoint::RastaEndpoint;
use rasta_core::config::RastaProfile;

let endpoint = RastaEndpoint::builder(channel_0_transport, channel_1_transport, clock)
    .local_id(local_id)
    .remote_id(remote_id)
    .profile(RastaProfile::academic_default()?)?
    .build()?;
```

For known unsafe interoperability profiles such as `librasta-local`, use the explicit opt-in path:

```rust
let endpoint = RastaEndpoint::builder(channel_0_transport, channel_1_transport, clock)
    .local_id(0x60)
    .remote_id(0x61)
    .unsafe_interop_profile(RastaProfile::librasta_local()?)?
    .build()?;
```

## Custom Profile

Custom profiles are created with `RastaProfileBuilder` and then passed to the endpoint builder.

```rust
use rasta_core::config::RastaProfileBuilder;

let profile = RastaProfileBuilder::new()
    .network_identifier(0x55aa)
    .timing(2500, 500, 125)
    .flow_control(12, 6)
    .build()?;
```

## Two Transports

The endpoint accepts two independent transport instances. They can be different concrete types as long as both implement `RastaTransport`.

Examples:

- channel 0: UDP adapter from `rasta-platform`
- channel 1: application-owned raw socket adapter
- channel 1: embedded Ethernet adapter
- tests: fixed-buffer mock transport

`rasta-core` defines the interface but does not implement raw sockets or OS-specific networking.

## Polling

Call `poll()` regularly. It processes incoming frames, sends due heartbeats, flushes queued application data, updates diagnostics, and advances endpoint status.

Use `status()` for coarse state:

- `Down`
- `Opening`
- `Up`
- `Retransmission`
- `Closing`

## Application Data

Use `send()` for application payloads and `receive()` with a caller-provided fixed buffer.

```rust
endpoint.send(b"hello")?;

let mut output = [0u8; 256];
let len = endpoint.receive(&mut output)?;
```

`receive()` returns `ReceiveQueueEmpty` when no application data is queued. `has_received_data()` is available for polling loops that prefer to check first.

## Close

Use `close()` for graceful disconnection.

```rust
endpoint.close()?;
```

## Diagnostics and Trace Events

Diagnostics and timestamp trace events can be drained without allocation:

```rust
endpoint.drain_diagnostics(|event| {
    // Log or inspect event.
});

endpoint.drain_trace_events(|event| {
    // Log or inspect timestamp trace event.
});
```

Wire tracing remains an adapter concern: wrap the transport if raw frame logging is needed.

## Errors

Public endpoint methods return `Result<T, RastaError>`. The error is intentionally coarse grained:

- configuration/profile errors
- invalid endpoint state
- transport failure
- packet/protocol rejection
- queue full or empty
- safety timeout
- retransmission unavailable

Detailed protocol diagnostics remain available through `take_diagnostic()` and `drain_diagnostics()`.

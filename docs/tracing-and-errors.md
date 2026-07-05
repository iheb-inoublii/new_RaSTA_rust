# Tracing and Errors

`rasta-core` exposes structured trace events through `RastaEndpoint` so applications can observe protocol behavior without decoding packet structs or redundancy internals.

## Enabling Tracing

Tracing is always bounded and optional from the application point of view. The core records events in fixed-capacity queues and applications drain them when needed:

```rust
endpoint.drain_trace_events(|event| {
    // format, log, or inspect the event
});
```

No heap allocation is required in `rasta-core`. If a trace queue fills, newer events are dropped deterministically and `trace_overflow_count()` reports the count.

## Trace Events

The public event type is `rasta_core::trace::RastaTraceEvent`.

Events include:

- packet TX/RX with direction, channel, frame length, redundancy sequence, SRL packet type, receiver/sender IDs, SN, CS, TS, and CTS
- state transition
- heartbeat sent and received
- application data sent and received
- diagnostic emitted
- timeout
- graceful disconnect
- timestamp compatibility data for peer-relative modes such as `librasta-local`

Packet traces intentionally expose decoded high-level fields, not internal packet structs.

## Draining Events

Use one-at-a-time draining:

```rust
while let Some(event) = endpoint.take_trace_event() {
    // handle event
}
```

Or use the helper:

```rust
endpoint.drain_trace_events(|event| {
    // handle event
});
```

## Trace Events, Diagnostics, and Errors

Trace events are observational. They describe what happened.

Diagnostics are protocol-level reports for conditions the connection can often continue through, such as malformed messages, safety-code failures, or channel supervision changes.

Errors are returned from public API calls when the requested operation failed or the connection must report a fatal condition.

## Public Error Model

Endpoint methods return `Result<T, RastaError>`.

`RastaError` is intentionally application-facing and stable. It groups internal details into high-level categories:

- configuration/profile errors
- transport failure
- packet/protocol rejection
- invalid state
- queue full or empty
- invalid payload size
- safety timeout
- retransmission unavailable
- random source failure

`ConfigError` and `TransportError` remain public for code that configures profiles or implements transports. Public errors implement `core::fmt::Display`, so they can be formatted in no-std-compatible code.

## Idiomatic Handling

```rust
match endpoint.poll() {
    Ok(()) => {}
    Err(error) => {
        // Coarse public error for control flow.
        log_error(error);

        // Detailed protocol evidence remains separate.
        endpoint.drain_diagnostics(|diagnostic| {
            log_diagnostic(diagnostic);
        });
    }
}
```

This keeps control-flow errors separate from continuing diagnostics and structured trace evidence.

# Transport Abstraction

`rasta-core` defines the transport interfaces used by the protocol, but it does not implement OS-specific or hardware-specific transports.

## Public trait

The public transport contract is `rasta_core::port::RastaTransport`.

```rust
use rasta_core::port::{RastaTransport, TransportError};

struct MyTransport;

impl RastaTransport for MyTransport {
    fn send(&mut self, frame: &[u8]) -> Result<(), TransportError> {
        // Send one complete redundancy-layer frame.
        Ok(())
    }

    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError> {
        // Copy one complete frame into buffer.
        // Return Ok(0) when no frame is currently available.
        Ok(0)
    }
}
```

The trait is synchronous, no-std compatible, and fixed-buffer oriented:

- `send` receives a complete redundancy-layer frame as a borrowed byte slice.
- `receive` writes into a caller-provided fixed buffer.
- `Ok(0)` means nonblocking/no-data and is not fatal.
- typed failures use `TransportError`.
- no heap allocation or `std::net` type is required in `rasta-core`.

`Transport` remains available as a backwards-compatible name for existing adapters. Implementations of `Transport` automatically satisfy `RastaTransport`.

## Per-channel transports

The redundancy layer is generic over two independent transport types:

```rust
use rasta_core::redundancy::RedundancyLayer;

let layer = RedundancyLayer::new(channel_0_transport, channel_1_transport);
```

The two channels do not need to use the same concrete type. A user can pair, for example:

- channel 0: OS UDP socket from `rasta-platform`
- channel 1: custom raw-socket transport from an application/platform crate

If transports must be selected at runtime, define an enum or wrapper that implements `RastaTransport` and delegates to the selected implementation. This keeps dynamic policy outside `rasta-core` while preserving the fixed-capacity core design.

## Adapter examples

UDP transport:
`rasta-platform::udp::UdpSocketTransport` keeps `std::net::UdpSocket` outside `rasta-core` and implements the transport contract.

Raw socket transport:
An application or platform crate can implement `RastaTransport` for a raw-socket adapter. `rasta-core` intentionally does not implement raw sockets because raw socket APIs are OS-specific, often require elevated privileges, and are not part of the protocol behavior.

Embedded Ethernet transport:
`rasta-platform::embedded_ethernet::EmbeddedEthernetAdapter` shows how a driver-style adapter can satisfy the same contract without moving hardware details into the core.

Mock transport:
Tests can implement `RastaTransport` or the compatibility `Transport` trait with fixed arrays to verify send, receive, no-data, and failure paths without allocation.

## Supervisor feedback

This design lets each redundancy channel own a separate transport instance. The RaSTA library provides the stable interface and channel management, while UDP, raw sockets, embedded Ethernet, or mocks remain pluggable outside the protocol core.

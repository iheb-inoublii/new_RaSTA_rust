# Guide to Build RaSTA Using Rust

This guide explains how to build a RaSTA-style Rust stack following the layered
architecture used in this project:

1. Railway signalling application / SCI
2. Application API / RaSTA service interface
3. Platform-independent RaSTA core
4. Abstract platform interfaces
5. Platform-specific adapters
6. Target platform / system services

The goal is to keep the protocol logic portable and `no_std` capable, while all
OS, socket, timer, clock, and hardware details stay outside the core.

Important note: this is a learning and reference roadmap. A production railway
safety implementation needs the official RaSTA specification, conformance
vectors, hazard analysis, review, target validation, and an approved safety
process.

## 1. Start With The Project Shape

Create the Rust crate first.

Recommended structure:

```text
rasta_stack/
  Cargo.toml
  README.md
  src/
    lib.rs
    application.rs
    core.rs
    platform.rs
    adapters.rs
    application/
      service_interface.rs
    core/
      connection.rs
      connection_state_machine.rs
      heartbeat.rs
      pdu.rs
      redundancy_management.rs
      retransmission.rs
      safety_code.rs
      sequencing.rs
      time_supervision.rs
    platform/
      clock.rs
      logger.rs
      synchronization.rs
      timer.rs
      transport.rs
    adapters/
      embedded_ethernet.rs
      linux.rs
      socket_transport.rs
      standard_clock.rs
      standard_timer.rs
      test.rs
      windows.rs
    bin/
      rasta_node.rs
    tests.rs
```

Why start here:

- The folder layout enforces the architecture.
- You can see immediately which code is portable and which code is platform-specific.
- The core can stay independent from sockets, threads, OS clocks, and heap allocation.

## 2. Configure Cargo

File: `Cargo.toml`

Start with a library crate and an optional `std` feature.

The core should compile without `std`; only adapters and binaries should require
`std`.

Recommended shape:

```toml
[package]
name = "rasta_stack"
version = "0.1.0"
edition = "2024"

[features]
default = []
std = []

[[bin]]
name = "rasta_node"
path = "src/bin/rasta_node.rs"
required-features = ["std"]

[dependencies]
```

Step-by-step:

1. Add `default = []` so the library does not automatically depend on `std`.
2. Add a `std` feature for desktop/socket examples.
3. Put runnable examples behind `required-features = ["std"]`.
4. Avoid dependencies at first. Add dependencies only when the architecture is stable.

Checkpoint:

```bash
cargo check
```

## 3. Create The Library Entry Point

File: `src/lib.rs`

This file exposes the main layers.

```rust
#![cfg_attr(not(feature = "std"), no_std)]

pub mod adapters;
pub mod application;
pub mod core;
pub mod platform;

#[cfg(test)]
mod tests;
```

Step-by-step:

1. Add `#![cfg_attr(not(feature = "std"), no_std)]`.
2. Export the four architecture layers.
3. Keep `tests` behind `#[cfg(test)]`.
4. Do not import any OS-specific code here.

Rule:

`lib.rs` should describe the architecture, not implement protocol behavior.

Checkpoint:

```bash
cargo check
```

## 4. Build The Abstract Platform Interfaces First

Start with the platform traits before implementing the protocol core. This makes
the core portable from the beginning.

### 4.1 Transport Interface

File: `src/platform/transport.rs`

Purpose:

Defines how bytes are sent and received without knowing whether the target uses
UDP, TCP, Ethernet, fieldbus, shared memory, or a test mock.

Implement:

```rust
#[derive(Debug, PartialEq)]
pub enum TransportError {
    SendFailed,
    ReceiveFailed,
    BufferTooSmall,
}

pub trait Transport {
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError>;
    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError>;
}
```

Design rules:

- Use byte slices.
- Do not use `Vec`.
- Do not mention UDP, TCP, Linux, or Windows.
- Keep errors small and portable.

### 4.2 Timer Interface

File: `src/platform/timer.rs`

Purpose:

Lets the core check heartbeat and liveness timeouts without depending on
`std::time` or an RTOS API.

Implement:

```rust
pub trait Timer {
    fn start(&mut self, duration_ms: u32);
    fn expired(&self) -> bool;
    fn stop(&mut self);
}
```

Design rules:

- The trait describes behavior only.
- The adapter decides whether this is implemented using `Instant`, hardware
  timers, or an RTOS timer.

### 4.3 Clock Interface

File: `src/platform/clock.rs`

Purpose:

Provides a monotonic millisecond value for timestamps and time supervision.

Implement:

```rust
pub trait Clock {
    fn now_ms(&self) -> u32;
}
```

Design rules:

- Keep it monotonic.
- Use wrapping arithmetic in the core to handle `u32` wraparound.
- Do not use wall-clock dates inside the core.

### 4.4 Logger Interface

File: `src/platform/logger.rs`

Purpose:

Allows diagnostics without coupling the core to `println!`, files, syslog, or
embedded logging frameworks.

Implement:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

pub trait Logger {
    fn log(&self, level: LogLevel, message: &str);
}
```

Design rules:

- Keep logging optional.
- The core should not require a logger to function.

### 4.5 Synchronization Interface

File: `src/platform/synchronization.rs`

Purpose:

Represents OS primitives or critical sections without tying the core to threads,
mutexes, or RTOS locks.

Implement:

```rust
pub trait CriticalSection {
    fn enter(&mut self);
    fn exit(&mut self);
}
```

Design rules:

- Do not use `std::sync` here.
- Desktop adapters may map this to a mutex.
- Embedded adapters may map this to interrupt masking or RTOS primitives.

### 4.6 Platform Module File

File: `src/platform.rs`

Expose the platform traits:

```rust
pub mod clock;
pub mod logger;
pub mod synchronization;
pub mod timer;
pub mod transport;
```

Checkpoint:

```bash
cargo check
```

## 5. Build The Platform-Independent Core

After the traits exist, build the protocol core. The core should import traits
from `platform`, but it must not import platform adapters.

### 5.1 Core Module File

File: `src/core.rs`

Expose the core responsibilities:

```rust
pub mod connection;
pub mod connection_state_machine;
pub mod heartbeat;
pub mod pdu;
pub mod redundancy_management;
pub mod retransmission;
pub mod safety_code;
pub mod sequencing;
pub mod time_supervision;
```

Recommended order to implement the files:

1. `pdu.rs`
2. `safety_code.rs`
3. `sequencing.rs`
4. `time_supervision.rs`
5. `redundancy_management.rs`
6. `retransmission.rs`
7. `heartbeat.rs`
8. `connection_state_machine.rs`
9. `connection.rs`

This order works because the connection logic depends on almost every other
core file.

## 6. Implement PDU / Packet Handling

File: `src/core/pdu.rs`

Purpose:

Serialize and parse Safety and Retransmission Layer PDUs.

What to define:

- `PacketType`
- `PacketError`
- `Packet`
- `Packet::parse`
- `Packet::serialize`

Fields to include:

```text
message length
message type
receiver id
sender id
sequence number
confirmed sequence number
timestamp
confirmed timestamp
payload
safety code
```

Step-by-step:

1. Define all packet type numbers.
2. Add a conversion function from `u16` to `PacketType`.
3. Define `PacketError` for invalid type, invalid length, checksum mismatch, and small buffers.
4. Use a fixed payload buffer, for example `[u8; 256]`.
5. In `parse`, check length before reading every field.
6. Decode numeric fields using little-endian.
7. Verify the safety code after reading the declared message length.
8. Copy payload bytes into the fixed payload buffer.
9. In `serialize`, write header fields first, then payload, then safety code.

Important rules:

- Never index blindly with `buffer[i]` unless the length is already proven.
- Prefer `.get()` and `.get_mut()` for checked access.
- Return errors instead of panicking.
- Keep payload bytes opaque. The application decides their meaning.

Tests to write:

- Serialize then parse a data packet.
- Reject too-small buffers.
- Reject invalid packet types.
- Reject wrong safety codes.
- Reject payloads larger than the fixed maximum.

## 7. Implement Safety Code / Integrity

File: `src/core/safety_code.rs`

Purpose:

Calculate and verify the RaSTA safety code.

What to define:

- `SafetyCodeMode`
- `SafetyCodeConfig`
- `Md4`

Recommended modes:

```rust
pub enum SafetyCodeMode {
    None,
    Md4Low8,
    Md4Full16,
}
```

Step-by-step:

1. Define the safety-code mode.
2. Define a config with the mode and MD4 initial value.
3. Add `len()` to return 0, 8, or 16 bytes.
4. Add `calculate()` to return the full 16-byte MD4 result.
5. Implement MD4 without allocation.
6. Keep MD4 state in fixed arrays.
7. Add known-vector tests for MD4.

Important rules:

- The safety-code module must not use external OS crypto providers.
- Keep it deterministic and portable.
- For a real product, verify against official RaSTA conformance vectors.

Tests to write:

- MD4 of empty input.
- MD4 of `abc`.
- PDU parse fails when safety code is modified.
- PDU parse succeeds for each configured safety-code length.

## 8. Implement Sequencing And Confirmation

File: `src/core/sequencing.rs`

Purpose:

Track transmit and receive sequence numbers, detect duplicates, and detect gaps.

What to define:

- `SequenceHandler`
- `SequenceResult`

Step-by-step:

1. Store `current_tx`.
2. Store `current_rx`.
3. Implement `next_tx()` to return the current value and increment with wrapping arithmetic.
4. Implement `validate_rx(received_seq)`.
5. Return `Ok` when the expected sequence arrives.
6. Return `Gap(expected)` when a future sequence arrives.
7. Return `Duplicate` when an older sequence arrives.
8. Add `confirmed_seq()` or `last_received_seq()` for outgoing confirmations.
9. Add a range check using `n_send_max` to reject unreasonable future packets.

Important rules:

- Use `wrapping_add` and `wrapping_sub`.
- Treat sequence wraparound intentionally.
- Do not silently accept large jumps.

Tests to write:

- First transmit sequence is the configured initial value.
- Receiving expected sequence advances `current_rx`.
- Receiving a future sequence returns `Gap`.
- Receiving an old sequence returns `Duplicate`.
- Wraparound behavior is correct.

## 9. Implement Time / Timestamp Supervision

File: `src/core/time_supervision.rs`

Purpose:

Check whether incoming packet timestamps are too old or too far in the future.

What to define:

- `TimeSupervisor`
- `TimeSupervisionError`

Step-by-step:

1. Store `t_max_ms`.
2. Store a small future tolerance, for example 100 ms.
3. Compare `local_now_ms` with `remote_timestamp_ms` using wrapping arithmetic.
4. If the packet is older than `t_max_ms`, reject it.
5. If the packet is too far in the future, reject it.
6. Otherwise accept it.

Important rules:

- Do not use wall-clock time.
- Do not assume timestamps never wrap.
- Keep the check platform-independent.

Tests to write:

- Accept a timestamp within `t_max`.
- Reject a timestamp older than `t_max`.
- Reject a timestamp too far in the future.

## 10. Implement Redundancy Management

File: `src/core/redundancy_management.rs`

Purpose:

Present two physical channels as one logical channel.

What to define:

- `RedundancyCheckCode`
- `RedundancyConfig`
- `RedundancyLayer<T1, T2>`

Step-by-step:

1. Store transport channel A and channel B.
2. Add a redundancy-layer frame header.
3. Add a redundancy sequence number.
4. On send, frame the payload and send a copy over both channels.
5. On receive, poll both channels.
6. Accept the first valid frame.
7. Discard duplicate channel copies.
8. Optionally add CRC16 or CRC32 for the redundancy frame.

Important rules:

- The redundancy layer depends only on `Transport`.
- It must not know whether the transport is UDP, Ethernet, or mock.
- Duplicate discard is essential because both channels may deliver the same PDU.

Tests to write:

- Sending writes to both transports.
- Receiving duplicate channel copies returns the payload once.
- Invalid frame length is discarded.
- Wrong CRC is discarded.

## 11. Implement Retransmission

File: `src/core/retransmission.rs`

Purpose:

Store recently sent data packets so they can be retransmitted if the receiver
detects a gap.

What to define:

- `RetransmissionBuffer`

Step-by-step:

1. Use a fixed array, for example `[Option<Packet>; 16]`.
2. Store outgoing data packets.
3. Track the oldest retained sequence.
4. Clear packets up to the confirmed sequence number.
5. Look up packets by sequence number.
6. When the buffer is full, decide whether to reject the new packet or replace the oldest.

Important rules:

- No heap allocation.
- Retain packets in sequence-aware order.
- Only store packets that may need retransmission, usually data packets.

Tests to write:

- Store and retrieve a packet.
- Clear acknowledged packets.
- Buffer full behavior.
- Wraparound behavior near `u32::MAX`.

## 12. Implement Heartbeat And Liveness

File: `src/core/heartbeat.rs`

Purpose:

Send heartbeat traffic and detect liveness timing through the abstract `Timer`.

What to define:

- `HeartbeatHandler<T: Timer>`

Step-by-step:

1. Store the timer object.
2. Store the heartbeat interval.
3. Implement `reset()` by stopping and starting the timer.
4. Implement `is_due()` by calling `timer.expired()`.
5. Let `connection.rs` decide which packet to send when heartbeat is due.

Important rules:

- Heartbeat code should not know packet layout.
- Heartbeat code should not know OS timer details.

Tests to write:

- New heartbeat is not due until timer says expired.
- Reset restarts the timer.

## 13. Implement The Connection State Machine

File: `src/core/connection_state_machine.rs`

Purpose:

Control legal connection states.

What to define:

- `RastaState`
- `StateMachine`

Recommended states:

```rust
pub enum RastaState {
    Down,
    Start,
    Up,
    Retransmission,
    Closed,
}
```

Step-by-step:

1. Start in `Down`.
2. Define valid transitions.
3. Reject invalid transitions.
4. Allow self-transitions only if they are harmless.
5. Keep transition validation in one place.

Typical transitions:

```text
Down -> Start
Start -> Up
Start -> Down
Up -> Retransmission
Retransmission -> Up
Up -> Closed
Closed -> Down
```

Tests to write:

- Initial state is `Down`.
- Valid transitions succeed.
- Invalid direct transitions fail, for example `Down -> Up`.

## 14. Implement The Main RaSTA Connection

File: `src/core/connection.rs`

Purpose:

Connect all core modules into a usable protocol connection.

What to define:

- `RastaConfig`
- `ConnectionError`
- `RastaConnection<T1, T2, TimerCtx, C>`

Recommended config fields:

```rust
pub struct RastaConfig {
    pub sender_id: u32,
    pub remote_id: u32,
    pub safety_code: SafetyCodeConfig,
    pub redundancy: RedundancyConfig,
    pub t_max: u32,
    pub initial_seq: u32,
    pub heartbeat_interval_ms: u32,
    pub n_send_max: u16,
}
```

Step-by-step:

1. Store the state machine.
2. Store the redundancy layer.
3. Store the clock and heartbeat handler.
4. Store sequence and retransmission state.
5. Store local and remote IDs.
6. Store safety-code and timing config.
7. Add fixed receive and transmit buffers.
8. Add a fixed application receive queue.
9. Implement `connect()`.
10. Implement `disconnect()`.
11. Implement `process()`.
12. Implement packet validation.
13. Implement packet handling by state.
14. Implement data send.
15. Implement retransmission request and response.
16. Implement receive queue access.

Packet receive flow:

```text
receive redundancy frame
parse PDU
check safety code
check IDs
check timestamp
check sequence
clear acknowledged retransmission buffer entries
dispatch packet by current state and packet type
```

Outgoing data flow:

```text
application calls send_data
service interface checks state is Up
connection creates Data PDU
PDU is serialized with safety code
redundancy layer sends over both channels
packet is stored in retransmission buffer
```

Important rules:

- `connection.rs` may depend on core modules and platform traits.
- `connection.rs` must not depend on platform adapters.
- Keep application payload storage fixed-size.
- Return errors instead of panicking.

Tests to write:

- Calling `connect()` moves from `Down` to `Start`.
- A valid connection request gets a response.
- Data packets are queued for application receive.
- Gap detection sends retransmission request.
- Duplicate packets are ignored.
- Bad timestamp closes or rejects the connection.
- Bad safety code closes or rejects the connection.

## 15. Build The Application API / RaSTA Service Interface

File: `src/application/service_interface.rs`

Purpose:

Expose a clean API to the railway signalling application / SCI. This file is
the boundary between application code and the RaSTA protocol stack.

What to define:

- `ConnectionStatus`
- `RastaService`
- Optional alias `RastaApi`

Step-by-step:

1. Wrap `RastaConnection`.
2. Expose `open_connection()`.
3. Expose `send_data()`.
4. Expose `receive_data()`.
5. Expose `close_connection()`.
6. Expose `status()`.
7. Expose `poll()` so the application can drive the stack.

Recommended API:

```rust
pub fn open_connection(&mut self) -> Result<(), ConnectionError>;
pub fn send_data(&mut self, data: &[u8]) -> Result<(), ConnectionError>;
pub fn receive_data(&mut self, output: &mut [u8]) -> Result<usize, ConnectionError>;
pub fn close_connection(&mut self) -> Result<(), ConnectionError>;
pub fn status(&self) -> ConnectionStatus;
pub fn poll(&mut self) -> Result<(), ConnectionError>;
```

Important rules:

- The application should not directly manipulate sequence numbers.
- The application should not directly serialize PDUs.
- The application should not directly access the redundancy layer.

File: `src/application.rs`

Expose the service interface:

```rust
pub mod service_interface;
```

Checkpoint:

```bash
cargo check
```

## 16. Build Platform-Specific Adapters Last

Adapters implement the platform traits. They are the only layer that should know
about sockets, `std::time`, test mocks, embedded Ethernet drivers, OS names, or
hardware services.

File: `src/adapters.rs`

Expose adapter modules:

```rust
pub mod embedded_ethernet;
pub mod test;

#[cfg(feature = "std")]
pub mod socket_transport;
#[cfg(feature = "std")]
pub mod standard_clock;
#[cfg(feature = "std")]
pub mod standard_timer;

#[cfg(all(feature = "std", target_os = "linux"))]
pub mod linux;
#[cfg(all(feature = "std", target_os = "windows"))]
pub mod windows;
```

### 16.1 Test Adapter

File: `src/adapters/test.rs`

Purpose:

Provide mock transport for tests and examples.

Step-by-step:

1. Store a fixed buffer for last sent bytes.
2. Implement `Transport`.
3. `send()` copies bytes into the buffer.
4. `receive()` can return zero or preloaded bytes depending on test needs.

Use this adapter to test the core without sockets.

### 16.2 Standard Clock Adapter

File: `src/adapters/standard_clock.rs`

Purpose:

Implement `Clock` using `std`.

Step-by-step:

1. Define `StdClock`.
2. Implement `Clock`.
3. Return milliseconds as `u32`.
4. Keep this behind `#[cfg(feature = "std")]` through `adapters.rs`.

### 16.3 Standard Timer Adapter

File: `src/adapters/standard_timer.rs`

Purpose:

Implement `Timer` using `std::time::Instant`.

Step-by-step:

1. Store `Option<Instant>`.
2. Store a `Duration`.
3. `start()` saves `Instant::now()`.
4. `expired()` compares elapsed time.
5. `stop()` clears the start time.

### 16.4 Socket Transport Adapter

File: `src/adapters/socket_transport.rs`

Purpose:

Implement `Transport` using a desktop UDP socket.

Step-by-step:

1. Bind a local UDP socket.
2. Connect it to a remote address.
3. Make it nonblocking.
4. `send()` calls `socket.send`.
5. `receive()` calls `socket.recv`.
6. Convert `WouldBlock` into `Ok(0)`.
7. Convert socket errors into `TransportError`.

Important rules:

- This file can use `std::net`.
- The core must never use `std::net`.

### 16.5 Embedded Ethernet Adapter

File: `src/adapters/embedded_ethernet.rs`

Purpose:

Wrap an embedded Ethernet or fieldbus driver behind the portable `Transport`
trait.

Step-by-step:

1. Define a small driver trait such as `EmbeddedEthernetDriver`.
2. Require `send_frame()` and `receive_frame()`.
3. Store the driver inside `EmbeddedEthernetAdapter`.
4. Implement `Transport` by delegating to the driver.

This lets each embedded target provide its own hardware driver while the RaSTA
core stays unchanged.

### 16.6 Linux And Windows Adapter Files

Files:

- `src/adapters/linux.rs`
- `src/adapters/windows.rs`

Purpose:

Provide OS-specific names, aliases, or wrappers.

Step-by-step:

1. Start with aliases to common socket adapters.
2. Add OS-specific implementation only when needed.
3. Keep OS-specific code behind `cfg`.

Example:

```rust
pub type LinuxUdpSocketAdapter = crate::adapters::socket_transport::UdpSocketTransport;
```

## 17. Build A Runnable Example

File: `src/bin/rasta_node.rs`

Purpose:

Demonstrate how an application uses the service API with concrete adapters.

Step-by-step:

1. Parse command-line mode, for example `A` or `B`.
2. Select local and remote addresses.
3. Create transport adapter A.
4. Create transport adapter B or a test adapter.
5. Create `RastaConfig`.
6. Create `RastaService`.
7. If active side, call `open_connection()`.
8. Loop:
   - call `poll()`
   - check `status()`
   - send data when `Up`
   - receive data when available
   - close when done

Important rule:

The example should use the public service API. It should not reach into the
connection internals unless it is specifically a low-level diagnostic example.

Checkpoint:

```bash
cargo run --features std --bin rasta_node -- A 127.0.0.1
```

## 18. Add Tests At Each Layer

File: `src/tests.rs`

Recommended test order:

1. PDU serialization and parsing.
2. MD4 safety-code known vectors.
3. Timestamp supervision.
4. Sequence handling.
5. State machine transitions.
6. Retransmission buffer.
7. Redundancy duplicate discard.
8. Connection handshake start.
9. Application receive queue.

Why this order:

- Low-level logic gets verified first.
- Connection tests are easier when packet, safety, sequence, and redundancy
  behavior already works.

Commands:

```bash
cargo test
cargo test --features std
```

## 19. Recommended Build Order Summary

Follow this order when rebuilding the project from scratch:

1. `Cargo.toml`
2. `src/lib.rs`
3. `src/platform/*`
4. `src/core/pdu.rs`
5. `src/core/safety_code.rs`
6. `src/core/sequencing.rs`
7. `src/core/time_supervision.rs`
8. `src/core/redundancy_management.rs`
9. `src/core/retransmission.rs`
10. `src/core/heartbeat.rs`
11. `src/core/connection_state_machine.rs`
12. `src/core/connection.rs`
13. `src/application/service_interface.rs`
14. `src/adapters/test.rs`
15. `src/tests.rs`
16. `src/adapters/standard_clock.rs`
17. `src/adapters/standard_timer.rs`
18. `src/adapters/socket_transport.rs`
19. `src/adapters/embedded_ethernet.rs`
20. `src/adapters/linux.rs`
21. `src/adapters/windows.rs`
22. `src/bin/rasta_node.rs`
23. `README.md`

## 20. Portability Checklist

Use this checklist before saying the stack works on all platforms:

- Core compiles without `std`.
- Core does not import `std::net`.
- Core does not import `std::time`.
- Core does not use threads.
- Core does not allocate with `Vec`, `Box`, or `String`.
- Core uses fixed-size buffers.
- Platform details are behind traits.
- Concrete adapters are behind `cfg` flags when needed.
- Tests run without the socket adapters.
- `std` examples compile only with `--features std`.

## 21. Safety Checklist

Use this checklist before increasing confidence in the protocol behavior:

- Every parser checks buffer length before reading.
- Every serializer checks output buffer capacity.
- Safety code is verified before accepting a PDU.
- Invalid packet type is rejected.
- Invalid message length is rejected.
- Sequence duplicates are ignored.
- Sequence gaps trigger retransmission.
- Timestamp too old is rejected.
- Timestamp too far in the future is rejected.
- Confirmed sequence numbers clear retransmission entries.
- Redundancy duplicates are discarded.
- Connection state transitions are validated.

## 22. What To Improve After The Basic Stack Works

After the roadmap implementation passes tests, continue with:

1. Add official RaSTA conformance test vectors.
2. Expand connection establishment validation.
3. Improve retransmission window behavior.
4. Add stronger diagnostics through the logger trait.
5. Add real dual-channel socket example.
6. Add embedded target examples.
7. Add fuzz tests for PDU parsing.
8. Add property tests for sequence wraparound.
9. Add integration tests with two connected nodes.
10. Document safety assumptions and limitations.

## 23. Final Mental Model

Build the project from the inside boundary outward:

```text
platform traits first
core protocol logic second
service API third
platform adapters fourth
examples and target integration last
```

The most important rule is simple:

The core should know what it needs, but never where it runs.


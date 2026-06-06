# RaSTA Protocol Stack (Educational Implementation)

This project is a safety-oriented, `no_std` implementation of the RaSTA (Rail Safe Transport Application) protocol, following MISRA-inspired design principles for safety-critical systems.

## Key Requirements & Implementation

| Requirement | Status | Implementation Detail |
|-------------|--------|-----------------------|
| **Safe Rust** | ✅ | No `unsafe` blocks, raw pointers, or manual memory management. |
| **No Dynamic Allocation** | ✅ | `#![no_std]` environment. No `Vec`, `Box`, or `String`. All buffers are fixed-size arrays. |
| **Full Protocol State Machine** | ✅ | Robust state transitions (Down -> Start -> Up -> Closed -> Down) with validation. |
| **Panic Risk Mitigation** | ✅ | Use of checked access (`.get()`, `.get_mut()`) and explicit bounds validation. |
| **RaSTA Features** | ✅ | Handshake, Heartbeats, Retransmission, Sequence Numbering, and MD4 Checksums. |
| **Unit Testing** | ✅ | Comprehensive test suite in `src/tests.rs` covering packet logic, states, and handshake. |

## Project Structure

- `src/core/`: Core protocol logic (packet parsing, state machine, retransmission).
- `src/platform/`: Traits for portability (Clock, Timer, Transport).
- `src/backends/`: Reference implementations for UDP/TCP and Mock objects.
- `src/application/`: High-level API for using the stack.
- `src/tests.rs`: Automated unit tests.

## Build and Test

```bash
# Check compilation
cargo check

# Run unit tests (requires MSVC or GNU toolchain)
cargo test
```

## Security Note

All packets are protected by a Safety Code (MD4) calculated as `MD4(SecurityKey | PDU)`. Timing and sequence numbers are validated to prevent replay and delay attacks.

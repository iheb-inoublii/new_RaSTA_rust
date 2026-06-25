# Cargo workspace migration

## Current active architecture

The active dependency direction is:

```text
rasta-node
    ↓
rasta-platform
    ↓
rasta-core
```

`rasta-core` owns the platform-independent protocol implementation:
configuration types, SRL types, service facade, connection handling, PDU
encode/decode, safety code, sequencing, retransmission, heartbeat, time
supervision, fixed queues, packet I/O, serial arithmetic, port traits, and
redundancy.

`rasta-platform` owns concrete platform adapters:

- `rasta_platform::udp::UdpSocketTransport`
- `rasta_platform::std_clock::StdClock`
- `rasta_platform::embedded_ethernet::{EmbeddedEthernetAdapter, EmbeddedEthernetDriver}`

`apps/rasta-node` owns the runnable demonstration node and the academic
interoperability-test profile in `apps/rasta-node/src/profile.rs`.

The original root package, `rasta_stack`, remains in place as a temporary
compatibility facade. Its retained protocol and adapter paths forward to the
canonical workspace crates.

## Removed obsolete compatibility modules

The old `Timer` abstraction and `StdTimer` adapter were removed after typed
monotonic deadlines became canonical. The unused logger trait, Linux/Windows UDP
alias modules, and public mock transport module were also removed because they
had no active consumers.

## Planned phases

1. Move platform-independent protocol utilities and port traits to `rasta-core`.
2. Group redundancy logic under the core crate without changing its behavior.
3. Replace the clock abstraction with typed monotonic time and add tests.
4. Move concrete adapters to `rasta-platform`.
5. Move the runnable node to `apps/rasta-node`.
6. Remove remaining temporary compatibility modules after downstream import
   migration.

## Protocol state names

`connection::state_machine::RastaState` drives the active connection state
machine. `srl::SrlState` is retained as an SRL-facing public type for
compatibility; it is not used internally to protect or advance connection
state.

## Concurrency boundary

The protocol core does not own synchronization primitives. A connection
instance requires exclusive mutable access while being processed. If an
integration shares it across threads or tasks, the integration layer is
responsible for providing the appropriate synchronization mechanism.

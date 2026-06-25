# Cargo workspace migration

## Temporary state

This repository now has a Cargo workspace skeleton. The platform-independent
RaSTA protocol implementation is canonical in `rasta-core`: configuration, SRL
types, service facade, connection handling, PDU encode/decode, safety code,
sequencing, retransmission, heartbeat, time supervision, fixed queues, packet
I/O, serial arithmetic, port traits, and redundancy.

The original root package, `rasta_stack`, remains in place as a temporary
compatibility facade. Its protocol module paths forward to `rasta-core` while
the concrete adapters and current demonstration binary remain in the root
package for now.

## Target dependency direction

```text
rasta-node
    ↓
rasta-platform
    ↓
rasta-core
```

`rasta-core` must never depend on `rasta-platform` or `rasta-node`.

## Planned phases

1. Move platform-independent protocol utilities and port traits to `rasta-core`.
2. Group redundancy logic under the core crate without changing its behavior.
3. Replace the clock abstraction with typed monotonic time and add tests.
4. Move concrete adapters to `rasta-platform`.
5. Move the runnable node to `apps/rasta-node`.
6. Remove temporary compatibility modules after import migration.

## Protocol state names

`connection::state_machine::RastaState` drives the active connection state
machine. `srl::SrlState` is retained as an SRL-facing public type for
compatibility; it is not used internally to protect or advance connection
state.

## Time compatibility

The active connection uses `rasta_core::time::MonotonicClock` and typed
deadlines. `platform::clock::Clock` is a compatibility alias for that canonical
trait. `platform::timer::Timer` and `adapters::standard_timer::StdTimer` remain
temporary compatibility scaffolding only; active protocol logic no longer uses
them and they are planned for removal in a later migration step.

## Concurrency boundary

The protocol core does not own synchronization primitives. A connection
instance requires exclusive mutable access while being processed. If an
integration shares it across threads or tasks, the integration layer is
responsible for providing the appropriate synchronization mechanism.

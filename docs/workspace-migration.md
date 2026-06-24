# Cargo workspace migration

## Temporary state

This repository now has a Cargo workspace skeleton. The original root package,
`rasta_stack`, remains in place and continues to own the existing protocol
implementation, public API, tests, and `rasta_node` demonstration binary.

The new packages are placeholders only. No protocol source code, packet format,
CRC, safety code, timing behavior, sequence handling, or test behavior has
changed in this phase.

## Target dependency direction

```text
apps/rasta-node
       ↓
crates/rasta-platform
       ↓
crates/rasta-core
```

`rasta-core` must never depend on `rasta-platform` or `rasta-node`.

## Planned phases

1. Move platform-independent protocol utilities and port traits to `rasta-core`.
2. Group redundancy logic under the core crate without changing its behavior.
3. Replace the clock abstraction with typed monotonic time and add tests.
4. Move concrete adapters to `rasta-platform`.
5. Move the runnable node to `apps/rasta-node`.
6. Remove temporary compatibility modules after import migration.

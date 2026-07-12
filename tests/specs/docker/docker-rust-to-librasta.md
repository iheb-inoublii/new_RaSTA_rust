# Docker Rust-to-librasta

## Objective
Define the future containerized Rust-to-librasta scenario.
## Related requirement
Future Docker plus librasta interoperability coverage.
## Preconditions
librasta Docker build recipe is available in a later phase.
## Test setup
Future Docker network with Rust and librasta containers.
## Test data
librasta-local profile and explicit unsafe/no-checksum opt-in.
## Test steps
Build images, start peers, run handshake, data, and heartbeat checks.
## Expected result
Interop behavior matches the local 40-second baseline.
## Postconditions
Containers stop and evidence logs are retained.
## Evidence
Future Docker logs.
## Automation status
Planned; no Docker implementation in this phase.

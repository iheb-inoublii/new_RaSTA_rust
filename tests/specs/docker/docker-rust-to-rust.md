# Docker Rust-to-Rust

## Objective
Define the future containerized Rust-to-Rust scenario.
## Related requirement
Future Docker test foundation.
## Preconditions
Docker support has not been implemented in this phase.
## Test setup
Future Docker network with two Rust containers.
## Test data
Academic/default profile, two channels, deterministic runtime.
## Test steps
Build image, start endpoint A and B, run handshake and data transfer.
## Expected result
Endpoints connect, exchange data, and disconnect gracefully.
## Postconditions
Containers stop and logs are retained.
## Evidence
Future Docker logs.
## Automation status
Planned; documentation only.

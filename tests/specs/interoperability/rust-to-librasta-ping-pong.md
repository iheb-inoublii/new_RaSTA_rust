# Rust-to-librasta ping-pong

## Objective
Verify repeated bidirectional payload exchange with librasta.
## Related requirement
Future ping-pong interoperability test.
## Preconditions
Rust-to-librasta data transfer succeeds.
## Test setup
Run Rust and librasta peers with bounded message count.
## Test data
Ping/pong payload numbers.
## Test steps
Rust sends ping; librasta replies pong; repeat.
## Expected result
All payload pairs complete in order without reconnect.
## Postconditions
Peers remain connected until graceful close.
## Evidence
Future interop logs.
## Automation status
Planned.

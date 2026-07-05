# Rust-to-SBB ping-pong

## Objective
Verify repeated bidirectional payload exchange with SBB.
## Related requirement
Future Rust-to-SBB ping-pong interoperability.
## Preconditions
Rust-to-SBB handshake succeeds.
## Test setup
Run Rust and SBB peers with bounded message count.
## Test data
TBD ping/pong payload format.
## Test steps
Rust sends ping; SBB replies pong; repeat.
## Expected result
All payload pairs complete in order.
## Postconditions
Peers remain connected until graceful close.
## Evidence
Future SBB interop logs.
## Automation status
Planned.

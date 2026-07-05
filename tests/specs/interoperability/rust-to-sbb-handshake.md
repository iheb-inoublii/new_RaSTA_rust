# Rust-to-SBB handshake

## Objective
Verify that a Rust endpoint establishes a connection with SBB.
## Related requirement
Future Rust-to-SBB interoperability.
## Preconditions
SBB baseline and confirmed profile values exist.
## Test setup
Run Rust endpoint and SBB peer with documented configuration.
## Test data
TBD SBB local IDs, ports, timing, safety, redundancy values.
## Test steps
Start both peers and poll until connected.
## Expected result
Handshake succeeds using evidence-based profile values.
## Postconditions
Peers remain available for ping-pong testing.
## Evidence
Future SBB interop logs.
## Automation status
Planned.

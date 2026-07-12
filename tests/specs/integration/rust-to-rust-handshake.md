# Rust-to-Rust handshake

## Objective
Verify that two Rust RaSTA endpoints establish a connection.
## Related requirement
Rust-to-Rust interoperability baseline.
## Preconditions
Academic/default profile is valid.
## Test setup
Run two Rust endpoints over mock or UDP transports.
## Test data
Default node IDs, two redundancy channels, academic/default timing.
## Test steps
Start endpoint A and B, open connection, poll until up.
## Expected result
Both endpoints reach `Up`.
## Postconditions
Connection can be closed cleanly.
## Evidence
Integration test logs.
## Automation status
Planned/partially covered by existing in-memory tests.

# Rust-to-Rust data transfer

## Objective
Verify application data transfer between Rust endpoints.
## Related requirement
SRL data delivery.
## Preconditions
Rust-to-Rust handshake succeeds.
## Test setup
Run two Rust endpoints with the academic/default profile.
## Test data
Small deterministic payloads.
## Test steps
Send data from A to B and from B to A.
## Expected result
Each payload is delivered once and unchanged.
## Postconditions
No pending retransmission remains.
## Evidence
Integration test logs.
## Automation status
Partially automated in core tests; scenario test planned.

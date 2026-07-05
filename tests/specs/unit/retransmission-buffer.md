# Retransmission buffer

## Objective
Verify fixed-capacity retransmission storage and resend behavior.
## Related requirement
Constant-memory RaSTA retransmission handling.
## Preconditions
Configured `n_send_max` is within supported bounds.
## Test setup
Use fixed buffers and mock transports.
## Test data
Packets up to capacity, over-capacity insertion, acknowledgement, retransmission request.
## Test steps
Store sent packets, acknowledge them, and request retransmission.
## Expected result
Capacity is enforced and requested packets are available without allocation.
## Postconditions
No dynamic memory is required.
## Evidence
Unit test output.
## Automation status
Partially automated in `crates/rasta-core/src/tests.rs`.

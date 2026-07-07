# Confirmed sequence handling

## Objective
Verify that peer confirmations remove acknowledged entries and reject invalid confirmations.
## Related requirement
RaSTA confirmed sequence supervision and retransmission support.
## Preconditions
Retransmission buffer contains sent packets.
## Test setup
Use mock connection state and deterministic sequence numbers.
## Test data
Valid ACK, duplicate ACK, stale ACK, and out-of-window ACK.
## Test steps
Process packets with confirmed sequence fields.
## Expected result
Valid confirmations advance acknowledgement state and invalid confirmations do not corrupt buffers.
## Postconditions
Retransmission buffer and sequence state remain bounded.
## Evidence
Unit test output.
## Automation status
Partially automated in `crates/rasta-core/src/tests.rs`.

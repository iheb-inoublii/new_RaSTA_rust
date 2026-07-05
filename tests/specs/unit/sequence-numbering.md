# Sequence numbering

## Objective
Verify transmit and receive sequence number progression.
## Related requirement
RaSTA sequence supervision.
## Preconditions
Connection sequence handler is initialized.
## Test setup
Use deterministic initial sequence values.
## Test data
Initial, next, stale, future, and wraparound sequence numbers.
## Test steps
Accept or reject incoming sequence values and advance transmit values.
## Expected result
Only expected sequence numbers advance state; invalid values are rejected.
## Postconditions
Sequence handler remains consistent after rejection.
## Evidence
Unit test output.
## Automation status
Partially automated in `crates/rasta-core/src/tests.rs`.

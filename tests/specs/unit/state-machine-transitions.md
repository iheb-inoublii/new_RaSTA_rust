# State machine transitions

## Objective
Verify legal and illegal connection state transitions.
## Related requirement
RaSTA connection state machine.
## Preconditions
State machine starts in the closed/down state.
## Test setup
Use direct state machine tests and connection-level scenarios.
## Test data
Open, start, up, retransmission requested, retransmission running, close, invalid transitions.
## Test steps
Apply transition events and inspect resulting state.
## Expected result
Legal transitions succeed; illegal transitions return errors without state corruption.
## Postconditions
Final state matches the last accepted transition.
## Evidence
Unit test output.
## Automation status
Automated in `crates/rasta-core/src/tests.rs`.

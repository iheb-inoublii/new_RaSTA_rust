# Heartbeat handling

## Objective
Verify periodic heartbeat generation and timeout behavior.
## Related requirement
RaSTA heartbeat supervision.
## Preconditions
Connection is open or up and heartbeat interval is configured.
## Test setup
Use fake clocks and mock transports.
## Test data
Heartbeat interval, received heartbeat, missing heartbeat, confirmed timestamp values.
## Test steps
Advance time, poll connection, inspect sent and received heartbeat behavior.
## Expected result
Heartbeats are sent at configured intervals and missing peer traffic triggers timeout.
## Postconditions
Connection state and diagnostics match timeout outcome.
## Evidence
Unit test output and future long-running interop logs.
## Automation status
Partially automated in `crates/rasta-core/src/tests.rs`.

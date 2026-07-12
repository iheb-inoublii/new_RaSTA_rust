# Graceful disconnect

## Objective
Verify orderly connection shutdown.
## Related requirement
RaSTA disconnection procedure.
## Preconditions
Connection is up.
## Test setup
Run two Rust endpoints.
## Test data
Disconnection request and response frames.
## Test steps
Endpoint A requests close; both endpoints poll until down.
## Expected result
Both endpoints leave `Up` without timeout or panic.
## Postconditions
No application data remains queued.
## Evidence
Integration test logs.
## Automation status
Partially automated; scenario test planned.

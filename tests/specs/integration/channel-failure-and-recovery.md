# Channel failure and recovery

## Objective
Verify behavior when one redundancy channel fails and later recovers.
## Related requirement
RaSTA redundancy resilience.
## Preconditions
Two-channel redundancy test succeeds.
## Test setup
Use controllable transports that can drop or resume frames.
## Test data
One channel disabled, one channel active, then both active.
## Test steps
Establish connection, fail one channel, exchange data, restore channel.
## Expected result
Connection remains up when at least one channel is usable and reports status changes.
## Postconditions
Recovered channel is usable for subsequent traffic.
## Evidence
Integration test logs.
## Automation status
Planned.

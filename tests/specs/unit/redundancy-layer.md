# Redundancy layer

## Objective
Verify redundancy frame encoding, CRC handling, channel status, and duplicate behavior.
## Related requirement
RaSTA redundancy layer.
## Preconditions
Two transports are available.
## Test setup
Use mock transports with fixed receive queues.
## Test data
Valid frames, CRC variants, duplicate frames, dropped frames, channel errors.
## Test steps
Send and receive redundancy frames over both channels.
## Expected result
Valid frames are delivered once; invalid frames are rejected and channel status is updated.
## Postconditions
Channel state remains bounded and deterministic.
## Evidence
Unit test output.
## Automation status
Partially automated in `crates/rasta-core/src/tests.rs`.

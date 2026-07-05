# Two-channel redundancy

## Objective
Verify Rust-to-Rust operation over both redundancy channels.
## Related requirement
RaSTA redundancy integration.
## Preconditions
Two transports are configured.
## Test setup
Run endpoints with two linked channels.
## Test data
Handshake and data frames sent over both channels.
## Test steps
Open connection and exchange data while both channels are available.
## Expected result
Data is delivered once and channel statuses remain healthy.
## Postconditions
Both channels can be closed without residual state.
## Evidence
Integration test logs.
## Automation status
Partially automated in core redundancy tests; scenario planned.

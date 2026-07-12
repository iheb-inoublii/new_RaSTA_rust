# Rust-to-librasta 40-second heartbeat

## Objective
Verify stable Rust-to-librasta connection supervision over 40 seconds.
## Related requirement
Supervisor-reported live interoperability baseline.
## Preconditions
Rust-to-librasta handshake succeeds.
## Test setup
Run Rust endpoint with `--profile librasta-local --run-seconds 40`.
## Test data
librasta-local heartbeat interval and timestamp compatibility.
## Test steps
Maintain connection for 40 seconds while collecting logs.
## Expected result
Connection remains up for the full duration without safety timeout.
## Postconditions
Rust endpoint disconnects gracefully when requested.
## Evidence
`interop/live-rust-to-librasta-result.md`.
## Automation status
Manual evidence exists; automated long-running test planned.

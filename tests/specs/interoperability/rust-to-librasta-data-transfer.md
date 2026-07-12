# Rust-to-librasta data transfer

## Objective
Verify application data exchange between Rust and librasta.
## Related requirement
Rust-to-librasta SRL compatibility.
## Preconditions
Rust-to-librasta handshake succeeds.
## Test setup
Run Rust endpoint and librasta peer with `--trace-wire` when evidence is needed.
## Test data
Small deterministic payloads.
## Test steps
Send payload from Rust to librasta and verify receipt.
## Expected result
Payload is delivered unchanged.
## Postconditions
Connection remains up until explicit close.
## Evidence
Wire trace and peer logs.
## Automation status
Manual baseline exists; automation planned.

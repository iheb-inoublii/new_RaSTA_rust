# Rust-to-librasta handshake

## Objective
Verify that a Rust endpoint establishes a connection with C librasta.
## Related requirement
Rust-to-librasta wire compatibility.
## Preconditions
librasta-local profile values are available and explicitly opt into unsafe no-checksum behavior.
## Test setup
Run Rust `rasta-node` and local librasta peer.
## Test data
librasta-local ports, IDs `0x60` and `0x61`, profile timing.
## Test steps
Start both peers and poll until connected.
## Expected result
Handshake succeeds without changing Rust protocol behavior.
## Postconditions
Peers remain available for data transfer.
## Evidence
Interop logs and wire trace.
## Automation status
Manual baseline exists; automation planned.

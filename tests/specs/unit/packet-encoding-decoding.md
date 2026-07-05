# Packet encoding/decoding

## Objective
Verify SRL packet serialization and parsing, including malformed input handling.
## Related requirement
DIN RaSTA packet wire compatibility.
## Preconditions
Safety code configuration is known.
## Test setup
Use fixed byte buffers and deterministic packet values.
## Test data
Connection, heartbeat, data, retransmission, disconnection, and malformed packets.
## Test steps
Serialize packets, parse packets, and feed malformed buffers of varying length.
## Expected result
Round trips preserve fields; malformed input returns errors and does not panic.
## Postconditions
No retained connection state is modified.
## Evidence
Unit test output and packet fixture assertions.
## Automation status
Partially automated in `crates/rasta-core/src/tests.rs`.

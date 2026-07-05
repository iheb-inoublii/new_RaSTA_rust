# Safety code / checksum / CRC

## Objective
Verify safety code and redundancy CRC calculation and rejection paths.
## Related requirement
DIN RaSTA safety code and redundancy checksum handling.
## Preconditions
Safety code mode and redundancy CRC option are configured.
## Test setup
Use fixed vectors and corrupted frames.
## Test data
MD4 lower-half vectors, no-safety interop vectors, redundancy CRC options.
## Test steps
Calculate codes, serialize packets, alter bytes, and parse/process results.
## Expected result
Known vectors match and corrupted messages are rejected or counted.
## Postconditions
Error counters and diagnostics reflect safety failures.
## Evidence
Unit test output.
## Automation status
Partially automated in `crates/rasta-core/src/tests.rs`.

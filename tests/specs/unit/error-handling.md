# Error handling

## Objective
Verify typed errors, diagnostics, and counters for invalid inputs and runtime failures.
## Related requirement
Robust protocol error reporting.
## Preconditions
Mock transports and deterministic clocks are available.
## Test setup
Inject invalid configuration, transport failures, malformed packets, and queue overflow.
## Test data
Invalid IDs, invalid timing, malformed wire data, failing transport, corrupted safety code.
## Test steps
Construct or process each invalid case.
## Expected result
Expected errors are returned and diagnostics/counters are updated where applicable.
## Postconditions
No panic or unbounded memory use occurs.
## Evidence
Unit test output.
## Automation status
Partially automated in `crates/rasta-core/src/tests.rs`.

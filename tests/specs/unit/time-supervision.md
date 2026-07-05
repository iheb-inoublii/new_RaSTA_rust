# Time supervision

## Objective
Verify timestamp and timeout supervision, including wraparound-safe comparisons.
## Related requirement
RaSTA timing supervision.
## Preconditions
Monotonic and protocol timestamp sources are deterministic.
## Test setup
Use fake clocks.
## Test data
Valid timestamps, stale timestamps, future timestamps, wraparound values, peer-relative timestamps.
## Test steps
Validate timestamps and process packets near timeout boundaries.
## Expected result
Valid timestamps pass; invalid or expired timestamps close or reject as specified.
## Postconditions
Diagnostic evidence is available for rejected timestamp paths.
## Evidence
Unit test output.
## Automation status
Automated in `crates/rasta-core/src/tests.rs`.

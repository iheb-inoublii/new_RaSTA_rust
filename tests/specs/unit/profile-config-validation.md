# Profile/config validation

## Objective
Verify that predefined and custom RaSTA profiles accept only valid protocol parameters.
## Related requirement
Supervisor feedback C and D.
## Preconditions
`rasta-core` builds without default features.
## Test setup
Run `rasta-core` unit tests.
## Test data
Academic/default, librasta-local, valid custom, invalid timing, unsafe/no-checksum custom profile.
## Test steps
Construct profiles through `RastaProfile` and `RastaProfileBuilder`.
## Expected result
Valid profiles build successfully; invalid timing and unsafe no-checksum without opt-in return typed `ConfigError` values.
## Postconditions
No runtime protocol state is changed.
## Evidence
Automated unit test output.
## Automation status
Automated in `crates/rasta-core/src/tests.rs`.

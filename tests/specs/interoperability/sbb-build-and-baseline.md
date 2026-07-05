# SBB build and baseline

## Objective

Record the manually verified SBB RaSTA stack build and unit-test baseline.

## Related requirement

Supervisor future-work request for SBB coverage and comparison against established RaSTA implementations.

## Preconditions

- Kali environment has the SBB repository cloned from `https://github.com/SchweizerischeBundesbahnen/sbb-rasta-stack`.
- CMake and Ninja are available.
- `libgmock-dev` is installed so `GTest::gmock` can be resolved.

## Test setup

Build the SBB repository locally in Debug mode with exported compile commands.

## Test steps

1. Run:

   ```sh
   cmake -DCMAKE_BUILD_TYPE:STRING=Debug \
         -DCMAKE_EXPORT_COMPILE_COMMANDS:BOOL=TRUE \
         -S. -B./build -G Ninja
   ```

2. Run:

   ```sh
   cmake --build ./build --config Debug --target all --
   ```

3. Run:

   ```sh
   ctest --test-dir ./build --output-on-failure
   ```

4. Inspect produced executables.
5. Record RedL and SafRetL configuration evidence.

## Expected result

- CMake configure succeeds.
- Build succeeds.
- CTest passes.
- Available runtime/demo executables are identified if present.
- SBB configuration values relevant to future Rust-to-SBB interop are recorded.

## Actual result

- CMake configure passed after installing `libgmock-dev`.
- Build passed.
- CTest passed: `24/24` tests passed.
- Only GoogleTest unit-test binaries were found, including `gtest_raas`, `gtest_radef`, `gtest_redmsg`, `gtest_redcrc`, `gtest_redint`, `gtest_srapi`, `gtest_srmsg`, `gtest_srsend`, `gtest_srrece`, `gtest_srmd4`, and `gtest_srstm`.
- No ready UDP client/server demo executable was found.
- RedL and SafRetL configuration evidence is documented in `docs/sbb-baseline-investigation.md`.

## Postconditions

No Rust protocol code or profile behavior is changed. No `sbb-local` profile is added.

## Evidence

- `docs/sbb-baseline-investigation.md`
- Manual Kali build notes summarized there.

## Automation status

Manual evidence captured. Not automated in this repository yet.

## Open points

- A small SBB adapter/wrapper executable is likely required for live Rust-to-SBB testing.
- SBB transport adapter functions must be implemented by an integrator.
- Rust-to-SBB interop is not yet claimed.
- `RastaProfile::sbb_local()` should wait until runnable endpoint evidence exists.

# SBB Wrapper Build

## Objective

Verify that the `interop/sbb-wrapper` skeleton configures, builds, and runs basic smoke checks in the Kali environment that contains the SBB checkout.

## Preconditions

- The Rust repository is available in Kali.
- SBB checkout exists at `/root/sbb-investigation/sbb-rasta-stack`.
- CMake, Ninja, and a C compiler are available in Kali.
- The SBB baseline has already configured, built, and passed `24/24` CTest tests.
- No Rust protocol behavior, Rust profiles, Docker setup, or Rust applications are modified.

## Build steps

From the Rust repository root:

```sh
cmake -S interop/sbb-wrapper \
      -B interop/sbb-wrapper/build \
      -G Ninja \
      -DSBB_ROOT=/root/sbb-investigation/sbb-rasta-stack
cmake --build interop/sbb-wrapper/build
ctest --test-dir interop/sbb-wrapper/build
./interop/sbb-wrapper/build/sbb-rasta-wrapper passive 127.0.0.1 --rounds 3 --trace
./interop/sbb-wrapper/build/sbb-rasta-wrapper active 127.0.0.1 --rounds 3 --trace
```

## Expected result

- CMake configure succeeds.
- CMake build succeeds.
- `ping_pong_payload_test` passes.
- Passive and active CLI smoke checks print parsed settings.
- Stub initialization functions print deterministic logs.
- Send smoke checks report `radef_kNotImplemented`.
- Read smoke checks report `radef_kNoMessageReceived`.
- Both CLI smoke checks exit successfully.
- No Rust-to-SBB interoperability is claimed.

## Actual result

Not completed in the available Codex host environment.

Observed host limitations:

- `wsl -l -v` listed only `docker-desktop`; Kali was not available.
- `/root/sbb-investigation/sbb-rasta-stack` was not reachable.
- `cmake` was not available on PATH.
- `ninja` was not available on PATH.
- `gcc`, `clang`, and `cl` were not available on PATH.

Rust validation did run and passed:

```sh
cargo fmt --all -- --check
cargo test --workspace --all-targets --all-features
```

## Postconditions

- Wrapper skeleton remains isolated under `interop/sbb-wrapper`.
- Rust protocol behavior remains unchanged.
- Rust apps remain unchanged.
- `RastaProfile::sbb_local()` is still not added.
- Docker is still not implemented.

## Evidence

- `interop/sbb-wrapper/README.md`
- `interop/sbb-wrapper/CMakeLists.txt`
- `interop/sbb-wrapper/src/main.c`
- `interop/sbb-wrapper/src/sbb_adapter.c`
- `interop/sbb-wrapper/src/udp_transport.c`
- `interop/sbb-wrapper/src/ping_pong_payload.c`
- `docs/sbb-wrapper-design.md`

## Automation status

Partially automated. Rust workspace validation is automated and passing. Kali CMake/Ninja wrapper validation is pending until the repository is opened in the Kali environment with the SBB checkout.

## Open points

- Run the recorded commands in Kali.
- Capture the exact CMake configure/build output.
- Capture passive and active CLI smoke output.
- Fix wrapper-only CMake/source issues if the Kali build exposes any.

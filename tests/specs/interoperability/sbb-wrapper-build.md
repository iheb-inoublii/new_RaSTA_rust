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

Passed in Kali.

- CMake configure passed with `SBB_ROOT=/root/sbb-investigation/sbb-rasta-stack`.
- CMake build passed.
- The build created `interop/sbb-wrapper/build/ping_pong_payload_test`.
- The build created `interop/sbb-wrapper/build/sbb-rasta-wrapper`.
- Passive smoke test passed:

  ```sh
  ./interop/sbb-wrapper/build/sbb-rasta-wrapper passive 127.0.0.1 --rounds 3 --trace
  ```

- Active smoke test passed:

  ```sh
  ./interop/sbb-wrapper/build/sbb-rasta-wrapper active 127.0.0.1 --rounds 3 --trace
  ```

- The wrapper clearly logs skeleton-only status and does not claim Rust-to-SBB interoperability.
- UDP is still stubbed.
- Send functions are stubbed.
- Read functions return no message.

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

Partially automated. The Kali CMake/Ninja build and CLI smoke checks have been run manually and passed. Rust workspace validation remains automated through Cargo.

## Open points

- Capture full command output as an artifact in a future evidence file if required.
- Confirm exact SBB adapter/transport function signatures before linking real SBB libraries.
- Implement real UDP behavior only after the wrapper is connected to SBB APIs.

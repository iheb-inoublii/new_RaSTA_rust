# SBB Wrapper Skeleton

## Objective

Provide a compile-ready SBB wrapper skeleton in the Rust repository without claiming Rust-to-SBB interoperability.

## Preconditions

- SBB baseline investigation is documented.
- SBB wrapper design is documented.
- The external SBB checkout is not modified by this test.
- Rust protocol behavior, profiles, transports, and applications remain unchanged.

## Build steps

1. Configure the wrapper skeleton:

   ```sh
   cmake -S interop/sbb-wrapper -B interop/sbb-wrapper/build -G Ninja -DSBB_ROOT=/root/sbb-investigation/sbb-rasta-stack
   ```

2. Build the wrapper skeleton:

   ```sh
   cmake --build interop/sbb-wrapper/build
   ```

3. Run the wrapper skeleton CTest tests:

   ```sh
   ctest --test-dir interop/sbb-wrapper/build
   ```

4. Run the Rust workspace validation:

   ```sh
   cargo fmt --all -- --check
   cargo test --workspace --all-targets --all-features
   ```

## Expected result

- The CMake project configures without requiring SBB to be vendored into the Rust repository.
- The wrapper executable builds.
- The Ping/Pong payload codec test passes.
- The wrapper CLI accepts active/passive role, remote IP, rounds, run seconds, trace mode, and two channel local/remote ports.
- The wrapper prints deterministic settings, opens UDP sockets, prints read/send statuses, closes sockets, and exits successfully.
- Required `sradin_*` and `redtri_*` symbols exist as logged stubs.
- SafRetL read stubs return `radef_kNoMessageReceived` when no queue is implemented.
- SafRetL send stubs do not fake successful interoperability.
- RedL send/read functions delegate to wrapper UDP transport.
- Rust workspace tests continue to pass.

## Current status

Skeleton with real UDP transport only. The wrapper builds in Kali, opens UDP sockets, and has UDP-backed RedL adapter functions, but it does not link SBB libraries, does not establish a RaSTA connection, and does not demonstrate Rust-to-SBB interoperability.

Step 8D verified that CMake configure and build pass in Kali with `SBB_ROOT=/root/sbb-investigation/sbb-rasta-stack`. The build creates `ping_pong_payload_test` and `sbb-rasta-wrapper`. Passive and active CLI smoke checks pass, while still logging skeleton-only status.

The wrapper CMake now accepts both `SBB_ROOT` and the early planning alias `SBB_RASTA_ROOT`; `SBB_RASTA_ROOT` is mapped to `SBB_ROOT` when the latter is unset.

Step 8F supersedes the earlier stub-only status for real SBB compile evidence: the wrapper configured with `SBB_ROOT=/home/iheb/Desktop/sbb-investigation/sbb-rasta-stack`, linked real SBB static libraries, built `build/sbb-rasta-wrapper`, and passed the payload, UDP, RedL bridge, transport notification, SafRetL smoke, and help smoke checks.

The Step 8F result is still wrapper-only. It distinguishes real SBB-linked compile and smoke success from Rust-to-SBB live interoperability, which remains pending.

## Open points

- Confirm exact SBB adapter and transport function signatures.
- Confirm exact SBB return-code names and values.
- Confirm SBB timing API name and expected call cadence.
- Keep validating external SBB library linkage through `SBB_ROOT`.
- Implement bounded queues for SafRetL read functions if required by later live scenarios.
- Run SBB-to-SBB wrapper baseline before Rust-to-SBB.

## Evidence

- `interop/sbb-wrapper/README.md`
- `interop/sbb-wrapper/CMakeLists.txt`
- `interop/sbb-wrapper/src/main.c`
- `interop/sbb-wrapper/src/sbb_adapter.c`
- `interop/sbb-wrapper/src/udp_transport.c`
- `interop/sbb-wrapper/src/ping_pong_payload.c`
- `docs/sbb-wrapper-design.md`

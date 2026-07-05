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
- The wrapper prints deterministic settings, read/send stub statuses, and exits successfully.
- Required `sradin_*` and `redtri_*` symbols exist as logged stubs.
- Read stubs return `radef_kNoMessageReceived` when no queue is implemented.
- Send stubs do not fake successful interoperability.
- Rust workspace tests continue to pass.

## Current status

Skeleton only. The wrapper does not link SBB libraries, does not open UDP sockets, does not establish a RaSTA connection, and does not demonstrate Rust-to-SBB interoperability.

Step 8D recorded that the available Codex host could not run the Kali wrapper build because Kali, the SBB checkout path, CMake, Ninja, and C compilers were not available from this session. The intended Kali commands are preserved here and in `interop/sbb-wrapper/README.md`.

## Open points

- Confirm exact SBB adapter and transport function signatures.
- Confirm exact SBB return-code names and values.
- Confirm SBB timing API name and expected call cadence.
- Link external SBB libraries through `SBB_ROOT`.
- Implement real UDP socket behavior outside `rasta-core`.
- Implement bounded queues for read functions.
- Run SBB-to-SBB wrapper baseline before Rust-to-SBB.

## Evidence

- `interop/sbb-wrapper/README.md`
- `interop/sbb-wrapper/CMakeLists.txt`
- `interop/sbb-wrapper/src/main.c`
- `interop/sbb-wrapper/src/sbb_adapter.c`
- `interop/sbb-wrapper/src/udp_transport.c`
- `interop/sbb-wrapper/src/ping_pong_payload.c`
- `docs/sbb-wrapper-design.md`

# SBB Wrapper RedL Bridge

## Objective

Verify the Step 8F internal bridge from the SBB SafRetL adapter functions to SBB RedL inside the wrapper.

## Preconditions

- The Rust repository is available in Kali/Linux.
- SBB checkout exists at `/root/sbb-investigation/sbb-rasta-stack`.
- CMake, Ninja, and a C compiler are available.
- Step 8E UDP transport validation has passed.
- Rust protocol behavior, Rust profiles, Docker setup, and Rust applications are unchanged.

## Build steps

From the Rust repository root:

```sh
cmake -S interop/sbb-wrapper \
      -B interop/sbb-wrapper/build \
      -G Ninja \
      -DSBB_ROOT=/root/sbb-investigation/sbb-rasta-stack
cmake --build interop/sbb-wrapper/build
```

## Test steps

Run existing wrapper smoke tests:

```sh
./interop/sbb-wrapper/build/ping_pong_payload_test
./interop/sbb-wrapper/build/udp_transport_test
```

Run the RedL bridge test:

```sh
./interop/sbb-wrapper/build/sbb_adapter_bridge_test
```

Run wrapper CLI smoke tests:

```sh
./interop/sbb-wrapper/build/sbb-rasta-wrapper passive 127.0.0.1 --rounds 3 --trace
./interop/sbb-wrapper/build/sbb-rasta-wrapper active 127.0.0.1 --rounds 3 --trace
```

## Expected result

- CMake configure succeeds with `SBB_ROOT`.
- CMake build succeeds.
- Existing payload and UDP tests pass.
- `sbb_adapter_bridge_test` initializes UDP, initializes RedL through `sradin_Init`, opens redundancy channel 0, invokes `sradin_SendMessage`, verifies the read path returns no-message or success, closes the channel, and closes UDP.
- Runtime log says Step 8F RedL bridge smoke only.
- No Rust-to-SBB interoperability is claimed.

## SBB RedL functions used

- `redint_Init`
- `redint_OpenRedundancyChannel`
- `redint_CloseRedundancyChannel`
- `redint_SendMessage`
- `redint_ReadMessage`
- `redint_CheckTimings`

The SBB transport notification function `redtrn_MessageReceivedNotification` was inspected. The wrapper currently uses `redint_CheckTimings` for RedL polling before reads.

## Current status

Implemented and validated in Kali as RedL bridge smoke only. This is not Rust-to-SBB interoperability.

## Link-error cleanup

Kali configure found `SBB_ROOT` at `$HOME/Desktop/sbb-investigation/sbb-rasta-stack`, but the first real SBB link failed with undefined references from SBB to integrator-provided functions.

Added wrapper-side functions:

- `rasys_GetTimerValue`
- `rasys_GetTimerGranularity`
- `rasys_GetRandomNumber`
- `rasys_FatalError`
- `rednot_MessageReceivedNotification`
- `rednot_DiagnosticNotification`

The wrapper common target is now an object library so these implementations are linked directly into the final executable and smoke test targets.

## Actual result

- Real SBB_ROOT was used: `-DSBB_ROOT=$HOME/Desktop/sbb-investigation/sbb-rasta-stack`.
- Real SBB libraries linked successfully after adding callback/system adapters.
- `ping_pong_payload_test` passed.
- `udp_transport_test` passed.
- `sbb_adapter_bridge_test` passed.
- `sbb_adapter_bridge_test` showed:

  ```text
  sradin_Init -> redint_Init result=0
  sradin_OpenRedundancyChannel channel 0 result=0
  redtri_SendMessage transport 0 length=36 sent
  redtri_SendMessage transport 1 length=36 sent
  sradin_SendMessage channel 0 length=28 redint_SendMessage result=0
  sradin_ReadMessage timing_result=0 read_result=1 length=0
  sradin_CloseRedundancyChannel channel 0 result=0
  ```

- Passive and active wrapper CLI smoke passed.
- CLI smoke with the 5-byte dummy message returns result `17` from RedL, expected because it is not a valid/minimum SafRetL PDU.
- Runtime log correctly says Step 8F SBB RedL bridge smoke only; no Rust-to-SBB interop is claimed.

## Open points

- Add bounded adapter queues if SafRetL requires asynchronous handoff.
- Step 8G supersedes this RedL-only scope by adding a smoke-only SBB SafRetL API run loop.
- Run an SBB-to-SBB wrapper baseline.
- Run Rust-to-SBB only after profile/config evidence exists.

## Evidence

- `interop/sbb-wrapper/src/sbb_adapter.c`
- `interop/sbb-wrapper/src/sbb_adapter.h`
- `interop/sbb-wrapper/src/sbb_system_adapter.c`
- `interop/sbb-wrapper/src/sbb_redundancy_notifications.c`
- `interop/sbb-wrapper/tests/sbb_adapter_bridge_test.c`
- `interop/sbb-wrapper/CMakeLists.txt`

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

Implemented in wrapper source. Kali CMake validation is required after this change is pushed or copied into the Kali checkout.

## Open points

- Validate `sbb_adapter_bridge_test` in Kali with the real SBB checkout.
- Add bounded adapter queues if SafRetL requires asynchronous handoff.
- Implement the SBB SafRetL API run loop.
- Run an SBB-to-SBB wrapper baseline.
- Run Rust-to-SBB only after profile/config evidence exists.

## Evidence

- `interop/sbb-wrapper/src/sbb_adapter.c`
- `interop/sbb-wrapper/src/sbb_adapter.h`
- `interop/sbb-wrapper/src/sbb_system_adapter.c`
- `interop/sbb-wrapper/tests/sbb_adapter_bridge_test.c`
- `interop/sbb-wrapper/CMakeLists.txt`

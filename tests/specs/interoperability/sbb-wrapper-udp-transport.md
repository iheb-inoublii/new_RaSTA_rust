# SBB Wrapper UDP Transport

## Objective

Verify the Step 8E wrapper UDP transport layer independently from full SBB protocol integration.

## Preconditions

- The Rust repository is available in Kali/Linux.
- CMake, Ninja, and a C compiler are available.
- SBB checkout exists at `/root/sbb-investigation/sbb-rasta-stack`.
- Step 8D wrapper skeleton build has passed.
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

Run the payload codec test:

```sh
./interop/sbb-wrapper/build/ping_pong_payload_test
```

Run the UDP loopback transport test:

```sh
./interop/sbb-wrapper/build/udp_transport_test
```

Run wrapper CLI smoke tests:

```sh
./interop/sbb-wrapper/build/sbb-rasta-wrapper passive 127.0.0.1 --rounds 3 --trace
./interop/sbb-wrapper/build/sbb-rasta-wrapper active 127.0.0.1 --rounds 3 --trace
```

## Expected result

- CMake configure succeeds.
- CMake build succeeds.
- `ping_pong_payload_test` passes.
- `udp_transport_test` opens two loopback UDP sockets, sends fixed bytes from channel 0 to channel 1, receives exact bytes, and verifies no-message behavior.
- Passive and active wrapper CLI smoke tests open channel 0 and channel 1 sockets.
- Trace output shows local/remote mapping and UDP send/receive/no-message activity.
- Wrapper CLI closes sockets cleanly before exit.
- No Rust-to-SBB interoperability is claimed.

## Current status

Implemented in wrapper source. Kali CMake validation is expected after this change is pushed or copied into the Kali checkout.

## Open points

- Verify `udp_transport_test` in Kali.
- Confirm whether SBB uses transport IDs beyond wrapper channels `0` and `1`.
- Confirm exact SBB adapter signatures before linking SBB libraries.
- Add bounded SafRetL queues if SBB adapter calls require asynchronous handoff.

## Evidence

- `interop/sbb-wrapper/src/udp_transport.c`
- `interop/sbb-wrapper/src/udp_transport.h`
- `interop/sbb-wrapper/src/sbb_adapter.c`
- `interop/sbb-wrapper/tests/udp_transport_test.c`
- `interop/sbb-wrapper/CMakeLists.txt`

# SBB Wrapper Skeleton

This directory contains a compile-ready skeleton for a future SBB RaSTA wrapper.
It does not implement Rust-to-SBB interoperability, does not add an `sbb-local`
Rust profile, and does not modify the external SBB checkout.

The current skeleton:

- accepts active/passive CLI settings
- prints deterministic parsed configuration
- defines required `sradin_*` and `redtri_*` symbols as logged stubs
- owns real POSIX UDP sockets for two wrapper transport channels
- returns `radef_kNoMessageReceived` when nonblocking UDP receive has no datagram
- provides a Ping/Pong payload codec matching Rust `ApplicationMessage`
- builds independently of SBB while accepting an `SBB_ROOT` CMake variable

## Build

```sh
cmake -S interop/sbb-wrapper -B interop/sbb-wrapper/build -G Ninja -DSBB_ROOT=/root/sbb-investigation/sbb-rasta-stack
cmake --build interop/sbb-wrapper/build
ctest --test-dir interop/sbb-wrapper/build
```

`SBB_ROOT` is recorded for the intended integration path, but this Step 8C
skeleton does not link SBB libraries yet.

## Step 8D Kali Verification

Intended Kali verification from the Rust repository root:

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

Verified Kali result:

- CMake configure passed with `SBB_ROOT=/root/sbb-investigation/sbb-rasta-stack`.
- CMake build passed.
- The build created `interop/sbb-wrapper/build/ping_pong_payload_test`.
- The build created `interop/sbb-wrapper/build/sbb-rasta-wrapper`.
- Passive smoke passed:
  `./interop/sbb-wrapper/build/sbb-rasta-wrapper passive 127.0.0.1 --rounds 3 --trace`
- Active smoke passed:
  `./interop/sbb-wrapper/build/sbb-rasta-wrapper active 127.0.0.1 --rounds 3 --trace`
- The wrapper clearly logs skeleton-only status and does not claim interoperability.
- UDP behavior remains stubbed.
- Send functions remain stubbed.
- Read functions return no message.

This result verifies that the Step 8D skeleton builds and runs smoke checks in
Kali. It is still not a Rust-to-SBB interoperability result.

## Step 8E UDP Transport

Step 8E replaces the UDP stub with real POSIX UDP sockets inside the wrapper.
This is still not SBB protocol integration and still not Rust-to-SBB
interoperability.

Implemented UDP behavior:

- two fixed transport channels
- one socket per channel
- bind each socket to the configured local port
- configure each socket as nonblocking
- send one complete datagram to the configured remote IP and port
- receive one complete datagram into the caller-provided buffer
- return no-message status for `EAGAIN` / `EWOULDBLOCK`
- report oversized datagrams instead of treating truncated data as valid
- close sockets on wrapper exit
- print deterministic trace logs when `--trace` is enabled

RedL adapter status:

- `redtri_Init` verifies that UDP was initialized.
- `redtri_SendMessage` sends through UDP by `transport_channel_id`.
- `redtri_ReadMessage` receives through UDP by `transport_channel_id`.
- `sradin_*` functions remain skeleton stubs until the SafRetL bridge is added.

UDP self-test command:

```sh
./interop/sbb-wrapper/build/udp_transport_test
```

## CLI

```sh
interop/sbb-wrapper/build/sbb-rasta-wrapper passive 127.0.0.1 --rounds 10 --run-seconds 40 --trace
interop/sbb-wrapper/build/sbb-rasta-wrapper active 127.0.0.1 --channel0-local 7100 --channel0-remote 7000
```

Default port mapping:

| Role | Channel | Local port | Remote port |
| --- | --- | --- | --- |
| passive | channel 0 | `7000` | `7100` |
| passive | channel 1 | `7001` | `7101` |
| active | channel 0 | `7100` | `7000` |
| active | channel 1 | `7101` | `7001` |

For now, the executable prints settings, opens two nonblocking UDP sockets, calls
stub initialization functions, runs read/send smoke checks, closes sockets, and
exits successfully. RedL send/read smoke checks use the real UDP wrapper
transport. SafRetL remains stubbed and the wrapper does not call SBB SafRetL
APIs.

## Ping/Pong Payload

The payload format matches `crates/rasta-core/src/application.rs`:

| Message | Bytes |
| --- | --- |
| Ping(counter) | `03 <counter little-endian u32>` |
| Pong(counter) | `04 <counter little-endian u32>` |

Total payload length is always five bytes.

## Stubbed Functions

SafRetL adapter stubs:

- `sradin_Init`
- `sradin_OpenRedundancyChannel`
- `sradin_CloseRedundancyChannel`
- `sradin_SendMessage`
- `sradin_ReadMessage`

RedL transport adapter:

- `redtri_Init`
- `redtri_SendMessage`
- `redtri_ReadMessage`

The RedL adapter functions now delegate to wrapper UDP transport.

The function signatures are local skeleton signatures until a later integration
step confirms the exact SBB headers and links against the real SBB modules.

## Remaining Step 8F Work

- Replace local skeleton function signatures only if SBB headers require it.
- Keep the wrapper outside Rust protocol code.
- Link external SBB libraries only after the exact include/library layout is confirmed.
- Connect SafRetL adapter calls to RedL/SBB expectations.
- Implement bounded queues if SBB requires asynchronous adapter reads.
- Preserve the current no-interop claim until a real SBB connection is observed.

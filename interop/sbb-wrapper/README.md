# SBB Wrapper Skeleton

This directory contains a compile-ready skeleton for a future SBB RaSTA wrapper.
It does not implement Rust-to-SBB interoperability, does not add an `sbb-local`
Rust profile, and does not modify the external SBB checkout.

The current skeleton:

- accepts active/passive CLI settings
- prints deterministic parsed configuration
- defines required `sradin_*` and `redtri_*` symbols with SBB-compatible signatures
- bridges `sradin_*` calls to SBB RedL when `SBB_ROOT` is provided
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

When `SBB_ROOT` points at the SBB checkout, the wrapper CMake embeds the SBB
`rasta_common` and `rasta_redundancy` modules and compiles the RedL bridge.

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
- At Step 8D, UDP behavior remained stubbed.
- At Step 8D, send functions remained stubbed.
- At Step 8D, read functions returned no message.

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
- At Step 8E, `sradin_*` functions remained skeleton stubs until the SafRetL bridge was added.

UDP self-test command:

```sh
./interop/sbb-wrapper/build/udp_transport_test
```

Verified Kali result:

- CMake configure passed.
- CMake build passed.
- `ping_pong_payload_test` passed.
- `udp_transport_test` passed.
- Passive wrapper smoke passed.
- Active wrapper smoke passed.
- UDP sockets opened and closed correctly.
- `redtri_SendMessage` sends through UDP and returns success.
- `redtri_ReadMessage` returns no message when the socket is empty.
- At Step 8E, `sradin_*` functions remained skeleton stubs.
- No Rust-to-SBB interoperability is claimed.

## Step 8F RedL Bridge

Step 8F connects the SafRetL adapter functions to SBB RedL public APIs inside
the wrapper. This is still an internal wrapper smoke path and is not
Rust-to-SBB interoperability.

SBB RedL functions used:

- `redint_Init`
- `redint_OpenRedundancyChannel`
- `redint_CloseRedundancyChannel`
- `redint_SendMessage`
- `redint_ReadMessage`
- `redint_CheckTimings`

SBB transport notification entry point inspected:

- `redtrn_MessageReceivedNotification`

The wrapper currently uses `redint_CheckTimings` before `redint_ReadMessage` so
RedL can poll pending transport messages. Direct notification wiring remains a
future option once the full SafRetL/SBB run loop is in place.

Bridge smoke test command:

```sh
./interop/sbb-wrapper/build/sbb_adapter_bridge_test
```

Kali link-error cleanup:

- Real SBB linking previously failed with undefined `rednot_MessageReceivedNotification`, `rednot_DiagnosticNotification`, `rasys_FatalError`, and `rasys_GetTimerValue`.
- The wrapper now implements the required RedL notification callbacks in `src/sbb_redundancy_notifications.c`.
- The wrapper implements SBB system adapter functions in `src/sbb_system_adapter.c`.
- `sbb_wrapper_common` is an object library so the adapter/callback objects are linked directly into `sbb-rasta-wrapper` and the wrapper test executables instead of being hidden behind a static archive.
- The validated Kali `SBB_ROOT` path for this phase is `$HOME/Desktop/sbb-investigation/sbb-rasta-stack`.

Verified Kali RedL bridge result:

- Real SBB libraries linked successfully after adding callback/system adapters.
- `ping_pong_payload_test` passed.
- `udp_transport_test` passed.
- `sbb_adapter_bridge_test` passed.
- `sbb_adapter_bridge_test` showed `sradin_Init` -> `redint_Init result=0`.
- Redundancy channel 0 opened with result `0`.
- `redtri_SendMessage` sent transport 0 and transport 1 datagrams with length `36`.
- `sradin_SendMessage` sent a 28-byte minimum SafRetL-like PDU through RedL with result `0`.
- `sradin_ReadMessage` returned `timing_result=0`, `read_result=1`, `length=0`.
- Redundancy channel 0 closed with result `0`.
- Passive and active wrapper CLI smoke passed.
- CLI smoke with the 5-byte dummy payload returns RedL result `17`, which is expected because it is not a valid/minimum SafRetL PDU.
- Runtime log says Step 8F SBB RedL bridge smoke only; no Rust-to-SBB interop is claimed.

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

For now, the executable logs Step 8F RedL bridge smoke status, prints settings,
opens two nonblocking UDP sockets, initializes RedL through `sradin_Init`, runs
read/send smoke checks, closes sockets, and exits successfully. It does not call
the SBB SafRetL API run loop and does not establish an SBB connection.

## Ping/Pong Payload

The payload format matches `crates/rasta-core/src/application.rs`:

| Message | Bytes |
| --- | --- |
| Ping(counter) | `03 <counter little-endian u32>` |
| Pong(counter) | `04 <counter little-endian u32>` |

Total payload length is always five bytes.

## Adapter Functions

SafRetL adapter bridge:

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

The function signatures have been aligned to the SBB public headers. If the SBB
checkout is not provided, the wrapper keeps fallback definitions for standalone
smoke builds.

## Remaining Step 8G Work

- Keep the wrapper outside Rust protocol code.
- Implement bounded queues if SBB requires asynchronous adapter reads.
- Integrate SBB SafRetL API calls: `srapi_Init`, `srapi_OpenConnection`, timing checks, send/read, and close.
- Run an SBB-to-SBB wrapper baseline.
- Preserve the current no-interop claim until a real SBB connection is observed.

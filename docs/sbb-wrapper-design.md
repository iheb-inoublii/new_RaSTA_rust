# SBB Wrapper Design Plan

This is a design plan only. It does not implement an SBB wrapper, add Docker, add `RastaProfile::sbb_local()`, or claim Rust-to-SBB interoperability.

## Why A Wrapper Is Needed

The SBB baseline investigation found that the SBB RaSTA stack builds and passes its unit tests, but the build output contains GoogleTest binaries only. No ready UDP client/server endpoint executable was found.

SBB also expects the integrator to provide adapter and transport functions. The core stack does not directly own UDP sockets or an executable application loop.

Required SafRetL adapter functions include:

- `sradin_Init`
- `sradin_OpenRedundancyChannel`
- `sradin_CloseRedundancyChannel`
- `sradin_SendMessage`
- `sradin_ReadMessage`

Required RedL transport functions include:

- `redtri_Init`
- `redtri_SendMessage`
- `redtri_ReadMessage`

Therefore, a small wrapper executable is needed before live Rust-to-SBB interoperability can be tested.

## Proposed Wrapper Architecture

The first wrapper should be a small C or C++ executable built alongside the SBB stack.

It should:

- link the SBB modules
- implement the required `sradin_*` adapter functions
- implement the required `redtri_*` transport functions
- own UDP sockets for local interop testing
- provide a simple CLI for active/passive roles
- implement a deterministic Ping/Pong application payload
- log state transitions, data, and errors

## Layer Mapping

```text
Application wrapper
  - CLI
  - ping/pong scenario
  - logging
  - run loop

SBB SafRetL API
  - srapi_Init
  - srapi_OpenConnection
  - srapi_SendData
  - srapi_ReadData
  - srapi_CloseConnection
  - srapi_GetConnectionState
  - timing check function

SafRetL adapter
  - sradin_* functions map connection/redundancy channel to RedL

SBB RedL
  - configured redundancy channels

RedL transport
  - redtri_* functions map transport_channel_id to UDP sockets
```

## Transport Mapping Proposal

For a first local wrapper, use one SBB redundancy channel pair for one connection and two UDP sockets for the two transport channels.

Example deterministic local mapping:

| Side | Channel | Local port | Remote port |
| --- | --- | --- | --- |
| SBB passive | channel 0 | `7000` | `7100` |
| SBB passive | channel 1 | `7001` | `7101` |
| Rust active | channel 0 | `7100` | `7000` |
| Rust active | channel 1 | `7101` | `7001` |

The exact mapping can change during implementation, but it must be documented and deterministic.

The SBB baseline RedL config uses two redundancy channels, with channel 0 transport IDs `{0, 1}` and channel 1 transport IDs `{2, 3}`. Step 8C must confirm the correct mapping between those IDs, the selected SBB connection, and UDP sockets.

## Role Mapping

Start with:

- SBB passive/server
- Rust active/client

This is safer for the first interop attempt because Rust active opening is already known to work with librasta. The SBB wrapper can initially focus on receiving a connection request, responding, and then handling Ping/Pong.

After that works, test the reverse:

- SBB active/client
- Rust passive/server

## Configuration Issue

SBB default values differ from `librasta-local`:

| Field | SBB value | librasta-local value |
| --- | --- | --- |
| network ID | `123456` | `1234` |
| `t_max` | `750 ms` | `10000 ms` |
| `t_h` | `300 ms` | `2000 ms` |
| connection 0 IDs | sender `0x61`, receiver `0x62` | `0x60`/`0x61` local setup |
| safety code | Lower MD4 | none |
| RedL check code | CheckCodeA / no check code | OptionA / no check code |

The Rust side will later need either:

- a dedicated `sbb-local` profile, or
- CLI override support for these values.

This step must not implement either option. Do not add `RastaProfile::sbb_local()` yet.

## Run Loop Design

Pseudo loop:

```text
initialize UDP sockets
initialize RedL/SafRetL
if active:
    call srapi_OpenConnection

while not stopped:
    poll UDP sockets into receive queues
    call SBB timing check function
    call srapi_ReadData
    if Ping received:
        send Pong with same counter
    if active and ready:
        send next Ping counter
    log state transitions and data
    stop after rounds or run_seconds

call srapi_CloseConnection
```

The exact SBB timing check function name must be confirmed from `srapi_sr_api.h` and related implementation files.

## Ping/Pong Message Compatibility

Use the same fixed-format application payload as Rust `ApplicationMessage`.

Current Rust format from `crates/rasta-core/src/application.rs`:

| Message | Bytes |
| --- | --- |
| `Ping { counter }` | tag `0x03`, then `counter` as little-endian `u32` |
| `Pong { counter }` | tag `0x04`, then `counter` as little-endian `u32` |

Total payload length is `5` bytes.

Step 8C should confirm exact payload bytes against `crates/rasta-core/src/application.rs` before implementing the SBB wrapper codec.

## Risks And Open Questions

- Exact SBB timing check function name must be confirmed from `srapi_sr_api.h` and implementation files.
- Mapping from RedL channels to SafRetL connection ID must be confirmed.
- It is unclear whether SBB expects an active/passive distinction or whether both sides call open.
- It is unclear whether SBB requires notification callbacks in addition to polling.
- Exact connection state enum values must be recorded.
- Timestamp behavior must be checked against Rust strict/local timestamp handling; peer-relative compatibility might be needed.
- Rust must eventually support SBB network ID `123456` and IDs `0x61`/`0x62` through a profile or CLI config path.
- Lower MD4 initial value and safety-code behavior must be verified with live SBB frames.

## Step 8C Skeleton Layout

The wrapper skeleton lives in `interop/sbb-wrapper/`.

Current files:

- `README.md`: skeleton status, build command, CLI examples, and stub list.
- `CMakeLists.txt`: standalone CMake project that accepts `SBB_ROOT` but does not link SBB yet and builds `sbb-rasta-wrapper`.
- `src/main.c`: active/passive CLI parser and deterministic settings logging.
- `src/sbb_adapter.h` / `src/sbb_adapter.c`: required `sradin_*` and `redtri_*` symbols as logged stubs.
- `src/udp_transport.h` / `src/udp_transport.c`: UDP configuration holder and logged no-socket initialization stub.
- `src/ping_pong_payload.h` / `src/ping_pong_payload.c`: Ping/Pong codec compatible with Rust `ApplicationMessage`.
- `tests/ping_pong_payload_test.c`: C codec smoke test.

The skeleton deliberately does not modify the Rust protocol implementation, add `RastaProfile::sbb_local()`, modify the external SBB checkout, or claim Rust-to-SBB interoperability.

## Step 8C Build Command

Intended local build:

```sh
cmake -S interop/sbb-wrapper -B interop/sbb-wrapper/build -G Ninja -DSBB_ROOT=/root/sbb-investigation/sbb-rasta-stack
cmake --build interop/sbb-wrapper/build
ctest --test-dir interop/sbb-wrapper/build
```

`SBB_ROOT` is accepted as a CMake cache variable for the future integration path. In Step 8C, it is informational only; no SBB include directories or libraries are consumed.

## Step 8D Verification Result

Requested Kali commands:

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

Actual Kali result:

- CMake configure passed with `SBB_ROOT=/root/sbb-investigation/sbb-rasta-stack`.
- CMake build passed.
- The build created `interop/sbb-wrapper/build/ping_pong_payload_test`.
- The build created `interop/sbb-wrapper/build/sbb-rasta-wrapper`.
- Passive smoke passed with `./interop/sbb-wrapper/build/sbb-rasta-wrapper passive 127.0.0.1 --rounds 3 --trace`.
- Active smoke passed with `./interop/sbb-wrapper/build/sbb-rasta-wrapper active 127.0.0.1 --rounds 3 --trace`.
- The wrapper logs skeleton-only status and does not claim Rust-to-SBB interoperability.
- UDP remains stubbed.
- Send functions remain stubbed.
- Read functions return no message.

Skeleton fixes made during Step 8D:

- Renamed the CMake executable target to `sbb-rasta-wrapper` to match the smoke-test command.
- Added explicit CLI smoke calls for send/read stubs so `radef_kNotImplemented` and `radef_kNoMessageReceived` behavior is visible in wrapper output.

## Step 8C Current Status

The wrapper is a compile-ready skeleton only.

Implemented:

- CLI parsing for `active` / `passive`, remote IP, `--rounds`, `--run-seconds`, `--trace`, and both channel local/remote ports.
- Default local port mapping for passive SBB (`7000/7001` local, `7100/7101` remote) and active SBB (`7100/7101` local, `7000/7001` remote).
- Logged stubs for `sradin_Init`, `sradin_OpenRedundancyChannel`, `sradin_CloseRedundancyChannel`, `sradin_SendMessage`, `sradin_ReadMessage`, `redtri_Init`, `redtri_SendMessage`, and `redtri_ReadMessage`.
- Read stubs return `radef_kNoMessageReceived` when no queue exists.
- Send stubs return `radef_kNotImplemented` rather than pretending to send data.
- CLI smoke runs print both send and read stub statuses before exiting successfully.
- Ping/Pong payload encoding and decoding using tag `0x03` / `0x04` plus little-endian `u32`.

Stubbed:

- UDP sockets are not opened yet.
- SBB SafRetL APIs are not called yet.
- SBB libraries are not linked yet.
- Exact SBB adapter function signatures still need confirmation against SBB headers.
- No connection, heartbeat, retransmission, or safety-code behavior is exercised.

## Step 8E Remaining Work

1. Confirm exact SBB function signatures and return-code names from the SBB headers.
2. Replace skeleton adapter signatures if needed to match SBB exactly.
3. Link the wrapper against the external SBB libraries using `SBB_ROOT`.
4. Implement real UDP socket ownership in the wrapper, still outside Rust `rasta-core`.
5. Implement bounded receive queues for `sradin_ReadMessage` and `redtri_ReadMessage`.
6. Call SBB initialization, timing, open, send, receive, state, and close APIs from the wrapper loop.
7. Run an SBB-to-SBB wrapper baseline before Rust-to-SBB.
8. Run Rust active to SBB passive with captured traces.
9. Only after live evidence, decide whether to add a Rust `sbb-local` profile or CLI config overrides.

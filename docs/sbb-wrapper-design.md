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

## Later Step 8C Implementation Plan

1. Create wrapper directory under `tools/sbb-wrapper` or `interop/sbb-wrapper`.
2. Add a CMake file that builds alongside the SBB stack.
3. Implement UDP transport ownership.
4. Implement required `redtri_*` functions.
5. Implement required `sradin_*` functions.
6. Implement active/passive CLI.
7. Implement Ping/Pong payload codec.
8. Build the SBB wrapper.
9. Run an SBB-to-SBB wrapper baseline first.
10. Run Rust-to-SBB with Rust active and SBB passive.
11. Capture traces and compare with Rust structured trace output.
12. Only after live evidence, decide whether to add `RastaProfile::sbb_local()`.

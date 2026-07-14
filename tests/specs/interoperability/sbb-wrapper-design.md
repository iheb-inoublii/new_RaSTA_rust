# SBB wrapper design

This specification records the original design phase. The wrapper and
controlled Rust-to-SBB campaign were subsequently completed; see
`docs/final-interop-summary.md`.

## Objective

Define the planned SBB wrapper architecture required for future Rust-to-SBB interoperability testing.

## Related requirement

Supervisor Step 8B: SBB wrapper design plan only.

## Preconditions

- SBB baseline investigation has been documented.
- SBB builds successfully and passes `24/24` CTest tests.
- No ready UDP SBB endpoint executable has been found.
- This step remains documentation/spec only.

## Design input

- SBB high-level API: `srapi_Init`, `srapi_OpenConnection`, `srapi_CloseConnection`, `srapi_SendData`, `srapi_ReadData`, `srapi_GetConnectionState`, and timing check function.
- Required SafRetL adapter functions: `sradin_Init`, `sradin_OpenRedundancyChannel`, `sradin_CloseRedundancyChannel`, `sradin_SendMessage`, `sradin_ReadMessage`.
- Required RedL transport functions: `redtri_Init`, `redtri_SendMessage`, `redtri_ReadMessage`.
- SBB RedL config: CheckCodeA/no check code, `t_seq = 50 ms`, two redundancy channels, transport IDs `{0,1}` and `{2,3}`.
- SBB SafRetL config: network ID `123456`, `t_max = 750 ms`, `t_h = 300 ms`, Lower MD4, `m_w_a = 10`, `n_send_max = 20`, connection 0 `0x61 -> 0x62`.

## Proposed architecture

A small C or C++ executable links the SBB modules, implements the required adapter/transport functions, owns UDP sockets, exposes active/passive CLI roles, and runs a Ping/Pong scenario compatible with Rust `ApplicationMessage`.

Layer order:

1. Application wrapper.
2. SBB SafRetL API.
3. SafRetL adapter `sradin_*`.
4. SBB RedL.
5. RedL transport `redtri_*`.
6. UDP sockets.

## Test setup for future implementation

Initial local mapping proposal:

| Side | Channel | Local port | Remote port |
| --- | --- | --- | --- |
| SBB passive | channel 0 | `7000` | `7100` |
| SBB passive | channel 1 | `7001` | `7101` |
| Rust active | channel 0 | `7100` | `7000` |
| Rust active | channel 1 | `7101` | `7001` |

First test Rust active to SBB passive. Later reverse the roles.

## Expected future result

The SBB wrapper can establish a connection with a Rust endpoint, exchange ordered Ping/Pong application messages, log state/data, and close gracefully.

## Current status

Design only. No wrapper has been implemented. No Rust-to-SBB interoperability is claimed.

## Open points

- Confirm exact SBB timing check function name.
- Confirm RedL channel to SafRetL connection mapping.
- Confirm active/passive connection-opening expectations.
- Confirm whether callbacks are required in addition to polling.
- Record exact SBB connection state enum values.
- Confirm timestamp compatibility behavior.
- Decide later whether Rust needs `sbb-local` profile or CLI overrides.

## Evidence

- `docs/sbb-baseline-investigation.md`
- `docs/sbb-wrapper-design.md`

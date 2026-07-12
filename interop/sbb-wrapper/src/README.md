# SBB wrapper source layout

The Step 8D wrapper skeleton is split across small C files instead of one large `sbb_rasta_wrapper.c` file. This keeps the experimental SBB interop artifact readable while preserving the planned responsibilities.

## Files

- `main.c`: CLI parsing, active/passive role selection, wrapper run loop, and smoke-test flow.
- `ping_pong_payload.c` / `ping_pong_payload.h`: fixed 5-byte Ping/Pong application payload codec compatible with Rust `ApplicationMessage`.
- `udp_transport.c` / `udp_transport.h`: wrapper-owned two-channel UDP transport. Step 8D began as a stub; later steps added real POSIX UDP sockets.
- `sbb_adapter.c` / `sbb_adapter.h`: SBB SafRetL adapter functions and RedL transport functions, including `sradin_*` and `redtri_*`.
- `sbb_system_adapter.c`: SBB system adapter functions such as timer, random, and fatal-error hooks.
- `sbb_redundancy_notifications.c`: RedL notification callbacks required when linking real SBB RedL.
- `sbb_safety_notifications.c`: SafRetL notification callbacks required when linking real SBB SafRetL.
- `sbb_endpoint.c` / `sbb_endpoint.h`: wrapper-only SafRetL endpoint/run-loop helper added after the initial skeleton.
- `sbb_diagnostics.c` / `sbb_diagnostics.h`: deterministic trace, return-code, state, and fatal diagnostic helpers.

## Skeleton boundary

This source tree is an experimental interoperability artifact. It must not modify Rust protocol behavior, modify the external SBB checkout, add Docker, or claim Rust-to-SBB interoperability by itself.

The Step 8D skeleton established the source locations and callable symbols needed by SBB:

- `redtri_Init`
- `redtri_SendMessage`
- `redtri_ReadMessage`
- `sradin_Init`
- `sradin_OpenRedundancyChannel`
- `sradin_CloseRedundancyChannel`
- `sradin_SendMessage`
- `sradin_ReadMessage`
- `rasys_GetTimerValue`
- `rasys_GetTimerGranularity`
- `rasys_GetRandomNumber`
- `rasys_FatalError`

# SBB wrapper plan

## Objective
Define the documentation-only plan for a small SBB wrapper executable that can later be used for Rust-to-SBB interoperability testing.

## Related requirement
Step 8B SBB wrapper design documentation and integration plan.

## Preconditions
The Step 8A SBB baseline investigation exists. SBB builds with CMake/Ninja, CTest passes `24/24`, no ready UDP client/server demo endpoint was found, and SBB requires integrator-provided adapter and transport functions.

## Test setup
No executable test is run in this planning step. Review `docs/sbb-wrapper-design.md`, `docs/sbb-baseline-investigation.md`, and this formal spec for completeness.

## Proposed commands
Future wrapper commands proposed by the plan:

```sh
sbb-rasta-wrapper active <remote_ip> --rounds 10 --run-seconds 30 --trace
sbb-rasta-wrapper passive <remote_ip> --rounds 10 --run-seconds 30 --trace
```

## Expected result
The design documents explain why a wrapper is needed, which SBB APIs and adapter functions must be integrated, how active/passive roles should work, how UDP transport channels should be mapped, and which trace evidence must be collected.

## Evidence to collect
- role and selected connection IDs
- local and remote UDP ports
- connection state changes
- TX/RX RedL frame lengths
- SafRetL message type or return code when accessible
- Ping/Pong counters
- adapter, transport, and SBB return codes
- packet lengths needed before any Rust-to-SBB profile or interoperability claim

## Postconditions
No Rust protocol behavior is changed. No SBB C wrapper source code is required by this planning spec. No Docker setup or Rust-to-SBB interoperability claim is made.

## Automation status
Documentation/spec review only. Later steps add wrapper source, wrapper build tests, SBB-to-SBB baseline tests, and Rust-to-SBB preparation tests.

## Skeleton status
Step 8D adds the wrapper skeleton under `interop/sbb-wrapper`. The current source is split into `main.c`, `sbb_adapter.c`, `sbb_system_adapter.c`, `udp_transport.c`, `ping_pong_payload.c`, endpoint helpers, and notification helpers rather than one monolithic C file. This structure provides the planned `sradin_*`, `redtri_*`, and `rasys_*` entry points while keeping the artifact isolated from Rust protocol code.

Compile verification is tracked separately by the wrapper build and skeleton specs. Live Rust-to-SBB interoperability remains pending until a real Rust/SBB live test passes.

## Open points
- Confirm the exact SBB timing/polling function from `srapi_sr_api.h`.
- Verify passive open/listen behavior.
- Map RedL initialization and open calls exactly.
- Observe timestamp behavior live.
- Capture packet lengths before claiming profile compatibility.

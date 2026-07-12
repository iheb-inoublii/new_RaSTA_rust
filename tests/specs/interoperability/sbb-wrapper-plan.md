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
No Rust protocol behavior is changed. No Docker setup or Rust-to-SBB interoperability claim is made.

## Step 8F status
- Skeleton compile with real SBB: passed with `SBB_ROOT=/home/iheb/Desktop/sbb-investigation/sbb-rasta-stack`.
- Real SBB libraries linked: `librasta_common.a`, `librasta_redundancy.a`, and `librasta_safety_retransmission.a`.
- Wrapper smoke tests: passed for payload codec, UDP transport, RedL bridge, transport notification, SafRetL smoke, and wrapper help.

## Step 8I status
- SBB-to-SBB `Up` baseline: passed before this step.
- Previous gap: passive stopped after `Up` and heartbeat, so active Ping messages were not answered with Pong replies.
- Runtime change: passive remains alive after `Up`, decodes Ping counters, sends matching Pong counters, and exits successfully only after all requested rounds are answered.
- Active runtime change: active waits for all expected Pong counters before reporting success.
- Live SBB active/passive Ping/Pong run: pending Kali verification after rebuild.
- Rust-to-SBB interoperability: pending; no success claim is made.

## Automation status
Documentation/spec review only. Later steps add wrapper source, wrapper build tests, SBB-to-SBB baseline tests, and Rust-to-SBB preparation tests.

## Open points
- Verify passive/active Ping/Pong runtime in Kali.
- Observe timestamp behavior live.
- Capture packet lengths before claiming Rust-to-SBB compatibility.

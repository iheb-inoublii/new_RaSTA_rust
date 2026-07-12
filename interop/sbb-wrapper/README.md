# SBB Wrapper Skeleton

This directory contains a compile-ready skeleton for a future SBB RaSTA wrapper.
It does not implement Rust-to-SBB interoperability, does not add an `sbb-local`
Rust profile, and does not modify the external SBB checkout.

The current skeleton:

- accepts active/passive CLI settings
- prints deterministic parsed configuration
- defines required `sradin_*` and `redtri_*` symbols with SBB-compatible signatures
- bridges `sradin_*` calls to SBB RedL when `SBB_ROOT` is provided
- links SBB SafRetL when `SBB_ROOT` is provided and runs a smoke-only `srapi_*` loop
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
`rasta_common`, `rasta_redundancy`, and `rasta_safety_retransmission` modules
and compiles the RedL/SafRetL smoke bridge.

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

Step 8F used `redint_CheckTimings` before `redint_ReadMessage` as the first
RedL smoke path. Step 8H supersedes that receive path by polling UDP into fixed
pending slots and invoking `redtrn_MessageReceivedNotification`.

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

## Step 8G SafRetL Run Loop

Step 8G links the SBB SafRetL module and adds a wrapper-only endpoint layer
around the public SBB SafRetL API. This is still a smoke path inside the SBB
wrapper and is not Rust-to-SBB interoperability.

SBB SafRetL functions used:

- `srapi_Init`
- `srapi_OpenConnection`
- `srapi_CheckTimings`
- `srapi_GetConnectionState`
- `srapi_SendData`
- `srapi_ReadData`
- `srapi_CloseConnection`

Wrapper-side SafRetL notification callbacks implemented:

- `srnot_MessageReceivedNotification`
- `srnot_ConnectionStateNotification`
- `srnot_SrDiagnosticNotification`
- `srnot_RedDiagnosticNotification`

Smoke configuration:

- network ID: `123456`
- connection 0 sender ID: `0x61`
- connection 0 receiver ID: `0x62`
- `t_max`: `750 ms`
- `t_h`: `300 ms`
- safety code: Lower MD4
- `m_w_a`: `10`
- `n_send_max`: `20`
- `n_max_packet`: `1`

New smoke test command:

```sh
./interop/sbb-wrapper/build/sbb_safretl_smoke_test
```

Expected Kali validation commands:

```sh
cmake -S interop/sbb-wrapper -B interop/sbb-wrapper/build -G Ninja -DSBB_ROOT=$HOME/Desktop/sbb-investigation/sbb-rasta-stack
cmake --build interop/sbb-wrapper/build
./interop/sbb-wrapper/build/ping_pong_payload_test
./interop/sbb-wrapper/build/udp_transport_test
./interop/sbb-wrapper/build/sbb_adapter_bridge_test
./interop/sbb-wrapper/build/sbb_safretl_smoke_test
./interop/sbb-wrapper/build/sbb_transport_notification_test
./interop/sbb-wrapper/build/sbb-rasta-wrapper passive 127.0.0.1 --rounds 3 --trace --run-seconds 5
./interop/sbb-wrapper/build/sbb-rasta-wrapper active 127.0.0.1 --rounds 3 --trace --run-seconds 5
```

Verified Kali SafRetL run-loop result:

- Real SBB_ROOT was used: `-DSBB_ROOT=$HOME/Desktop/sbb-investigation/sbb-rasta-stack`.
- CMake configure passed.
- CMake build passed.
- The build created `ping_pong_payload_test`, `udp_transport_test`, `sbb_adapter_bridge_test`, `sbb_safretl_smoke_test`, and `sbb-rasta-wrapper`.
- `ping_pong_payload_test` passed.
- `udp_transport_test` passed.
- `sbb_adapter_bridge_test` passed.
- `sbb_safretl_smoke_test` passed.
- `sbb_safretl_smoke_test` showed that the `srapi_Init` path works.
- During SafRetL smoke, the RedL bridge sent UDP frames with lengths `58` and `48`.
- During SafRetL smoke, `sradin_SendMessage` sent lengths `50` and `40` with result `0`.
- Passive single-process smoke reported `srapi_Init result=0`, stayed `Closed` because no active peer was running, and shut down cleanly.
- Active single-process smoke reported `srapi_Init result=0` and `srapi_OpenConnection result=0`, sent length `58` and `48` frames, moved `Start` then `Closed` because no passive peer was running at the same time, and shut down cleanly.
- Runtime log now says Step 8H SBB-to-SBB baseline smoke only; no Rust-to-SBB interop is claimed.

## Step 8H Receive Notification Path

The first concurrent SBB-to-SBB baseline attempt showed that the passive wrapper
stayed `Closed` for 30 seconds and did not log any UDP receive,
`redtri_ReadMessage`, or `redtrn_MessageReceivedNotification` activity.

A later tcpdump run clarified packet direction:

```text
127.0.0.1.7000 > 127.0.0.1.7100 length 58
127.0.0.1.7001 > 127.0.0.1.7101 length 58
127.0.0.1.7000 > 127.0.0.1.7100 length 48
127.0.0.1.7001 > 127.0.0.1.7101 length 48
```

Observed failed attempt:

- passive sent UDP frames from `7000/7001` to active `7100/7101`
- expected initial direction was active `7100/7101` to passive `7000/7001`
- passive stayed `Closed` or did not progress to `Up`
- passive did not process incoming UDP into RedL/SafRetL

Root cause:

- SBB RedL expects the transport layer to call
  `redtrn_MessageReceivedNotification(transport_channel_id)` after a datagram
  arrives.
- `redtrn_MessageReceivedNotification` then calls `redtri_ReadMessage`.
- The wrapper previously read UDP directly from `redtri_ReadMessage`, so RedL
  was never notified that incoming transport data existed.
- The wrapper also inverted SBB SafRetL role IDs. SBB's
  `srcor_IsConnRoleServer` treats `sender_id > receiver_id` as server/passive,
  so the passive wrapper had been configured as the client and sent the initial
  frames.

Implemented wrapper fix:

- poll UDP sockets during the SafRetL run loop
- store each received datagram in a fixed pending slot per transport channel
- call `redtrn_MessageReceivedNotification` after pending data is available
- make `redtri_ReadMessage` consume the pending datagram instead of reading the
  socket directly
- keep no-message behavior after the pending datagram is consumed
- configure active as client with `0x61 -> 0x62`
- configure passive as server/listener with `0x62 -> 0x61`
- keep passive `srapi_OpenConnection`, because SBB state-machine tests show a
  server open event moves the connection to `Down`/listen without sending
  `ConnReq`

Connection matching finding:

- `srapi_OpenConnection` resolves a static connection by exact `sender_id`,
  `receiver_id`, and network.
- SBB source inspection showed `srcor_GetConnectionId` exact sender/receiver
  matching and `srcor_IsConnRoleServer` treating `sender_id > receiver_id` as
  server/passive.
- Incoming SafRetL frames are checked against the reversed peer tuple.
- Two wrapper processes cannot connect as `0x61 <-> 0x62` if both use only the
  same static `0x61 -> 0x62` entry.
- The wrapper therefore keeps role-local configs: active `0x61 -> 0x62`,
  passive `0x62 -> 0x61`. This is local to `interop/sbb-wrapper`; the external
  SBB checkout is not modified.

Follow-up Kali result after direction/receive fixes:

- tcpdump showed active `7100/7101` to passive `7000/7001` for length `58` and
  `48` frames.
- passive logged UDP receive activity on both channels
- passive logged transport polling into pending slots
- `redtri_ReadMessage` consumed pending datagrams
- `redtrn_MessageReceivedNotification` was invoked
- passive still remained `Down`

Newly identified blocker:

- `rednot_MessageReceivedNotification` is the RedL-to-SafRetL adapter callback.
- The wrapper had only logged this callback.
- SBB expects this callback to forward into
  `sradno_MessageReceivedNotification(red_channel_id)`.

Additional fix:

- `rednot_MessageReceivedNotification` now calls `sradno_MessageReceivedNotification`.
- `rednot_DiagnosticNotification` now calls `sradno_DiagnosticNotification`.
- trace logs include `sradno_*` return codes.
- received RedL frame trace includes datagram length, RedL length, SafRetL
  length, SafRetL message type from fixed offsets, and a short hex prefix.
- endpoint trace logs include `srapi_OpenConnection`, `srapi_CheckTimings`,
  `srapi_GetConnectionState`, and `srapi_ReadData` return codes.
- SafRetL notification logs include state names, disconnect reason values, and
  safety/address/type/SN/CSN diagnostic counters.

Current Kali result:

- Both active and passive SBB wrapper processes reach `Up`.
- Passive logs `srapi_GetConnectionState: connection=0 result=0 state=Up`.
- Passive receives a heartbeat frame with `sr_type=0x184c(Heartbeat)`.
- Passive receives a disconnect request with `sr_type=0x1848(DiscReq)`.
- Passive transitions to `Closed` through
  `srnot_ConnectionStateNotification connection=0 state=1(Closed)`.
- Active already exits cleanly with `connection closed after Up; graceful
  SBB-to-SBB smoke complete` and skips `srapi_CloseConnection` because the
  connection is already closed after `Up`.
- This proves the SBB-to-SBB baseline connection works.

Post-disconnect fix:

- The remaining issue was continued RedL/SafRetL polling after SBB had closed
  the channel, which could trigger `rasys_FatalError` with
  `InvalidParameter` or `InternalError` in
  `sradin_ReadMessage:redint_ReadMessage`.
- The wrapper now records `Up` and `Closed after Up` both from endpoint state
  polling and directly from `srnot_ConnectionStateNotification`.
- This matters for passive, where `DiscReq` can close the connection while the
  wrapper is still inside the RedL/SafRetL notification callback stack.
- Once the global `Closed after Up` latch is set, `sradin_ReadMessage` returns
  no-message instead of entering `redint_ReadMessage`.
- Transport polling also stops notifying RedL once the latch is set.
- Once `Closed` after `Up` is observed, the run loop exits before calling
  `srapi_ReadData`, before another poll cycle, and before any further
  `sradin_ReadMessage` / `redint_ReadMessage` path.
- `srapi_CloseConnection` is skipped if the connection already closed after
  reaching `Up`.
- Step 8H success condition is `Up`, heartbeat, `DiscReq`/`Closed`, and no
  `rasys_FatalError` in the normal run.

Remaining Up-state fatal and fix:

- Kali then showed passive could still hit `rasys_FatalError
  reason=6(InvalidParameter)` while still `Up`, before the later `DiscReq`.
- The fatal phase was `sradin_ReadMessage:redint_ReadMessage`.
- The suspicious pattern was `sradin_ReadMessage` entering
  `redint_ReadMessage` even when no RedL upper-layer message notification was
  active.
- The wrapper now tracks RedL channel-open state and a one-shot read allowance
  per redundancy channel.
- `rednot_MessageReceivedNotification` grants the allowance while forwarding to
  `sradno_MessageReceivedNotification`.
- `sradin_ReadMessage` calls `redint_ReadMessage` only when the RedL channel is
  open, the connection is not closed after `Up`, and a RedL message
  notification is currently active.
- Poll-style SafRetL calls to `sradin_ReadMessage` outside that notification
  path return `radef_kNoMessageReceived` without touching RedL.

DiscReq notification re-entrancy fix:

- Kali then showed the final normal-run abort happened while passive handled
  `sr_type=0x1848(DiscReq)`.
- SBB closed RedL/SafRetL during that notification, but the same callback stack
  could still re-enter `sradin_ReadMessage` / `redint_ReadMessage`.
- The wrapper now marks `DiscReq` before calling
  `redtrn_MessageReceivedNotification`.
- During a DiscReq notification, only the first valid `redint_ReadMessage` is
  allowed; the read is marked consumed before entering RedL so re-entrant reads
  return `radef_kNoMessageReceived`.
- If `Closed after Up` is observed while forwarding
  `rednot_MessageReceivedNotification`, the wrapper stops the notification path
  safely and does not call further RedL/SafRetL operations.
- Expected normal-run Step 8H result remains `Up`, heartbeat,
  `DiscReq`/`Closed`, graceful smoke completion, and no `rasys_FatalError`.

Final Step 8H passive smoke boundary before Ping/Pong runtime work:

- Step 8H proof requires SBB-to-SBB reaching `Up` and exchanging heartbeat.
- Active clean close is verified separately and remains unchanged.
- Passive now exits after it has reached `Up` and received at least one
  heartbeat, before waiting for active `DiscReq`.
- This avoids the unstable SBB post-smoke shutdown path while preserving the
  useful SBB-to-SBB baseline evidence.
- Passive logs `passive observed Up and heartbeat; SBB-to-SBB smoke complete`,
  `passive smoke success condition reached`, `stopping SafRetL/RedL polling`,
  skips SafRetL close because smoke is complete, and exits the baseline smoke.

Step 8I replaces that early passive exit for Ping/Pong runs:

- Passive stays alive after `Up` and heartbeat.
- Passive reads Ping payloads, logs `received Ping(counter)`, sends matching
  Pong payloads, and logs `sent Pong(counter)`.
- Active sends `Ping(1)..Ping(N)` and waits for `Pong(1)..Pong(N)`.
- Runtime summaries report `success=true` only after all requested rounds
  complete.

Fatal diagnostics added:

- `rasys_FatalError` now logs `SBB rasys_FatalError called` before aborting.
- The fatal log includes numeric and symbolic return code, role, connection ID,
  sender ID, receiver ID, current wrapper phase, and whether diagnostic
  no-abort mode is enabled.
- stdout and stderr are flushed before the default abort.
- `--debug-no-abort` records the fatal and lets the wrapper exit after the
  current poll/read path. This is diagnostic only and must not be used to claim
  success.
- received datagram logs now include source endpoint, first bytes, RedL length,
  SafRetL length, and decoded SafRetL message type before
  `redtrn_MessageReceivedNotification` is called.

New transport notification smoke test:

```sh
./interop/sbb-wrapper/build/sbb_transport_notification_test
```

Expected concurrent SBB-to-SBB baseline command:

```sh
rm -f /tmp/sbb-passive.log /tmp/sbb-active.log
./interop/sbb-wrapper/build/sbb-rasta-wrapper passive 127.0.0.1 --rounds 3 --trace --run-seconds 30 > /tmp/sbb-passive.log 2>&1 &
sleep 1
./interop/sbb-wrapper/build/sbb-rasta-wrapper active 127.0.0.1 --rounds 3 --trace --run-seconds 30 > /tmp/sbb-active.log 2>&1
cat /tmp/sbb-passive.log
cat /tmp/sbb-active.log
```

Diagnostic no-abort variant:

```sh
rm -f /tmp/sbb-passive.log /tmp/sbb-active.log
./interop/sbb-wrapper/build/sbb-rasta-wrapper passive 127.0.0.1 --rounds 3 --trace --debug-no-abort --run-seconds 30 > /tmp/sbb-passive.log 2>&1 &
sleep 1
./interop/sbb-wrapper/build/sbb-rasta-wrapper active 127.0.0.1 --rounds 3 --trace --debug-no-abort --run-seconds 30 > /tmp/sbb-active.log 2>&1
cat /tmp/sbb-passive.log
cat /tmp/sbb-active.log
```

This remains SBB-wrapper-only and still does not claim Rust-to-SBB
interoperability.

The CLI runtime log now says:

```text
Step 8I SBB-to-SBB Ping/Pong runtime smoke only; no Rust-to-SBB interop is claimed
```

## CLI

```sh
interop/sbb-wrapper/build/sbb-rasta-wrapper passive 127.0.0.1 --rounds 10 --run-seconds 40 --trace
interop/sbb-wrapper/build/sbb-rasta-wrapper active 127.0.0.1 --channel0-local 7100 --channel0-remote 7000
```

Use `--debug-no-abort` only for diagnosing an SBB `rasys_FatalError`; default
behavior still aborts to preserve SBB safety semantics.

Default port mapping:

| Role | Channel | Local port | Remote port |
| --- | --- | --- | --- |
| passive | channel 0 | `7000` | `7100` |
| passive | channel 1 | `7001` | `7101` |
| active | channel 0 | `7100` | `7000` |
| active | channel 1 | `7101` | `7001` |

For now, the executable logs Step 8I SBB-to-SBB Ping/Pong runtime smoke status,
prints settings, opens two nonblocking UDP sockets, initializes the RedL adapter
and SafRetL, runs `srapi_CheckTimings`/state/read polling for `--run-seconds`,
and closes sockets. Both roles call `srapi_OpenConnection`: active uses the
client ID ordering `0x61 -> 0x62` and passive uses the server/listening ID
ordering `0x62 -> 0x61`. Startup logs print the selected connection ID, sender
ID, receiver ID, network ID, and whether `srapi_OpenConnection` is called.
Active sends Ping payloads only after SafRetL reports `Up`; passive replies
with matching Pong payloads. This still does not establish or claim Rust-to-SBB
interoperability.

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
- `redtrn_MessageReceivedNotification`

The RedL send path delegates to wrapper UDP transport. The receive path polls
UDP into a fixed pending transport slot, notifies RedL with
`redtrn_MessageReceivedNotification`, and lets `redtri_ReadMessage` consume the
pending datagram when RedL asks for it. RedL then notifies SafRetL through
`rednot_MessageReceivedNotification`, which the wrapper forwards to
`sradno_MessageReceivedNotification`.

The function signatures have been aligned to the SBB public headers. If the SBB
checkout is not provided, the wrapper keeps fallback definitions for standalone
smoke builds.

## Remaining Work After Step 8H

- Keep the wrapper outside Rust protocol code.
- Verify in Kali that the post-disconnect polling fix prevents `rasys_FatalError` in the normal SBB-to-SBB baseline run.
- Preserve the current no Rust-to-SBB interoperability claim.

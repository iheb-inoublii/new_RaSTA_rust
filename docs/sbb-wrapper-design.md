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
- Step 8H confirmed SBB RedL requires transport notification through `redtrn_MessageReceivedNotification` after UDP receive.
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
- `src/sbb_adapter.h` / `src/sbb_adapter.c`: required `sradin_*` symbols connected to SBB RedL when `SBB_ROOT` is provided, plus `redtri_*` functions connected to wrapper UDP transport.
- `src/udp_transport.h` / `src/udp_transport.c`: two-channel POSIX UDP transport for Kali/Linux.
- `src/ping_pong_payload.h` / `src/ping_pong_payload.c`: Ping/Pong codec compatible with Rust `ApplicationMessage`.
- `tests/ping_pong_payload_test.c`: C codec smoke test.
- `tests/udp_transport_test.c`: loopback UDP datagram and no-message smoke test.

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
- At Step 8D, UDP remained stubbed.
- At Step 8D, send functions remained stubbed.
- At Step 8D, read functions returned no message.

Skeleton fixes made during Step 8D:

- Renamed the CMake executable target to `sbb-rasta-wrapper` to match the smoke-test command.
- Added explicit CLI smoke calls for the Step 8D send/read stubs so no-message behavior was visible in wrapper output.

## Step 8E UDP Transport

Step 8E adds real POSIX UDP transport inside `interop/sbb-wrapper` only. It does
not modify Rust protocol behavior, add `RastaProfile::sbb_local()`, implement
Docker, link SBB libraries, or claim Rust-to-SBB interoperability.

UDP design:

- The wrapper has two deterministic transport channels.
- Each channel owns one nonblocking UDP socket.
- Each channel is bound to its configured local port.
- Each channel sends complete datagrams to the configured remote IP and remote port.
- Receive reads one datagram into a caller-provided fixed buffer.
- Empty nonblocking receive maps to no-message.
- Oversized datagrams are reported and discarded as invalid for the caller buffer.
- No heap allocation or threads are used.

RedL adapter connection:

- `redtri_Init` checks that UDP is initialized.
- `redtri_SendMessage` delegates to `sbb_wrapper_udp_send`.
- `redtri_ReadMessage` delegates to `sbb_wrapper_udp_receive`.
- At Step 8E, `sradin_*` functions still remained SafRetL skeleton stubs.

New wrapper test:

```sh
./interop/sbb-wrapper/build/udp_transport_test
```

The test opens two loopback channels, sends fixed bytes from channel 0 to channel
1, verifies exact receive bytes, and verifies no-message behavior on an empty
socket.

Actual Kali result:

- CMake configure passed.
- CMake build passed.
- `ping_pong_payload_test` passed.
- `udp_transport_test` passed.
- Passive wrapper smoke passed.
- Active wrapper smoke passed.
- UDP sockets opened and closed correctly.
- `redtri_SendMessage` sends through UDP and returns success.
- `redtri_ReadMessage` returns no message when empty.
- At Step 8E, `sradin_*` remained skeleton/stubbed.
- No Rust-to-SBB interoperability is claimed.

## Current Status

The wrapper is a compile-ready smoke harness with real UDP transport, an
internal SafRetL adapter to SBB RedL bridge, and a Step 8G SafRetL public API
run loop.

Implemented:

- CLI parsing for `active` / `passive`, remote IP, `--rounds`, `--run-seconds`, `--trace`, and both channel local/remote ports.
- Default local port mapping for passive SBB (`7000/7001` local, `7100/7101` remote) and active SBB (`7100/7101` local, `7000/7001` remote).
- SBB-compatible signatures for `sradin_*` and `redtri_*`.
- Internal bridge from `sradin_*` to SBB RedL public APIs.
- Real UDP-backed behavior for `redtri_Init`, `redtri_SendMessage`, and `redtri_ReadMessage`.
- `sradin_ReadMessage` returns `radef_kNoMessageReceived` when RedL has no queued message.
- SBB SafRetL public API calls for init, active open, timing checks, state reads, application send/read, and close.
- CLI smoke opens sockets, initializes RedL/SafRetL, polls UDP and SafRetL for `--run-seconds`, and closes sockets before exiting.
- UDP receive data is stored in a fixed pending transport slot before RedL is notified with `redtrn_MessageReceivedNotification`.
- Ping/Pong payload encoding and decoding using tag `0x03` / `0x04` plus little-endian `u32`.

Stubbed:

- Full SBB-to-SBB and Rust-to-SBB scenarios are not implemented yet.
- No Rust `sbb-local` profile exists yet.
- No Rust-to-SBB interoperability is claimed.

## Step 8F RedL Bridge

The wrapper uses these exact SBB RedL public functions from
`rasta_redundancy/redint_red_interface.h`:

- `redint_Init`
- `redint_OpenRedundancyChannel`
- `redint_CloseRedundancyChannel`
- `redint_SendMessage`
- `redint_ReadMessage`
- `redint_CheckTimings`

The transport notification entry point
`redtrn_MessageReceivedNotification` was inspected in
`rasta_redundancy/redtrn_transport_notifications.h`. Step 8F used
`redint_CheckTimings` for the first RedL smoke path; Step 8H adds the real
transport notification receive path.

New bridge test:

```sh
./interop/sbb-wrapper/build/sbb_adapter_bridge_test
```

The test initializes wrapper UDP, initializes the RedL bridge, opens redundancy
channel 0, sends a deterministic minimum-size SafRetL-like buffer through
`sradin_SendMessage`, verifies the read path returns either no-message or a
successful read without crashing, closes the channel, and closes UDP.

### Link-Error Cleanup

Kali configure found the real SBB checkout at
`$HOME/Desktop/sbb-investigation/sbb-rasta-stack`, but the first real RedL link
failed because SBB requires integrator-provided notification and system adapter
symbols.

Implemented wrapper-side callbacks:

- `rednot_MessageReceivedNotification`
- `rednot_DiagnosticNotification`

Implemented wrapper-side system adapter functions:

- `rasys_GetTimerValue`
- `rasys_GetTimerGranularity`
- `rasys_GetRandomNumber`
- `rasys_FatalError`

The wrapper common target is now an object library so these integration objects
are linked directly into `sbb-rasta-wrapper`, `sbb_adapter_bridge_test`, and the
other wrapper smoke tests when `SBB_ROOT` is set.

### Kali Validation Result

The real Kali validation used:

```sh
cmake -S interop/sbb-wrapper -B interop/sbb-wrapper/build -G Ninja -DSBB_ROOT=$HOME/Desktop/sbb-investigation/sbb-rasta-stack
cmake --build interop/sbb-wrapper/build
```

Result:

- Real SBB libraries linked successfully after adding callback/system adapters.
- `ping_pong_payload_test` passed.
- `udp_transport_test` passed.
- `sbb_adapter_bridge_test` passed.
- `sradin_Init` called `redint_Init` with result `0`.
- `sradin_OpenRedundancyChannel` opened channel 0 with result `0`.
- `redtri_SendMessage` sent transport 0 and transport 1 datagrams with length `36`.
- `sradin_SendMessage` sent a 28-byte minimum SafRetL-like PDU with `redint_SendMessage result=0`.
- `sradin_ReadMessage` returned `timing_result=0`, `read_result=1`, and `length=0`.
- `sradin_CloseRedundancyChannel` closed channel 0 with result `0`.
- Passive and active wrapper CLI smoke passed.
- CLI smoke with the 5-byte dummy payload returns RedL result `17`, expected because it is not a valid/minimum SafRetL PDU.
- No Rust-to-SBB interoperability is claimed.

## Step 8G SafRetL Run Loop

Step 8G adds `src/sbb_endpoint.h` and `src/sbb_endpoint.c` as a small
wrapper-only layer around SBB SafRetL. It does not change Rust protocol code,
add Docker, add a Rust `sbb-local` profile, or claim Rust-to-SBB
interoperability.

Exact SBB SafRetL public functions used from `srapi_sr_api.h`:

- `srapi_Init`
- `srapi_OpenConnection`
- `srapi_CheckTimings`
- `srapi_GetConnectionState`
- `srapi_SendData`
- `srapi_ReadData`
- `srapi_CloseConnection`

Exact app-side SafRetL notifications implemented from
`srnot_sr_notifications.h`:

- `srnot_MessageReceivedNotification`
- `srnot_ConnectionStateNotification`
- `srnot_SrDiagnosticNotification`
- `srnot_RedDiagnosticNotification`

Smoke configuration comes from SBB test defaults:

| Field | Value |
| --- | --- |
| network ID | `123456` |
| connection 0 sender ID | `0x61` |
| connection 0 receiver ID | `0x62` |
| `t_max` | `750 ms` |
| `t_h` | `300 ms` |
| safety code | Lower MD4 |
| `m_w_a` | `10` |
| `n_send_max` | `20` |
| `n_max_packet` | `1` |

The CLI runtime banner is:

```text
Step 8G SBB SafRetL run-loop smoke only; no Rust-to-SBB interop is claimed
```

Active mode calls `srapi_OpenConnection`. Both active and passive modes poll
`srapi_CheckTimings`, read state through `srapi_GetConnectionState`, call
`srapi_ReadData`, and close via `srapi_CloseConnection`. Ping payloads are sent
only after SafRetL reports `Up`.

New smoke test:

```sh
./interop/sbb-wrapper/build/sbb_safretl_smoke_test
```

## Step 8H SBB-To-SBB Receive Path

The first concurrent SBB-to-SBB baseline failed in a useful way: passive stayed
`Closed` and never logged UDP receive, `redtri_ReadMessage`, or
`redtrn_MessageReceivedNotification`.

Follow-up tcpdump evidence showed the packet direction was wrong:

```text
127.0.0.1.7000 > 127.0.0.1.7100 length 58
127.0.0.1.7001 > 127.0.0.1.7101 length 58
127.0.0.1.7000 > 127.0.0.1.7100 length 48
127.0.0.1.7001 > 127.0.0.1.7101 length 48
```

The passive process was sending from ports `7000/7001` to active ports
`7100/7101`; the expected initial direction is active `7100/7101` to passive
`7000/7001`.

Root cause:

- SBB RedL's transport notification API is
  `redtrn_MessageReceivedNotification(const uint32_t transport_channel_id)`.
- That notification reads the datagram through `redtri_ReadMessage`.
- The wrapper previously waited for `redtri_ReadMessage` before polling UDP, so
  RedL never learned that transport data was available.
- The wrapper had active/passive SafRetL IDs inverted. SBB tests and
  `srcor_IsConnRoleServer` show that `sender_id > receiver_id` means server.
  Therefore active must use `0x61 -> 0x62`, while passive/server uses
  `0x62 -> 0x61`.

Step 8H changes the wrapper receive path:

1. Poll each UDP socket during the SafRetL run loop.
2. Store a received datagram in a fixed-size pending slot for that transport channel.
3. Call `redtrn_MessageReceivedNotification`.
4. Let SBB RedL call `redtri_ReadMessage`.
5. Return the pending datagram exactly once, then restore no-message behavior.

The wrapper also keeps the passive SafRetL open call, because SBB state-machine
tests show that opening a server-role connection moves it to `Down`/listen
without sending `ConnReq`.

New smoke test:

```sh
./interop/sbb-wrapper/build/sbb_transport_notification_test
```

Expected concurrent baseline command:

```sh
rm -f /tmp/sbb-passive.log /tmp/sbb-active.log
./interop/sbb-wrapper/build/sbb-rasta-wrapper passive 127.0.0.1 --rounds 3 --trace --run-seconds 30 > /tmp/sbb-passive.log 2>&1 &
sleep 1
./interop/sbb-wrapper/build/sbb-rasta-wrapper active 127.0.0.1 --rounds 3 --trace --run-seconds 30 > /tmp/sbb-active.log 2>&1
cat /tmp/sbb-passive.log
cat /tmp/sbb-active.log
```

### Kali Validation Result

The real Kali validation used:

```sh
cmake -S interop/sbb-wrapper -B interop/sbb-wrapper/build -G Ninja -DSBB_ROOT=$HOME/Desktop/sbb-investigation/sbb-rasta-stack
cmake --build interop/sbb-wrapper/build
```

Result:

- CMake configure passed with real `SBB_ROOT`.
- CMake build passed.
- The build created `ping_pong_payload_test`, `udp_transport_test`, `sbb_adapter_bridge_test`, `sbb_safretl_smoke_test`, and `sbb-rasta-wrapper`.
- `ping_pong_payload_test` passed.
- `udp_transport_test` passed.
- `sbb_adapter_bridge_test` passed.
- `sbb_safretl_smoke_test` passed.
- `sbb_safretl_smoke_test` showed that the `srapi_Init` path works.
- The RedL bridge sent UDP frames with lengths `58` and `48`.
- `sradin_SendMessage` sent lengths `50` and `40` with result `0`.
- Passive single-process smoke reported `srapi_Init result=0`, stayed `Closed` because no active peer was running, and shut down cleanly.
- Active single-process smoke reported `srapi_Init result=0` and `srapi_OpenConnection result=0`, sent length `58` and `48` frames, moved `Start` then `Closed` because no passive peer was running at the same time, and shut down cleanly.
- Runtime log says Step 8G SBB SafRetL run-loop smoke only; no Rust-to-SBB interop is claimed.

## Remaining Work After Step 8G

1. Run the corrected Step 8H SBB-to-SBB wrapper baseline in Kali and confirm packet direction is active `7100/7101` to passive `7000/7001`.
2. Run Rust active to SBB passive with captured traces only after SBB-to-SBB is understood.
3. Only after live evidence, decide whether to add a Rust `sbb-local` profile or CLI config overrides.

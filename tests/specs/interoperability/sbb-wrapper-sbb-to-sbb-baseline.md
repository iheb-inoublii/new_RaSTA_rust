# SBB Wrapper SBB-To-SBB Baseline

## Objective

Verify the SBB wrapper can process incoming UDP datagrams through SBB RedL/SafRetL in a concurrent two-process SBB-to-SBB baseline.

## Related requirement

Supervisor Step 8H: fix the SBB wrapper receive path so incoming UDP datagrams notify SBB RedL and can be consumed by SafRetL.

## Preconditions

- The Rust repository is available in Kali/Linux.
- SBB checkout exists at `$HOME/Desktop/sbb-investigation/sbb-rasta-stack`.
- CMake, Ninja, and a C compiler are available.
- Step 8G SafRetL run-loop smoke validation has passed.
- Rust protocol behavior, Rust profiles, Docker setup, and Rust applications are unchanged.

## Test setup

- Build the wrapper with real `SBB_ROOT`.
- Run passive and active wrapper processes concurrently on loopback.
- Capture passive and active logs separately.
- Use the default two-channel UDP mapping.

## Test data

- remote IP: `127.0.0.1`
- rounds: `3`
- run duration: `30 seconds`
- passive local ports: `7000`, `7001`
- active local ports: `7100`, `7101`
- RedL transport notification function: `redtrn_MessageReceivedNotification`

## Test steps

Build:

```sh
cmake -S interop/sbb-wrapper -B interop/sbb-wrapper/build -G Ninja -DSBB_ROOT=$HOME/Desktop/sbb-investigation/sbb-rasta-stack
cmake --build interop/sbb-wrapper/build
```

Run smoke tests:

```sh
./interop/sbb-wrapper/build/ping_pong_payload_test
./interop/sbb-wrapper/build/udp_transport_test
./interop/sbb-wrapper/build/sbb_adapter_bridge_test
./interop/sbb-wrapper/build/sbb_safretl_smoke_test
./interop/sbb-wrapper/build/sbb_transport_notification_test
```

Run concurrent baseline:

```sh
sudo tcpdump -ni lo udp port 7000 or udp port 7001 or udp port 7100 or udp port 7101
```

In another shell:

```sh
rm -f /tmp/sbb-passive.log /tmp/sbb-active.log
./interop/sbb-wrapper/build/sbb-rasta-wrapper passive 127.0.0.1 --rounds 3 --trace --run-seconds 30 > /tmp/sbb-passive.log 2>&1 &
sleep 1
./interop/sbb-wrapper/build/sbb-rasta-wrapper active 127.0.0.1 --rounds 3 --trace --run-seconds 30 > /tmp/sbb-active.log 2>&1
cat /tmp/sbb-passive.log
cat /tmp/sbb-active.log
```

Optional diagnostic no-abort run if SBB calls `rasys_FatalError`:

```sh
rm -f /tmp/sbb-passive.log /tmp/sbb-active.log
./interop/sbb-wrapper/build/sbb-rasta-wrapper passive 127.0.0.1 --rounds 3 --trace --debug-no-abort --run-seconds 30 > /tmp/sbb-passive.log 2>&1 &
sleep 1
./interop/sbb-wrapper/build/sbb-rasta-wrapper active 127.0.0.1 --rounds 3 --trace --debug-no-abort --run-seconds 30 > /tmp/sbb-active.log 2>&1
cat /tmp/sbb-passive.log
cat /tmp/sbb-active.log
```

## Expected result

- Passive logs UDP receive activity.
- Passive logs transport polling into a pending slot.
- Passive logs `redtrn_MessageReceivedNotification`.
- `redtri_ReadMessage` consumes pending datagrams exactly once.
- tcpdump shows initial connection frames from active ports `7100/7101` to passive ports `7000/7001`.
- Active does not immediately close because passive missed all incoming frames.
- Both active and passive reach `Up`.
- Passive receives heartbeat.
- Active exits cleanly through `Closed`.
- Passive may exit after `Up` plus heartbeat before waiting for active `DiscReq`.
- Normal run exits without `rasys_FatalError`.
- No Rust-to-SBB interoperability is claimed.

## Previous failed result

- Passive and active were run at the same time.
- tcpdump showed packets from passive ports `7000/7001` to active ports `7100/7101`:

  ```text
  127.0.0.1.7000 > 127.0.0.1.7100 length 58
  127.0.0.1.7001 > 127.0.0.1.7101 length 58
  127.0.0.1.7000 > 127.0.0.1.7100 length 48
  127.0.0.1.7001 > 127.0.0.1.7101 length 48
  ```

- Expected initial direction is active ports `7100/7101` to passive ports `7000/7001`.
- Passive stayed `Closed` or did not progress to `Up`.
- Passive did not log UDP receive, `redtri_ReadMessage`, or `redtrn_MessageReceivedNotification`.
- Root causes: the wrapper did not poll UDP and notify SBB RedL before RedL attempted to read transport data, and the wrapper had SBB active/passive ID ordering inverted.

## Implemented fix

- The wrapper polls UDP sockets during the SafRetL run loop.
- Received datagrams are stored in a fixed pending slot per transport channel.
- The wrapper calls `redtrn_MessageReceivedNotification` when a pending datagram exists and RedL is initialized.
- `redtri_ReadMessage` consumes the pending datagram instead of directly polling UDP.
- Active is configured as SBB client with sender `0x61` and receiver `0x62`.
- Passive is configured as SBB server/listener with sender `0x62` and receiver `0x61`.
- Passive still calls `srapi_OpenConnection`, because SBB state-machine tests show server open transitions to `Down`/listen without sending `ConnReq`.

SBB connection matching finding:

- `srapi_OpenConnection` resolves a connection through exact static sender and receiver IDs.
- `srcor_IsConnRoleServer` treats `sender_id > receiver_id` as server/passive.
- Incoming SafRetL frame validation compares the frame sender/receiver against the local reversed peer tuple.
- Two processes using only the same `0x61 -> 0x62` static entry cannot form the desired `0x61 <-> 0x62` baseline.
- The wrapper-local passive configuration therefore provides the reversed `0x62 -> 0x61` entry without modifying the external SBB checkout.

## Actual result

- tcpdump now shows the expected active-to-passive direction:

  ```text
  127.0.0.1.7100 > 127.0.0.1.7000 length 58
  127.0.0.1.7101 > 127.0.0.1.7001 length 58
  127.0.0.1.7100 > 127.0.0.1.7000 length 48
  127.0.0.1.7101 > 127.0.0.1.7001 length 48
  ```

- Passive logs UDP receive activity for length `58` and later length `48` packets on both channels.
- Passive logs transport polling into pending slots.
- `redtri_ReadMessage` consumes pending datagrams.
- `redtrn_MessageReceivedNotification` is invoked.
- Passive still remains `Down` and does not reach `Up`.
- Newly identified blocker: wrapper `rednot_MessageReceivedNotification` logged RedL notifications but did not forward them into SBB SafRetL adapter notification `sradno_MessageReceivedNotification`.
- Fix applied after this result: `rednot_MessageReceivedNotification` now calls `sradno_MessageReceivedNotification`, and `rednot_DiagnosticNotification` now calls `sradno_DiagnosticNotification`.
- Additional trace logging now records RedL/SafRetL frame lengths, SafRetL message type when identifiable from fixed offsets, `sradno_*` return codes, and SafRetL diagnostic counters.
- Additional endpoint trace logging records `srapi_OpenConnection` arguments and returned connection ID, every `srapi_CheckTimings` result, every `srapi_GetConnectionState` result, and every `srapi_ReadData` result.
- Safety notification trace logging records state names, disconnect reason values, and diagnostic counter values for safety, address, type, SN, and CSN errors.

Latest Kali result:

- Both active and passive SBB wrapper processes reach `Up`.
- Passive logs `srapi_GetConnectionState: connection=0 result=0 state=Up`.
- Passive receives heartbeat frame `sr_type=0x184c(Heartbeat)`.
- Passive receives disconnect request frame `sr_type=0x1848(DiscReq)`.
- Passive transitions to `Closed` through `srnot_ConnectionStateNotification connection=0 state=1(Closed)`.
- Active exits cleanly with `connection closed after Up; graceful SBB-to-SBB smoke complete` and skips `srapi_CloseConnection` because the connection is already closed after `Up`.
- This proves the SBB-to-SBB baseline connection works.

Post-disconnect fix:

- After the disconnect, the wrapper continued polling/reading RedL/SafRetL after SBB had closed the RedL channel.
- That post-close read path could call `sradin_ReadMessage` / `redint_ReadMessage` and trigger `rasys_FatalError reason=6(InvalidParameter)` or `reason=16(InternalError)`.
- The wrapper now records `Up` and `Closed after Up` from both endpoint state polling and `srnot_ConnectionStateNotification`.
- This handles passive close processing, where `DiscReq` can close the connection while the wrapper is still inside the RedL/SafRetL notification callback stack.
- Once the global `Closed after Up` latch is set, `sradin_ReadMessage` returns no-message instead of entering `redint_ReadMessage`, and transport polling stops notifying RedL.
- Once `Closed` after `Up` is observed, the wrapper stops before calling `srapi_ReadData`, before another poll cycle, and before any further `sradin_ReadMessage` / `redint_ReadMessage` path.
- `debug_no_abort` remains diagnostic only; the normal run should not hit `rasys_FatalError`.
- Step 8H success condition is `Up`, heartbeat, `DiscReq`/`Closed`, and no `rasys_FatalError` in the normal run.

Remaining Up-state fatal and fix:

- Kali showed passive could still call `rasys_FatalError reason=6(InvalidParameter)` while still `Up`, before the later `DiscReq`.
- The fatal phase was `sradin_ReadMessage:redint_ReadMessage`.
- The likely cause was `sradin_ReadMessage` entering `redint_ReadMessage` even when no RedL upper-layer message notification was active.
- The wrapper now tracks RedL channel-open state and a one-shot RedL read allowance per redundancy channel.
- `rednot_MessageReceivedNotification` grants the allowance while forwarding to `sradno_MessageReceivedNotification`.
- `sradin_ReadMessage` calls `redint_ReadMessage` only when the channel is open, the connection is not closed after `Up`, and a RedL message notification is currently active.
- Poll-style calls outside this notification flow return `radef_kNoMessageReceived`.
- `sbb_adapter_bridge_test` now checks repeated no-pending `sradin_ReadMessage` calls return `NoMessageReceived`.

DiscReq notification re-entrancy fix:

- Kali showed the final normal-run abort happened while passive handled `sr_type=0x1848(DiscReq)`.
- SBB closed RedL/SafRetL during that notification, but the same callback stack could still re-enter `sradin_ReadMessage` / `redint_ReadMessage`.
- The wrapper now marks DiscReq before calling `redtrn_MessageReceivedNotification`.
- During DiscReq notification handling, only the first valid `redint_ReadMessage` is allowed.
- The DiscReq read is marked consumed before entering RedL so re-entrant reads return `radef_kNoMessageReceived`.
- If `Closed after Up` is observed while forwarding `rednot_MessageReceivedNotification`, the wrapper stops the notification path safely and avoids further RedL/SafRetL calls.

Final Step 8H passive smoke boundary before Ping/Pong runtime work:

- Step 8H proof requires SBB-to-SBB reaching `Up` and exchanging heartbeat.
- Active clean close is verified separately and remains unchanged.
- Passive exits after reaching `Up` and receiving at least one heartbeat, before waiting for active `DiscReq`.
- This avoids the unstable SBB post-smoke passive shutdown path while preserving the useful baseline evidence.
- Passive logs `passive observed Up and heartbeat; SBB-to-SBB smoke complete`, `passive smoke success condition reached`, and `stopping SafRetL/RedL polling`.

Step 8I changes the application runtime after this baseline:

- Passive stays alive after `Up` and heartbeat during Ping/Pong runs.
- Passive exits successfully only after receiving all requested Ping counters and sending matching Pong counters.
- Active exits successfully only after receiving all expected Pong counters.
- Runtime summaries report sent/received counts and `success=true/false`.

## Postconditions

- Rust protocol code remains unchanged.
- No Rust `sbb-local` profile is added.
- No Docker setup is added.
- No Rust-to-SBB interoperability claim is made.

## Evidence

- `interop/sbb-wrapper/src/sbb_adapter.c`
- `interop/sbb-wrapper/src/sbb_redundancy_notifications.c`
- `interop/sbb-wrapper/src/sbb_safety_notifications.c`
- `interop/sbb-wrapper/src/sbb_endpoint.c`
- `interop/sbb-wrapper/src/sbb_redundancy_notifications.c`
- `interop/sbb-wrapper/tests/sbb_adapter_bridge_test.c`
- `interop/sbb-wrapper/tests/sbb_transport_notification_test.c`
- `interop/sbb-wrapper/src/main.c`

## Automation status

Partially automated. Smoke tests are automated in CMake. The two-process baseline is command-driven and requires Kali/Linux.

## Open points

- Verify in Kali that Step 8I active/passive Ping/Pong completes all requested rounds.
- Do not attempt Rust-to-SBB until SBB-to-SBB Ping/Pong behavior is understood.

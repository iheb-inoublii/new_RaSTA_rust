# SBB Wrapper SafRetL Run Loop

## Objective

Verify that the Step 8G wrapper links SBB SafRetL and can execute a smoke-only public API run loop without claiming Rust-to-SBB interoperability.

## Related requirement

Supervisor Step 8G: integrate the SBB SafRetL public API run loop into the SBB wrapper while keeping Rust protocol behavior and Rust profiles unchanged.

## Preconditions

- The Rust repository is available in Kali/Linux.
- SBB checkout exists at `$HOME/Desktop/sbb-investigation/sbb-rasta-stack`.
- CMake, Ninja, and a C compiler are available.
- Step 8E UDP transport validation has passed.
- Step 8F RedL bridge validation has passed.
- Rust protocol behavior, Rust profiles, Docker setup, and Rust applications are unchanged.

## Test setup

- Build the wrapper with real `SBB_ROOT`.
- Use the wrapper's two-channel loopback UDP mapping.
- Run the active and passive wrapper CLI modes separately as smoke checks.

## Test data

- remote IP: `127.0.0.1`
- rounds: `3`
- run duration: `5 seconds`
- SafRetL network ID: `123456`
- SafRetL sender ID: `0x61`
- SafRetL receiver ID: `0x62`
- payload format: fixed five-byte Ping/Pong application payload

## Build steps

From the Rust repository root:

```sh
cmake -S interop/sbb-wrapper -B interop/sbb-wrapper/build -G Ninja -DSBB_ROOT=$HOME/Desktop/sbb-investigation/sbb-rasta-stack
cmake --build interop/sbb-wrapper/build
```

## Test steps

Run wrapper smoke tests:

```sh
./interop/sbb-wrapper/build/ping_pong_payload_test
./interop/sbb-wrapper/build/udp_transport_test
./interop/sbb-wrapper/build/sbb_adapter_bridge_test
./interop/sbb-wrapper/build/sbb_safretl_smoke_test
```

Run wrapper CLI smoke tests:

```sh
./interop/sbb-wrapper/build/sbb-rasta-wrapper passive 127.0.0.1 --rounds 3 --trace --run-seconds 5
./interop/sbb-wrapper/build/sbb-rasta-wrapper active 127.0.0.1 --rounds 3 --trace --run-seconds 5
```

## Expected result

- CMake configure succeeds with real `SBB_ROOT`.
- CMake build succeeds.
- Existing payload, UDP, and RedL bridge tests pass.
- `sbb_safretl_smoke_test` initializes UDP, initializes SafRetL, calls active open, polls timings/state, closes, and exits successfully.
- CLI runtime log says Step 8G SBB SafRetL run-loop smoke only.
- Active mode calls `srapi_OpenConnection`.
- Both roles poll with `srapi_CheckTimings` for `--run-seconds`.
- Ping payloads are sent only if SafRetL reports `Up`.
- No Rust-to-SBB interoperability is claimed.

## SafRetL API functions used

- `srapi_Init`
- `srapi_OpenConnection`
- `srapi_CheckTimings`
- `srapi_GetConnectionState`
- `srapi_SendData`
- `srapi_ReadData`
- `srapi_CloseConnection`

## SafRetL notification callbacks

- `srnot_MessageReceivedNotification`
- `srnot_ConnectionStateNotification`
- `srnot_SrDiagnosticNotification`
- `srnot_RedDiagnosticNotification`

## Smoke configuration

- network ID: `123456`
- connection 0 sender ID: `0x61`
- connection 0 receiver ID: `0x62`
- `t_max`: `750 ms`
- `t_h`: `300 ms`
- safety code: Lower MD4
- `m_w_a`: `10`
- `n_send_max`: `20`
- `n_max_packet`: `1`

## Actual result

- Real SBB_ROOT was used: `-DSBB_ROOT=$HOME/Desktop/sbb-investigation/sbb-rasta-stack`.
- CMake configure passed.
- CMake build passed.
- The build created `ping_pong_payload_test`, `udp_transport_test`, `sbb_adapter_bridge_test`, `sbb_safretl_smoke_test`, and `sbb-rasta-wrapper`.
- `ping_pong_payload_test` passed.
- `udp_transport_test` passed.
- `sbb_adapter_bridge_test` passed.
- `sbb_safretl_smoke_test` passed.
- `sbb_safretl_smoke_test` showed that the `srapi_Init` path works.
- RedL bridge sent UDP frames with lengths `58` and `48`.
- `sradin_SendMessage` sent lengths `50` and `40` with result `0`.
- Passive single-process smoke reported `srapi_Init result=0`, stayed `Closed` because no active peer was running, and shut down cleanly.
- Active single-process smoke reported `srapi_Init result=0` and `srapi_OpenConnection result=0`, sent length `58` and `48` frames, moved `Start` then `Closed` because no passive peer was running at the same time, and shut down cleanly.
- Runtime log correctly says Step 8G SBB SafRetL run-loop smoke only; no Rust-to-SBB interop is claimed.

## Postconditions

- Rust protocol code remains unchanged.
- No Rust `sbb-local` profile is added.
- Docker/Podman setup was added later and passed in Step 9B.
- Rust-to-SBB five-round Ping/Pong was claimed later from Step 8O/9B evidence.

## Evidence

- `interop/sbb-wrapper/src/sbb_endpoint.c`
- `interop/sbb-wrapper/src/sbb_endpoint.h`
- `interop/sbb-wrapper/src/sbb_safety_notifications.c`
- `interop/sbb-wrapper/src/main.c`
- `interop/sbb-wrapper/tests/sbb_safretl_smoke_test.c`
- `interop/sbb-wrapper/tests/sbb_transport_notification_test.c`
- `interop/sbb-wrapper/CMakeLists.txt`

## Automation status

Partially automated. Rust validation can run on Windows. SBB wrapper configure, build, and smoke tests require Kali/Linux with CMake, Ninja, a C compiler, and the SBB checkout.

## Open points

- Add bounded queues if SBB requires asynchronous adapter handoff.
- Step 8H adds the missing RedL transport notification receive path for the SBB-to-SBB wrapper baseline.
- Do not attempt Rust-to-SBB until SBB-to-SBB wrapper behavior is understood.

# SBB Wrapper Test Evidence

## Environment

Wrapper path:

```text
/home/iheb/Desktop/new_RaSTA_rust/interop/sbb-wrapper
```

SBB checkout:

```text
/home/iheb/Desktop/sbb-investigation/sbb-rasta-stack
```

## Commands

Configure:

```sh
cmake -S. -B build -G Ninja -DSBB_ROOT=/home/iheb/Desktop/sbb-investigation/sbb-rasta-stack
```

Build:

```sh
cmake --build build --verbose
```

Smoke checks:

```sh
./ping_pong_payload_test
./udp_transport_test
./sbb_adapter_bridge_test
./sbb_transport_notification_test
./sbb_safretl_smoke_test
./sbb-rasta-wrapper --help
```

## Expected Result

The wrapper configures with a real `SBB_ROOT`, links SBB static libraries, builds `sbb-rasta-wrapper`, and passes wrapper smoke tests without claiming Rust-to-SBB interoperability.

## Actual Result

Configure printed:

```text
-- SBB_ROOT=/home/iheb/Desktop/sbb-investigation/sbb-rasta-stack
```

Build completed successfully and linked:

- `sbb-rasta-common/librasta_common.a`
- `sbb-rasta-redundancy/librasta_redundancy.a`
- `sbb-rasta-safety-retransmission/librasta_safety_retransmission.a`

Executable built:

- `build/sbb-rasta-wrapper`

Smoke checks passed:

- `ping_pong_payload_test`
- `udp_transport_test`
- `sbb_adapter_bridge_test`
- `sbb_transport_notification_test`
- `sbb_safretl_smoke_test`
- `sbb-rasta-wrapper --help`

## Important Log Excerpts

- UDP transport opened POSIX UDP sockets.
- `redtri_Init` reported UDP transport ready.
- `sradin_Init` called `redint_Init` and returned `NoError`.
- `sradin_OpenRedundancyChannel` channel 0 returned `NoError`.
- `sradin_SendMessage` called `redint_SendMessage` and returned `NoError`.
- `srapi_OpenConnection` active returned `NoError` and `connection_id=0`.
- Active smoke produced a 58-byte RedL frame for a 50-byte SafRetL ConnectionRequest.
- Active smoke produced a 48-byte RedL frame for a 40-byte SafRetL disconnect/message.

Wrapper help showed:

```text
usage: ./sbb-rasta-wrapper <active|passive> <remote-ip> [options]
  --rounds
  --run-seconds
  --trace
  --debug-no-abort
  --channel0-local
  --channel0-remote
  --channel1-local
  --channel1-remote
```

## Interpretation

The wrapper is no longer only a stub build when `SBB_ROOT` is provided. It compiles against real SBB modules, links real SBB libraries, and exercises UDP, RedL bridge, transport notification, and SafRetL smoke paths.

## Limitations

This is wrapper-only evidence. It does not prove Rust-to-SBB interoperability, does not add Docker, and does not modify Rust protocol behavior or Rust applications. The next evidence step is an active/passive SBB wrapper runtime test, followed by a live Rust-to-SBB test.

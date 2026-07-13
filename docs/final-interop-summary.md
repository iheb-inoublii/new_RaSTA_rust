# Final Interop Summary

Branch: `refactor/test-profile-foundation`

Protected branch: `interop/librasta-wire-compat`

## Profiles

Implemented profiles:

- `academic`
- `librasta-local`
- `sbb-local`

`academic` remains the default Rust profile. `librasta-local` and `sbb-local`
are explicit interoperability profiles.

## Main Achievements

- Profile/config API with predefined profiles and custom builder support.
- Per-channel transport abstraction for independent redundancy-channel transports.
- Public endpoint API around connect, poll, send, receive, close, status, diagnostics, and tracing.
- Structured tracing and public error cleanup.
- Signal-controller and interlocking-controller examples.
- Rust-to-Rust and interop-oriented Ping/Pong app.
- SBB wrapper with POSIX UDP transport, RedL/SafRetL bridge, and Ping/Pong runtime.
- Native Rust-to-SBB interop with `sbb-local`.
- Docker/Podman reproduction of the Rust-to-SBB five-round Ping/Pong run.

## Final Status

- Native SBB-to-SBB Ping/Pong 5 rounds: passed.
- Native Rust-to-SBB handshake/heartbeat: passed.
- Native Rust-to-SBB Ping/Pong 5 rounds: passed.
- Docker/Podman Rust tests: passed.
- Docker/Podman SBB wrapper build/tests: passed.
- Docker/Podman Rust-to-SBB 5-round Ping/Pong: passed.

## Known Caveat

`ChannelSupervisionFailure` diagnostics can appear during SBB interoperability
runs, but they did not prevent successful five-round Rust-to-SBB Ping/Pong
completion in the captured native and Docker/Podman evidence.

## Final Native Commands

SBB passive:

```sh
./build/sbb-rasta-wrapper passive 127.0.0.1 \
  --rounds 5 --trace --run-seconds 30 \
  --channel0-local 7000 --channel0-remote 7100 \
  --channel1-local 7001 --channel1-remote 7101
```

Rust active:

```sh
cargo run -p ping-pong-node -- active 127.0.0.1 \
  --profile sbb-local \
  --rounds 5 \
  --trace-wire \
  --run-seconds 30 \
  --ping-delay-ms 300 \
  --channel-0-local-port 7100 \
  --channel-0-remote-port 7000 \
  --channel-1-local-port 7101 \
  --channel-1-remote-port 7001
```

## Final Docker/Podman Commands

Set the external SBB checkout path:

```sh
export SBB_HOST_ROOT=/home/iheb/Desktop/sbb-investigation/sbb-rasta-stack
export SBB_ROOT=/sbb-rasta-stack
```

Run Rust tests:

```sh
docker compose -f docker/interop/docker-compose.yml run --rm rust-test
```

Build and test the SBB wrapper:

```sh
docker compose -f docker/interop/docker-compose.yml run --rm sbb-wrapper-build
```

Run the live Rust-to-SBB interop scenario:

```sh
docker compose -f docker/interop/docker-compose.yml --profile live up --build
```

If a native CMake cache has been created under the wrapper directory, clean it
before Docker/Podman runs:

```sh
rm -rf interop/sbb-wrapper/build
```

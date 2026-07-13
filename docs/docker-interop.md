# Docker Interop Environment

This document defines the Step 9A Docker skeleton for reproducing the Rust and
SBB wrapper interop tests in a controlled environment.

Status:

- Native Kali Rust-to-SBB 5-round Ping/Pong: passed.
- Docker/Podman reproduction: passed in Step 9B.

The Docker setup does not change Rust protocol behavior, SBB wrapper behavior,
or the native Windows/Kali workflow.

## Files

- `docker/interop/Dockerfile.rust`
- `docker/interop/Dockerfile.sbb-wrapper`
- `docker/interop/docker-compose.yml`
- `docker/interop/README.md`

## Rust Container

The Rust image builds from `rust:1-bookworm` and defaults to:

```sh
cargo test --workspace --all-targets --all-features
```

Run it with:

```sh
docker compose -f docker/interop/docker-compose.yml run --rm rust-test
```

## SBB Wrapper Container

The SBB wrapper image installs a C toolchain, CMake, and Ninja. It expects an
external SBB checkout to be supplied by bind mount.

Linux/Kali example:

```sh
export SBB_HOST_ROOT=/home/iheb/Desktop/sbb-investigation/sbb-rasta-stack
export SBB_ROOT=/sbb-rasta-stack
docker compose -f docker/interop/docker-compose.yml run --rm sbb-wrapper-build
```

PowerShell example:

```powershell
$env:SBB_HOST_ROOT = "C:\path\to\sbb-rasta-stack"
$env:SBB_ROOT = "/sbb-rasta-stack"
docker compose -f docker/interop/docker-compose.yml run --rm sbb-wrapper-build
```

The wrapper build service runs:

```sh
cmake -S interop/sbb-wrapper -B interop/sbb-wrapper/build -G Ninja -DSBB_ROOT=$SBB_ROOT
cmake --build interop/sbb-wrapper/build
ctest --test-dir interop/sbb-wrapper/build --output-on-failure
```

## Docker/Podman Live Rust-To-SBB Test

The Step 9A live test mirrors the successful native Kali run:

- SBB passive wrapper container
- Rust active `ping-pong-node` container
- `--profile sbb-local`
- 5 rounds
- `--ping-delay-ms 300`
- channel 0: Rust `7100` to SBB `7000`
- channel 1: Rust `7101` to SBB `7001`

Run:

```sh
export SBB_HOST_ROOT=/home/iheb/Desktop/sbb-investigation/sbb-rasta-stack
export SBB_ROOT=/sbb-rasta-stack
docker compose -f docker/interop/docker-compose.yml --profile live up --build
```

The compose network uses fixed IP addresses because the Rust CLI currently
parses the peer as an IP address:

- Rust active: `172.28.0.10`
- SBB passive: `172.28.0.20`

Expected success evidence:

- Rust transitions `Opening -> Up`.
- Rust sends `Ping(1)..Ping(5)`.
- Rust receives `Pong(1)..Pong(5)`.
- Rust prints `active summary: sent_pings=5 received_pongs=5 success=true`.
- SBB passive prints `passive summary: received_pings=5 sent_pongs=5 success=true`.

## Step 9B Result

The Docker/Podman environment was verified with:

```sh
docker compose -f docker/interop/docker-compose.yml run --rm rust-test
docker compose -f docker/interop/docker-compose.yml run --rm sbb-wrapper-build
docker compose -f docker/interop/docker-compose.yml --profile live up --build
```

Observed live evidence:

```text
sbb-passive received Ping(5)
sbb-passive sent Pong(5)
passive Ping/Pong success condition reached
passive summary: received_pings=5 sent_pongs=5 success=true
rust-active Pong(5) received
rust-active Completed 5 ping-pong rounds
active summary: sent_pings=5 received_pongs=5 success=true
```

Status:

- Native SBB-to-SBB Ping/Pong 5 rounds: passed.
- Native Rust-to-SBB handshake/heartbeat: passed.
- Native Rust-to-SBB Ping/Pong 5 rounds: passed.
- Docker/Podman Rust tests: passed.
- Docker/Podman SBB wrapper build/tests: passed.
- Docker/Podman Rust-to-SBB 5-round Ping/Pong: passed.

## Docker Build Cache Note

An earlier Docker/Podman build hit a CMake path mismatch because the native
`interop/sbb-wrapper/build` cache had been created outside the container and
then bind-mounted at `/workspace`.

The workaround used before rerunning the Docker/Podman flow was:

```sh
rm -rf interop/sbb-wrapper/build
```

Permanent recommendation: add a `.dockerignore` in a later cleanup step so
native build artifacts are excluded from Docker build contexts and bind-mounted
workflows.

## Notes

The native Kali run remains the source of Step 8O evidence. Step 9B confirms
the same Rust-to-SBB five-round Ping/Pong scenario in Docker/Podman.

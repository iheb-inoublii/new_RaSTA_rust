# Docker Interop Skeleton

This directory contains the first Docker-based interop skeleton. It is meant to
reproduce the existing native Kali Rust-to-SBB evidence in a controlled
environment without changing Rust protocol behavior or SBB wrapper behavior.

Status:

- Native Kali Rust-to-SBB 5-round Ping/Pong: passed.
- Docker reproduction: pending.

## Files

- `Dockerfile.rust` builds a Rust workspace image and defaults to
  `cargo test --workspace --all-targets --all-features`.
- `Dockerfile.sbb-wrapper` installs CMake, Ninja, and a C toolchain for the SBB
  wrapper.
- `docker-compose.yml` defines Rust test, SBB wrapper build, and intended live
  Rust active to SBB passive services.

## SBB Checkout

The SBB checkout is not stored in this repository. Provide it with a bind mount:

```sh
export SBB_HOST_ROOT=/home/iheb/Desktop/sbb-investigation/sbb-rasta-stack
export SBB_ROOT=/sbb-rasta-stack
```

On PowerShell:

```powershell
$env:SBB_HOST_ROOT = "C:\path\to\sbb-rasta-stack"
$env:SBB_ROOT = "/sbb-rasta-stack"
```

## Rust Workspace Test

```sh
docker compose -f docker/interop/docker-compose.yml run --rm rust-test
```

## SBB Wrapper Build And Smoke Tests

```sh
docker compose -f docker/interop/docker-compose.yml run --rm sbb-wrapper-build
```

This runs:

```sh
cmake -S interop/sbb-wrapper -B interop/sbb-wrapper/build -G Ninja -DSBB_ROOT=$SBB_ROOT
cmake --build interop/sbb-wrapper/build
ctest --test-dir interop/sbb-wrapper/build --output-on-failure
```

## Intended Live Rust-To-SBB Test

The compose live profile uses fixed container IP addresses because the current
Rust CLI accepts IP addresses, not DNS service names.

Start the SBB passive wrapper and Rust active ping-pong node:

```sh
docker compose -f docker/interop/docker-compose.yml --profile live up --build
```

The intended scenario is:

- SBB passive wrapper at `172.28.0.20`
- Rust active `ping-pong-node` at `172.28.0.10`
- profile `sbb-local`
- 5 rounds
- `--ping-delay-ms 300`
- channel 0: Rust `7100` to SBB `7000`
- channel 1: Rust `7101` to SBB `7001`

Do not treat Docker interop as passed until the compose run has been executed
and the logs show both sides completing the five Ping/Pong rounds.

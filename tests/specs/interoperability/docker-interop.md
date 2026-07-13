# Docker Interop Test Environment

## Objective
Define a Docker-based environment for reproducing the Rust workspace tests, SBB
wrapper build/tests, and Rust-to-SBB Ping/Pong live interoperability scenario.

## Related requirement
Step 9A Docker-based interop test environment planning and skeleton.

## Preconditions
- Docker with compose support is installed.
- The Rust repository is available as the Docker build context.
- The external SBB checkout is available on the host.
- Native Kali Rust-to-SBB 5-round Ping/Pong has passed.

## Test setup
Use the files under `docker/interop/`:

- `Dockerfile.rust`
- `Dockerfile.sbb-wrapper`
- `docker-compose.yml`
- `README.md`

Provide the SBB checkout with:

```sh
export SBB_HOST_ROOT=/home/iheb/Desktop/sbb-investigation/sbb-rasta-stack
export SBB_ROOT=/sbb-rasta-stack
```

## Test data
- Rust active IP: `172.28.0.10`
- SBB passive IP: `172.28.0.20`
- Rust profile: `sbb-local`
- Rounds: `5`
- Ping delay: `300 ms`
- SBB channel 0: local `7000`, remote `7100`
- SBB channel 1: local `7001`, remote `7101`
- Rust channel 0: local `7100`, remote `7000`
- Rust channel 1: local `7101`, remote `7001`

## Test steps
1. Run the Rust workspace test container:

   ```sh
   docker compose -f docker/interop/docker-compose.yml run --rm rust-test
   ```

2. Build and test the SBB wrapper container:

   ```sh
   docker compose -f docker/interop/docker-compose.yml run --rm sbb-wrapper-build
   ```

3. Run the intended live Rust-to-SBB test:

   ```sh
   docker compose -f docker/interop/docker-compose.yml --profile live up --build
   ```

## Expected result
- Rust workspace tests pass in the Rust container.
- SBB wrapper configures and builds against the mounted SBB checkout.
- SBB wrapper smoke tests pass in the SBB wrapper container.
- Live Docker run reproduces the native Kali Rust-to-SBB five-round Ping/Pong result.

## Actual result
Passed in Step 9B.

Docker/Podman Rust tests passed:

```sh
docker compose -f docker/interop/docker-compose.yml run --rm rust-test
```

Docker/Podman SBB wrapper build/tests passed:

```sh
docker compose -f docker/interop/docker-compose.yml run --rm sbb-wrapper-build
```

Docker/Podman live interop passed:

```sh
docker compose -f docker/interop/docker-compose.yml --profile live up --build
```

Observed live evidence:

- `sbb-passive received Ping(5)`
- `sbb-passive sent Pong(5)`
- `passive Ping/Pong success condition reached`
- `passive summary: received_pings=5 sent_pongs=5 success=true`
- `rust-active Pong(5) received`
- `rust-active Completed 5 ping-pong rounds`
- `active summary: sent_pings=5 received_pongs=5 success=true`

An earlier Docker/Podman build hit a CMake path mismatch because native
`interop/sbb-wrapper/build` cache files were reused inside `/workspace`. The
workaround used was:

```sh
rm -rf interop/sbb-wrapper/build
```

Permanent recommendation: add `.dockerignore` later to exclude build artifacts.

## Postconditions
- Native Kali Rust-to-SBB 5-round Ping/Pong remains passed.
- Docker/Podman Rust tests passed.
- Docker/Podman SBB wrapper build/tests passed.
- Docker/Podman Rust-to-SBB 5-round Ping/Pong passed.
- Rust protocol behavior is unchanged.
- SBB wrapper behavior is unchanged.

## Evidence
Docker evidence:

- `cargo test --workspace --all-targets --all-features` output from the Rust container.
- CMake configure/build and `ctest` output from the SBB wrapper container.
- Rust active log showing `active summary: sent_pings=5 received_pongs=5 success=true`.
- SBB passive log showing `passive summary: received_pings=5 sent_pongs=5 success=true`.

## Automation status
Docker/Podman validation passed manually with compose.

## Open points
- Add `.dockerignore` to exclude native build artifacts.
- Decide whether to add health checks or a more deterministic startup wrapper.
- Keep native Windows/Kali workflow available.

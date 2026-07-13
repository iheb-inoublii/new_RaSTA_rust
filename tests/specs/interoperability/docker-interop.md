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
Pending. Step 9A adds Docker files and documentation only.

## Postconditions
- Native Kali Rust-to-SBB 5-round Ping/Pong remains passed.
- Docker reproduction remains pending until executed.
- Rust protocol behavior is unchanged.
- SBB wrapper behavior is unchanged.

## Evidence
Expected Docker evidence:

- `cargo test --workspace --all-targets --all-features` output from the Rust container.
- CMake configure/build and `ctest` output from the SBB wrapper container.
- Rust active log showing `active summary: sent_pings=5 received_pongs=5 success=true`.
- SBB passive log showing `passive summary: received_pings=5 sent_pongs=5 success=true`.

## Automation status
Skeleton added. Docker validation is pending.

## Open points
- Execute the Docker compose live profile and capture logs.
- Decide whether to add health checks or a more deterministic startup wrapper after the first Docker run.
- Keep native Windows/Kali workflow available.

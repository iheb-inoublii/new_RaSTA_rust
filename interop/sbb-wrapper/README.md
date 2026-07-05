# SBB Wrapper Skeleton

This directory contains a compile-ready skeleton for a future SBB RaSTA wrapper.
It does not implement Rust-to-SBB interoperability, does not add an `sbb-local`
Rust profile, and does not modify the external SBB checkout.

The current skeleton:

- accepts active/passive CLI settings
- prints deterministic parsed configuration
- defines required `sradin_*` and `redtri_*` symbols as logged stubs
- returns `radef_kNoMessageReceived` from read stubs when no queue exists
- provides a Ping/Pong payload codec matching Rust `ApplicationMessage`
- builds independently of SBB while accepting an `SBB_ROOT` CMake variable

## Build

```sh
cmake -S interop/sbb-wrapper -B interop/sbb-wrapper/build -G Ninja -DSBB_ROOT=/root/sbb-investigation/sbb-rasta-stack
cmake --build interop/sbb-wrapper/build
ctest --test-dir interop/sbb-wrapper/build
```

`SBB_ROOT` is recorded for the intended integration path, but this Step 8C
skeleton does not link SBB libraries yet.

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

Current recorded result from the available Codex session:

- Kali WSL distribution was not available; `wsl -l -v` listed only `docker-desktop`.
- `/root/sbb-investigation/sbb-rasta-stack` was not reachable from this host.
- `cmake`, `ninja`, `gcc`, `clang`, and `cl` were not available on PATH.
- Rust workspace validation still passed.

Therefore the Kali wrapper build remains pending until the repo is opened inside
the Kali environment that contains `/root/sbb-investigation/sbb-rasta-stack`.

## CLI

```sh
interop/sbb-wrapper/build/sbb-rasta-wrapper passive 127.0.0.1 --rounds 10 --run-seconds 40 --trace
interop/sbb-wrapper/build/sbb-rasta-wrapper active 127.0.0.1 --channel0-local 7100 --channel0-remote 7000
```

Default port mapping:

| Role | Channel | Local port | Remote port |
| --- | --- | --- | --- |
| passive | channel 0 | `7000` | `7100` |
| passive | channel 1 | `7001` | `7101` |
| active | channel 0 | `7100` | `7000` |
| active | channel 1 | `7101` | `7001` |

For now, the executable prints settings, calls stub initialization functions,
runs stub read/send smoke checks, and exits successfully. Send smoke checks
return `radef_kNotImplemented`; read smoke checks return
`radef_kNoMessageReceived`. It does not open UDP sockets or call SBB SafRetL
APIs.

## Ping/Pong Payload

The payload format matches `crates/rasta-core/src/application.rs`:

| Message | Bytes |
| --- | --- |
| Ping(counter) | `03 <counter little-endian u32>` |
| Pong(counter) | `04 <counter little-endian u32>` |

Total payload length is always five bytes.

## Stubbed Functions

SafRetL adapter stubs:

- `sradin_Init`
- `sradin_OpenRedundancyChannel`
- `sradin_CloseRedundancyChannel`
- `sradin_SendMessage`
- `sradin_ReadMessage`

RedL transport stubs:

- `redtri_Init`
- `redtri_SendMessage`
- `redtri_ReadMessage`

The function signatures are local skeleton signatures until Step 8D confirms the
exact SBB headers and links against the real SBB modules.

## Remaining Step 8E Work

- Run the recorded Kali CMake/Ninja commands in the Kali environment.
- Confirm the wrapper skeleton compiles with the available C compiler.
- Replace local skeleton function signatures only if SBB headers require it.
- Keep the wrapper outside Rust protocol code.
- Link external SBB libraries only after the exact include/library layout is confirmed.

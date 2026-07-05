# SBB Baseline Investigation

## Purpose

Document the manual baseline investigation of the SBB RaSTA stack before any Rust-to-SBB interoperability wrapper, Docker setup, or `sbb-local` profile is added.

This is evidence capture only. It does not claim Rust-to-SBB interoperability.

## Build Environment

The investigation was performed manually in Kali against:

```text
https://github.com/SchweizerischeBundesbahnen/sbb-rasta-stack
```

The repository was cloned locally and built with CMake and Ninja.

## Commands Used

```sh
cmake -DCMAKE_BUILD_TYPE:STRING=Debug \
      -DCMAKE_EXPORT_COMPILE_COMMANDS:BOOL=TRUE \
      -S. -B./build -G Ninja

cmake --build ./build --config Debug --target all --

ctest --test-dir ./build --output-on-failure
```

## Dependency Issue

CMake configure initially required Google Mock support. Installing `libgmock-dev` resolved the missing `GTest::gmock` dependency.

After that dependency fix:

- CMake configure passed.
- Build passed.
- CTest passed with `24/24` tests passing.

## Module Structure

Observed SBB modules:

- `rasta_common`
- `rasta_redundancy`
- `rasta_redundancy_config`
- `rasta_safety_retransmission`
- `rasta_safety_retransmission_config`

## Executables Found

Only GoogleTest unit-test binaries were found, for example:

- `gtest_raas`
- `gtest_radef`
- `gtest_redmsg`
- `gtest_redcrc`
- `gtest_redint`
- `gtest_srapi`
- `gtest_srmsg`
- `gtest_srsend`
- `gtest_srrece`
- `gtest_srmd4`
- `gtest_srstm`

No ready UDP client/server demo executable was found.

## Common Constants

From `radef_rasta_definitions.h`:

| Constant | Value |
| --- | --- |
| `RADEF_MAX_NUMBER_OF_RASTA_CONNECTIONS` | `2` |
| `RADEF_MAX_SR_LAYER_PAYLOAD_DATA_SIZE` | `1055` |
| `RADEF_SR_LAYER_MESSAGE_HEADER_SIZE` | `28` |
| `RADEF_SR_LAYER_APPLICATION_MESSAGE_LENGTH_SIZE` | `2` |
| `RADEF_MAX_SR_LAYER_SAFETY_CODE_SIZE` | `16` |
| `RADEF_MAX_N_SEND_MAX` | `20` |
| `RADEF_RED_LAYER_MESSAGE_HEADER_SIZE` | `8` |
| `RADEF_MAX_RED_LAYER_CHECK_CODE_SIZE` | `4` |
| `RADEF_MAX_DEFER_QUEUE_SIZE` | `10` |
| `RADEF_MAX_RED_LAYER_N_DIAGNOSIS` | `1000` |

## RedL Config

From `redcfg_red_config.c`:

| Field | Value |
| --- | --- |
| `check_code_type` | `redcty_kCheckCodeA` |
| `t_seq` | `50` |
| `n_diagnosis` | `200` |
| `n_defer_queue_size` | `4` |
| `number_of_redundancy_channels` | `2` |
| channel 0 transport IDs | `{0, 1}` |
| channel 1 transport IDs | `{2, 3}` |

SBB RedL check code enum evidence:

| SBB enum | Meaning |
| --- | --- |
| `CheckCodeA` | no check code |
| `CheckCodeB` | CRC32 polynomial `0xEE5B42FD` |
| `CheckCodeC` | CRC32 polynomial `0x1EDC6F41` |
| `CheckCodeD` | CRC16 polynomial `0x1021` |
| `CheckCodeE` | CRC16 polynomial `0x8005` |

## SafRetL Config

From `srcfg_sr_config.c`:

| Field | Value |
| --- | --- |
| `rasta_network_id` | `123456` |
| `t_max` | `750` |
| `t_h` | `300` |
| `safety_code_type` | `srcty_kSafetyCodeTypeLowerMd4` |
| `m_w_a` | `10` |
| `n_send_max` | `20` |
| `n_max_packet` | `1` |
| `n_diag_window` | `5000` |
| `number_of_connections` | `2` |
| connection 0 | sender `0x61`, receiver `0x62` |
| connection 1 | sender `1`, receiver `3` |
| MD4 A | `0x67452301` |
| MD4 B | `0xEFCDAB89` |
| MD4 C | `0x98BADCFE` |
| MD4 D | `0x10325476` |
| diagnostic intervals | `150`, `300`, `450`, `600` |

SBB safety code enum evidence:

- None
- Lower MD4
- Full MD4

## Public API Summary

From `srapi_sr_api.h`, the high-level SafRetL API includes:

- `srapi_Init`
- `srapi_GetInitializationState`
- `srapi_OpenConnection`
- `srapi_CloseConnection`
- `srapi_SendData`
- `srapi_ReadData`
- `srapi_GetConnectionState`
- timing check function for polling, heartbeat, timeouts, and send processing

## Adapter Interface Summary

SBB does not implement transport directly in the core stack. The integrator must provide adapter/interface functions, including:

- `sradin_Init`
- `sradin_OpenRedundancyChannel`
- `sradin_CloseRedundancyChannel`
- `sradin_SendMessage`
- `sradin_ReadMessage`
- `redtri_Init`
- `redtri_SendMessage`
- `redtri_ReadMessage`

This aligns with the Rust design where `rasta-core` owns protocol behavior and platform/application crates provide transports.

## Packet and Message Evidence

SBB SR message tests show:

- Connection Request type is `6200`.
- default/lower-MD4 connection request length is `50`.
- no-safety connection request length is `42`.
- full-MD4 connection request length is `58`.
- protocol version bytes are ASCII digits.
- `nSendMax` valid range is tested around `2..20`.

## Conclusion

The SBB stack builds and its unit-test baseline passes locally after installing `libgmock-dev`. The available executable artifacts are unit-test binaries only; no ready UDP endpoint/demo executable was found.

SBB configuration values are now documented, but a runtime endpoint or adapter/wrapper executable is still needed before performing Rust-to-SBB live interoperability.

Do not add `RastaProfile::sbb_local()` yet. The values are known, but a final runtime interoperability profile should wait until the SBB adapter/wrapper path is confirmed with live evidence.

## Next Steps

1. Identify or create a small SBB wrapper executable that implements the required adapter interfaces.
2. Decide how to map SBB channel transport IDs to local UDP or another test transport.
3. Capture SBB wire traces for connection request/response, heartbeat, data, and ping-pong.
4. Compare SBB traces against Rust packet encoding.
5. Add an evidence-backed `sbb-local` profile only after a runnable endpoint exists.
6. Add Rust-to-SBB handshake and ping-pong test specs once live execution is possible.

# RaSTA Test Profiles

This project exposes protocol profiles from `rasta-core::config` so library users, tests, and applications can share the same validated values.

## Academic/default profile

`RastaProfile::academic_default()` returns the existing Rust-to-Rust academic test profile. It keeps the current default behavior:

- protocol version `0303`
- MD4 lower 8-byte safety code with the existing non-standard academic initial value
- redundancy CRC option B
- two redundancy channels
- `t_max_ms = 1800`, `t_h_ms = 300`, `t_seq_ms = 100`
- `n_send_max = 20`, `mwa = 10`
- strict synchronized timestamp compatibility

This profile is intended for Rust-to-Rust tests and examples only. It is not a production railway profile.

## librasta-local profile

`RastaProfile::librasta_local()` returns the known local C librasta interoperability profile from the working `interop/librasta-wire-compat` baseline:

- protocol version `0303`
- no safety code
- redundancy CRC option A
- network identifier `1234`
- `t_max_ms = 10000`, `t_h_ms = 2000`, `t_seq_ms = 50`
- `n_send_max = 20`, `mwa = 10`
- peer-relative timestamp compatibility

This profile is intended only for Rust-to-librasta local interoperability testing. Its no-checksum behavior is unsafe and is accepted only through an explicit interop/unsafe opt-in path.

## Future sbb-local profile

An SBB local profile is intentionally not encoded yet. The project needs evidence from the SBB implementation or test configuration before adding values. Until then, SBB work should reference the interoperability specs under `tests/specs/interoperability/` and record confirmed values before introducing `RastaProfile::sbb_local()`.

## Custom profile builder

Use `RastaProfileBuilder::new()` for custom profiles. It starts from the academic/default profile and validates on `build()`.

Example:

```rust
use rasta_core::config::RastaProfileBuilder;

let profile = RastaProfileBuilder::new()
    .network_identifier(0x55aa)
    .timing(2500, 500, 125)
    .flow_control(12, 6)
    .build()?;
```

Unsafe/no-checksum profiles must call `.allow_unsafe_no_checksums(true)` before `build()`.

## Unsafe/no-checksum opt-in

Profiles with `SafetyCodeLength::None` or redundancy CRC option A do not provide the normal safety/checksum protection expected by the Rust default configuration. They are therefore rejected by the safe builder/validation path unless an explicit interop/unsafe opt-in is set. This keeps accidental unsafe profiles out of Rust-to-Rust tests while preserving known librasta interoperability behavior.

## Intended use

| Target | Profile |
| --- | --- |
| Rust-to-Rust | `RastaProfile::academic_default()` |
| Rust-to-librasta | `RastaProfile::librasta_local()` |
| Rust-to-SBB | Future `sbb-local`, values TBD from evidence |

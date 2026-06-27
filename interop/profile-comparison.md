# Profile comparison

Do not mark `Match?` as yes until the peer configuration is inspected directly.

| Parameter | Rust value | Peer value | Match? | Source/reference | Notes |
|---|---|---|---|---|---|
| RaSTA version | ASCII `0303` | Pending | Pending | `apps/rasta-node/src/profile.rs` | Academic test profile |
| local node ID | A: `0x1234`, B: `0x5678` by default | Pending | Pending | `apps/rasta-node/src/main.rs` | Override with `--local-id` |
| remote node ID | A: `0x5678`, B: `0x1234` by default | Pending | Pending | `apps/rasta-node/src/main.rs` | Override with `--remote-id` |
| network identifier | `0x00000001` | Pending | Pending | `apps/rasta-node/src/profile.rs` | Test-only |
| MD4 initial value | `02 23 45 67 98 ab cd ef ff dc ba 98 77 54 32 10` | Pending | Pending | `apps/rasta-node/src/profile.rs` | Non-production |
| MD4-8 or MD4-16 | MD4-8 lower 8 bytes | Pending | Pending | `apps/rasta-node/src/main.rs` | `SafetyCodeConfig::md4_low8` |
| redundancy CRC option | Option B | Pending | Pending | `apps/rasta-node/src/main.rs` | No silent changes |
| channel count | 2 | Pending | Pending | `apps/rasta-node/src/profile.rs` | Fixed demo mapping |
| local channel ports | A: 5000/6000, B: 5001/6001 | Pending | Pending | `apps/rasta-node/src/main.rs` | Override per channel |
| remote channel ports | A: 5001/6001, B: 5000/6000 | Pending | Pending | `apps/rasta-node/src/main.rs` | Override per channel |
| `T_max` | 1800 ms | Pending | Pending | `apps/rasta-node/src/profile.rs` | Test-only |
| `T_h` | 300 ms | Pending | Pending | `apps/rasta-node/src/profile.rs` | Test-only |
| `T_seq` | 100 ms | Pending | Pending | `apps/rasta-node/src/profile.rs` | Test-only |
| `N_sendmax` | 20 | Pending | Pending | `apps/rasta-node/src/profile.rs` | Profile-selected |
| MWA | 10 | Pending | Pending | `apps/rasta-node/src/profile.rs` | Partially verified |
| defer queue capacity | 4 | Pending | Pending | `apps/rasta-node/src/profile.rs` | Test-only |
| packetization limit | 1 | Pending | Pending | `apps/rasta-node/src/profile.rs` | One application message per packet |
| maximum packet size | SRL payload 256 bytes | Pending | Pending | `crates/rasta-core/src/connection/pdu.rs` | Fixed core buffer |
| byte order assumptions | little-endian SRL/RL numeric fields | Pending | Pending | core PDU/RL frame code | Verify by capture |
| connection initiator | Lower sender ID opens actively | Pending | Pending | `RastaConnection::connect` | Default A initiates |

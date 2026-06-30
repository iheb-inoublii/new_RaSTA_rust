# librasta local wire profile

This note records the local C librasta layout used by the `librasta-local`
Rust node profile. It is wire-layout evidence only, not an interoperability
result.

## Length accounting

The observed Rust default frames are 12 bytes longer than the C librasta local
frames because the defaults add both configured check fields:

| Layer | Rust default | librasta local |
|---|---:|---:|
| SR safety/checksum | 8 bytes, MD4 lower half | 0 bytes, `RASTA_SR_CHECKSUM_LEN = NONE` |
| RL CRC/check code | 4 bytes, option B | 0 bytes, `RASTA_CRC_TYPE = TYPE_A` |
| Total difference | 12 bytes | 0 bytes |

## 6200 ConnectionRequest

| Offset | Width | Endianness | Meaning | In encoded length? | Protected by checksum/CRC |
|---:|---:|---|---|---|---|
| 0 | 2 | little | RL length, includes RL header, SR PDU, RL CRC | yes | RL CRC if width > 0 |
| 2 | 2 | little | RL reserve | yes | RL CRC if width > 0 |
| 4 | 4 | little | RL sequence number | yes | RL CRC if width > 0 |
| 8 | 2 | little | SR length, includes SR header, payload, SR safety | yes | SR safety if length > 0 |
| 10 | 2 | little | SR type 6200 | yes | SR safety if length > 0 |
| 12 | 4 | little | receiver ID | yes | SR safety if length > 0 |
| 16 | 4 | little | sender ID | yes | SR safety if length > 0 |
| 20 | 4 | little | SR sequence number | yes | SR safety if length > 0 |
| 24 | 4 | little | confirmed SR sequence | yes | SR safety if length > 0 |
| 28 | 4 | little | timestamp | yes | SR safety if length > 0 |
| 32 | 4 | little | confirmed timestamp | yes | SR safety if length > 0 |
| 36 | 4 | bytes | protocol version `0303` | yes | SR safety if length > 0 |
| 40 | 2 | little | `N_sendmax` | yes | SR safety if length > 0 |
| 42 | 8 | bytes | setup reserved zeros | yes | SR safety if length > 0 |
| 50 | 0/4 | little | RL CRC; absent for TYPE_A | yes if present | not self-protected |

For librasta local, offsets 50 onward are absent because both SR safety and RL
CRC lengths are zero. The total UDP payload is 50 bytes.

## 6220 Heartbeat

| Offset | Width | Endianness | Meaning | In encoded length? | Protected by checksum/CRC |
|---:|---:|---|---|---|---|
| 0 | 2 | little | RL length, includes RL header, SR PDU, RL CRC | yes | RL CRC if width > 0 |
| 2 | 2 | little | RL reserve | yes | RL CRC if width > 0 |
| 4 | 4 | little | RL sequence number | yes | RL CRC if width > 0 |
| 8 | 2 | little | SR length, includes SR header, payload, SR safety | yes | SR safety if length > 0 |
| 10 | 2 | little | SR type 6220 | yes | SR safety if length > 0 |
| 12 | 4 | little | receiver ID | yes | SR safety if length > 0 |
| 16 | 4 | little | sender ID | yes | SR safety if length > 0 |
| 20 | 4 | little | SR sequence number | yes | SR safety if length > 0 |
| 24 | 4 | little | confirmed SR sequence | yes | SR safety if length > 0 |
| 28 | 4 | little | timestamp | yes | SR safety if length > 0 |
| 32 | 4 | little | confirmed timestamp | yes | SR safety if length > 0 |
| 36 | 0/4 | little | RL CRC; absent for TYPE_A | yes if present | not self-protected |

For librasta local, the heartbeat has no payload, no SR safety field, and no RL
CRC. The total UDP payload is 36 bytes.

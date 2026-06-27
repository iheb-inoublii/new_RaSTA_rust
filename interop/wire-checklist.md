# Wire-field comparison checklist

Use this for the first failing packet. Fix the earliest mismatch first; do not
adjust multiple fields at once.

| Field | Rust value | Peer value | Match? | Notes |
|---|---|---|---|---|
| UDP source IP/port |  |  |  |  |
| UDP destination IP/port |  |  |  |  |
| RL total length |  |  |  |  |
| RL sequence |  |  |  |  |
| RL reserve |  |  |  |  |
| RL check code |  |  |  |  |
| SRL message type |  |  |  |  |
| Protocol version |  |  |  | Applies to connection PDUs |
| Sender ID |  |  |  |  |
| Receiver ID |  |  |  |  |
| SRL sequence |  |  |  |  |
| Confirmed sequence |  |  |  | Note special `RetransmissionRequest` semantics |
| Timestamp |  |  |  |  |
| Confirmed timestamp |  |  |  |  |
| Payload |  |  |  |  |
| Safety-code coverage |  |  |  |  |
| Safety-code bytes |  |  |  |  |
| Byte order |  |  |  |  |
| Total UDP payload length |  |  |  |  |

## First-failure workflow

1. Identify the earliest packet where behavior diverges.
2. Fill this table from packet capture, Rust `--trace-wire`, and peer logs.
3. Determine whether the mismatch is configuration, wire layout, byte order,
   checksum, timing, or state-machine behavior.
4. Change only one documented item at a time.
5. Re-run from Phase 0 or the earliest affected phase.

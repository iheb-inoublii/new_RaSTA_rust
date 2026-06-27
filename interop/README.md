# Controlled interoperability harness

This directory prepares a controlled interoperability campaign against an
independent C or C++ RaSTA implementation.

No external interoperability has passed yet. Two instances of this Rust
implementation are not independent interoperability evidence.

## Peer implementation

| Field | Value |
|---|---|
| Implementation name | Pending |
| Repository/source location | Pending |
| Language | C or C++ expected; pending selection |
| Supported RaSTA version | Pending |
| Build instructions | Pending |
| Transport mapping | Pending |
| Licence | Pending |
| Configuration mechanism | Pending |
| One/two channel support | Pending |
| Safety-code options | Pending |
| Redundancy CRC options | Pending |

Do not mark a profile parameter as matched until the peer configuration has
been inspected directly.

## Rust node quick start

The original simple commands remain supported:

```bash
cargo run -p rasta-node --release -- B <rust-or-peer-ip>
cargo run -p rasta-node --release -- A <rust-or-peer-ip>
```

Optional interop aids:

```text
--trace-wire
--local-ip <ip>
--channel-0-local-port <port>
--channel-0-remote-port <port>
--channel-1-local-port <port>
--channel-1-remote-port <port>
--local-id <decimal-or-0xhex>
--remote-id <decimal-or-0xhex>
```

Example with tracing and explicit ports:

```bash
cargo run -p rasta-node --release -- A 192.0.2.20 \
  --local-ip 192.0.2.10 \
  --channel-0-local-port 5000 \
  --channel-0-remote-port 5001 \
  --channel-1-local-port 6000 \
  --channel-1-remote-port 6001 \
  --local-id 0x1234 \
  --remote-id 0x5678 \
  --trace-wire
```

The current profile is academic and non-production. Do not silently change it
to match a peer. Record mismatches in `profile-comparison.md` first.

## Documents

- `test-plan.md` — phased test procedure
- `profile-comparison.md` — Rust/peer configuration comparison
- `packet-capture.md` — Wireshark/tcpdump support
- `wire-checklist.md` — first failing packet comparison checklist
- `results/template.md` — result capture template

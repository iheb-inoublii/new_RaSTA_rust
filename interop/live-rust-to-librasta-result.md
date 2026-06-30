# Live Rust-to-librasta interoperability result

Date: 2026-06-30

## Environment

- Rust client: Windows
- C librasta server: Kali Linux
- Rust ID: 0x60
- C server ID: 0x61
- Version: 0303
- Redundancy channels: 2
- T_H: 2000 ms
- T_MAX: 10000 ms
- SR checksum: NONE
- RL CRC profile: TYPE_A
- Test duration: 40 seconds

## Successful behavior

- Rust sent 6200 ConnectionRequest.
- C accepted version 0303.
- C returned 6201 ConnectionResponse.
- Rust transitioned from Opening to Up.
- Rust sent application data: Hello from A.
- C decoded and delivered the application data.
- Heartbeats 6220 were exchanged repeatedly in both directions.
- Peer-relative timestamp normalization remained valid.
- No SafetyTimeout occurred.
- No ConnectionTimeout occurred.
- Rust remained Up for the configured 40-second duration.
- Rust sent graceful 6216 only after the duration expired.

## Command

cargo run -p rasta-node --release -- A 192.168.16.128 --profile librasta-local --trace-wire --run-seconds 40

## Conclusion

Sustained Rust-client to librasta-server interoperability is demonstrated for handshake, redundancy framing, bidirectional heartbeat supervision, timestamp compatibility, application data transfer, and graceful disconnection.

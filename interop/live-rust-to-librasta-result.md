Live Rust-to-librasta interoperability test

Date: 2026-06-30

Result:
- Rust client sent 6200, 50 bytes
- C librasta accepted ConnectionRequest
- C accepted version 0303
- C sent 6201, 50 bytes
- Rust accepted ConnectionResponse
- Rust sent 6220, 36 bytes
- Both sides established the connection
- Rust sent 6240 application data
- C librasta delivered: Hello from A
- C sent heartbeat back to Rust
- Rust disconnected due to timestamp/time-supervision incompatibility

Remaining blocker:
Different protocol timestamp clock origins between Rust on Windows and librasta on Linux.

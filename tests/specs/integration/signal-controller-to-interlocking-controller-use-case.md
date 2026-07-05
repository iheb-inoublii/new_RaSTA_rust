# Signal-controller to interlocking-controller use case

## Objective
Verify the sample signal-controller to interlocking-controller bidirectional application flow.
## Related requirement
Supervisor Step 6 signal/interlocking object-controller example.
## Preconditions
Workspace builds successfully and UDP loopback is available.
## Test setup
Run `interlocking-controller` and `signal-controller` on the same host.
## Test data
`SignalStatus(signal_id=1, Red)`, `MovementAuthority(false)`, `SignalStatus(signal_id=1, GreenRequested)`, `MovementAuthority(true)`, and Ping/Pong counters.
## Test steps
1. Start `cargo run -p interlocking-controller -- 127.0.0.1 --run-seconds 20`.
2. Start `cargo run -p signal-controller -- 127.0.0.1 --run-seconds 20`.
3. Observe the connection reaching `Up`.
4. Observe Red status, denied authority, GreenRequested status, allowed authority, and Ping/Pong rounds.
5. Let the run duration expire.
## Expected result
Messages are exchanged in order and both endpoints disconnect gracefully.
## Postconditions
No process remains running and no protocol panic occurs.
## Evidence
Console logs from both applications.
## Automation status
Application message model and in-memory public API flow are automated; two-process UDP scenario is manual.

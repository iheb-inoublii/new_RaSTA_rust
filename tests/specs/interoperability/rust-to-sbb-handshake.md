# Rust-to-SBB handshake

## Objective
Verify that a Rust endpoint establishes a connection with SBB.
## Related requirement
Future Rust-to-SBB interoperability.
## Preconditions
SBB baseline and confirmed profile values exist. Step 8I Rust-side preparation profile and CLI support exist.
## Test setup
Run Rust active endpoint and SBB passive wrapper with documented configuration.
## Test data
Use `--profile sbb-local`: network ID `123456`, active sender `0x61`, passive sender `0x62`, `t_max = 750 ms`, `t_h = 300 ms`, `t_seq = 50 ms`, lower MD4 safety, RedL option A/no check code.
## Test steps
Start both peers and poll until connected.
## Expected result
Handshake succeeds using evidence-based profile values and observed RedL datagram lengths: ConnReq `58`, Heartbeat `44`, Disconnect `48`.
## Postconditions
Peers remain available for ping-pong testing.
## Evidence
Future SBB interop logs.
## Automation status
Planned.

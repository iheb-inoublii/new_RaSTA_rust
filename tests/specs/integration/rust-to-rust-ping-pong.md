# Rust-to-Rust ping-pong

## Objective
Verify repeated bidirectional Rust-to-Rust Ping/Pong application data exchange over an established RaSTA connection.
## Related requirement
Supervisor Step 7 Rust-to-Rust ping-pong test and demo.
## Preconditions
Rust-to-Rust handshake and data transfer pass. UDP loopback is available for the manual demo.
## Test setup
Automated test uses two Rust endpoints with in-memory transports and a fake clock. Manual demo uses `ping-pong-node` active/passive processes.
## Test data
`ApplicationMessage::Ping { counter }` and `ApplicationMessage::Pong { counter }`, counters `1..=N`.
## Test steps
1. Establish connection.
2. Endpoint A sends `Ping(1)`.
3. Endpoint B receives `Ping(1)` and sends `Pong(1)`.
4. Endpoint A receives `Pong(1)`.
5. Repeat until `N` rounds complete.
6. Continue polling so heartbeats and sequence supervision remain active.
7. Endpoint A disconnects gracefully.
## Expected result
Exactly `N` Ping messages are received by B in counter order and exactly `N` Pong messages are received by A in counter order. No timeout, malformed message, duplicate delivery, or counter gap occurs. Both endpoints disconnect gracefully.
## Postconditions
Both endpoints are down and no diagnostic indicates connection timeout or malformed message.
## Evidence
Automated test result and optional `ping-pong-node` console logs.
## Automation status
Automated by `rust_to_rust_ping_pong_repeats_bidirectional_messages_and_disconnects_cleanly`; manual demo available via `ping-pong-node`.

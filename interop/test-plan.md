# Controlled interoperability test plan

Use statuses: `PASS`, `FAIL`, `BLOCKED`, `NOT RUN`.

## Phase 0 — Build validation

- Rust workspace builds.
- Peer implementation builds.
- Both applications start independently.
- Version and configuration are printed before communication.

Success criterion: both processes run and print their selected profile values.

## Phase 1 — Network reachability

- Confirm local and remote addresses.
- Confirm channel ports.
- Verify UDP packets appear in Wireshark or tcpdump.
- Verify firewalls allow both directions.
- Verify no port collision.

Success criterion: both channels show bidirectional UDP traffic.

## Phase 2 — Redundancy-layer framing

- Rust sends an RL frame.
- Peer receives and accepts it.
- Peer sends an RL frame.
- Rust receives and accepts it.
- Verify RL sequence, reserve field, declared length, and selected CRC.
- Test one channel first only if the peer requires staged setup.
- Then test both configured channels.

Success criterion: both implementations accept the same RL frame format.

## Phase 3 — Connection establishment

Record:

- `ConnectionRequest` bytes;
- `ConnectionResponse` bytes;
- protocol version;
- sender and receiver identifiers;
- initial sequence numbers;
- confirmed-sequence values;
- timestamps;
- safety code;
- resulting states.

Success criterion:

```text
Both implementations reach Up.
```

## Phase 4 — Data transfer

- Rust sends one short application payload.
- Peer receives exact bytes once.
- Peer sends one short payload.
- Rust receives exact bytes once.
- Confirmations advance correctly.
- No duplicate application delivery occurs.

Success criterion: each side receives exactly one copy of the expected payload.

## Phase 5 — Heartbeat and idle operation

- Allow the connection to remain idle.
- Verify heartbeat exchange.
- Verify neither peer times out.
- Verify timestamps and confirmations progress as expected.

Success criterion: both sides remain `Up` beyond several heartbeat intervals.

## Phase 6 — Clean disconnection

- Rust initiates disconnect.
- Peer accepts it.
- Repeat with peer initiating.
- Capture disconnection reason and resulting states.

Success criterion: both sides close cleanly with expected reason codes.

## Later phases — disabled until basic exchange succeeds

Keep these disabled until Phases 0–6 pass:

- one-packet loss;
- retransmission request/response;
- multiple missing packets;
- one-channel failure;
- channel recovery;
- both-channel failure;
- malformed packet rejection;
- timestamp violation;
- invalid confirmation;
- flow-control boundary.

Fix the earliest observed mismatch before changing multiple fields.

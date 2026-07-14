# Rust-to-SBB preparation

## Objective
Verify that the Rust library and `rasta-node` expose an opt-in SBB-compatible
test profile. Interoperability evidence is recorded separately from profile
validation.

## Related requirement
Step 8I Rust-to-SBB preparation after the verified Step 8H SBB-to-SBB wrapper baseline.

## Preconditions
The SBB wrapper SBB-to-SBB baseline has been verified. Rust academic/default and `librasta-local` profiles remain unchanged.

## Test setup
Run Rust unit tests for `RastaProfile::sbb_local()` and `rasta-node` CLI parsing.

## Test data
- network ID `123456`
- active sender `0x61`, passive sender `0x62`
- `t_max = 750 ms`, `t_h = 300 ms`, `t_seq = 50 ms`
- lower MD4 safety code
- RedL option A / no check code
- expected RedL datagram lengths: ConnReq `58`, Heartbeat `44`, Disconnect `48`

## Test steps
1. Construct `RastaProfile::sbb_local()`.
2. Validate that safe validation rejects the profile unless the explicit interoperability opt-in path is used.
3. Encode ConnReq, Heartbeat, and Disconnect frames with the SBB profile.
4. Verify RedL datagram lengths.
5. Parse `rasta-node --profile sbb-local` for role `A` and role `B`.

## Expected result
The SBB preparation profile exists, is opt-in only, preserves known SBB timing and checksum settings, and produces the expected SBB-observed frame lengths. The CLI accepts `--profile sbb-local` with role-specific ID and UDP defaults.

## Postconditions
No Rust protocol behavior is changed for academic/default or `librasta-local`
profiles. The profile test alone is not an interoperability success claim.

## Evidence
Automated Rust tests cover the profile values, opt-in validation, RedL datagram lengths, and CLI parsing.

## Automation status
Automated in Rust unit tests. The subsequent live Rust-to-SBB handshake and
five-round Ping/Pong passed; see the
[completed result](../../../interop/results/sbb-rust-ping-pong-5-rounds.md).

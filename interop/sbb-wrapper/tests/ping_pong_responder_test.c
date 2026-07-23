#include "ping_pong_payload.h"
#include "ping_pong_responder.h"

#include <stdint.h>

static int expect_round_termination(void)
{
    SbbWrapperResponderState state;
    uint32_t counter;

    sbb_wrapper_responder_init(&state, 12u);
    for (counter = 1u; counter <= 12u; counter += 1u) {
        uint8_t pong[SBB_WRAPPER_PING_PONG_PAYLOAD_LEN] = {0};
        size_t written = 0u;
        uint32_t pong_counter = sbb_wrapper_responder_accept_ping(&state, counter);

        if (sbb_wrapper_encode_pong(pong_counter, pong, sizeof(pong), &written) != SBB_WRAPPER_PAYLOAD_OK) {
            return 1;
        }
        if (written != 5u || pong[0] != SBB_WRAPPER_PONG_TAG ||
            pong[1] != (uint8_t)(counter & 0xffu)) {
            return 2;
        }
        sbb_wrapper_responder_note_pong_sent(&state);
        if (counter < 12u && sbb_wrapper_responder_is_complete(&state)) {
            return 3;
        }
    }

    if (!sbb_wrapper_responder_is_complete(&state) ||
        !sbb_wrapper_responder_succeeded(&state)) {
        return 4;
    }
    return 0;
}

static int expect_mismatch_is_not_success(void)
{
    SbbWrapperResponderState state;

    sbb_wrapper_responder_init(&state, 2u);
    (void)sbb_wrapper_responder_accept_ping(&state, 1u);
    sbb_wrapper_responder_note_pong_sent(&state);
    if (sbb_wrapper_responder_accept_ping(&state, 7u) != 7u) {
        return 10;
    }
    sbb_wrapper_responder_note_pong_sent(&state);

    if (!sbb_wrapper_responder_is_complete(&state) ||
        sbb_wrapper_responder_succeeded(&state) ||
        state.malformed_or_mismatched != 1u) {
        return 11;
    }
    return 0;
}

static int expect_large_round_count(void)
{
    SbbWrapperResponderState state;
    uint32_t counter;

    sbb_wrapper_responder_init(&state, 5100u);
    for (counter = 1u; counter <= 5100u; counter += 1u) {
        if (sbb_wrapper_responder_accept_ping(&state, counter) != counter) {
            return 20;
        }
        sbb_wrapper_responder_note_pong_sent(&state);
    }

    return sbb_wrapper_responder_succeeded(&state) ? 0 : 21;
}

static int expect_send_failure_still_terminates_at_requested_pings(void)
{
    SbbWrapperResponderState state;

    sbb_wrapper_responder_init(&state, 2u);
    (void)sbb_wrapper_responder_accept_ping(&state, 1u);
    sbb_wrapper_responder_note_pong_sent(&state);
    (void)sbb_wrapper_responder_accept_ping(&state, 2u);

    if (!sbb_wrapper_responder_is_complete(&state) ||
        sbb_wrapper_responder_succeeded(&state) ||
        state.received_pings != 2u || state.sent_pongs != 1u) {
        return 30;
    }
    return 0;
}

int main(void)
{
    int result = expect_round_termination();
    if (result != 0) {
        return result;
    }
    result = expect_mismatch_is_not_success();
    if (result != 0) {
        return result;
    }
    result = expect_large_round_count();
    if (result != 0) {
        return result;
    }
    return expect_send_failure_still_terminates_at_requested_pings();
}

#include "ping_pong_responder.h"

#include <string.h>

void sbb_wrapper_responder_init(SbbWrapperResponderState *state, uint32_t requested_rounds)
{
    memset(state, 0, sizeof(*state));
    state->requested_rounds = requested_rounds;
    state->expected_counter = 1u;
}

uint32_t sbb_wrapper_responder_accept_ping(SbbWrapperResponderState *state, uint32_t counter)
{
    state->received_pings += 1u;
    if (counter != state->expected_counter) {
        state->malformed_or_mismatched += 1u;
    }
    state->expected_counter += 1u;
    return counter;
}

void sbb_wrapper_responder_note_pong_sent(SbbWrapperResponderState *state)
{
    state->sent_pongs += 1u;
}

void sbb_wrapper_responder_note_malformed(SbbWrapperResponderState *state)
{
    state->malformed_or_mismatched += 1u;
}

int sbb_wrapper_responder_is_complete(const SbbWrapperResponderState *state)
{
    return state->received_pings == state->requested_rounds;
}

int sbb_wrapper_responder_succeeded(const SbbWrapperResponderState *state)
{
    return sbb_wrapper_responder_is_complete(state) &&
           state->sent_pongs == state->requested_rounds &&
           state->malformed_or_mismatched == 0u;
}

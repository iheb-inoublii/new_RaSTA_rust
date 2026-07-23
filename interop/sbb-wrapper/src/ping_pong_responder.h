#ifndef SBB_WRAPPER_PING_PONG_RESPONDER_H
#define SBB_WRAPPER_PING_PONG_RESPONDER_H

#include <stdint.h>

typedef struct SbbWrapperResponderState {
    uint32_t requested_rounds;
    uint32_t received_pings;
    uint32_t sent_pongs;
    uint32_t malformed_or_mismatched;
    uint32_t expected_counter;
} SbbWrapperResponderState;

void sbb_wrapper_responder_init(SbbWrapperResponderState *state, uint32_t requested_rounds);
uint32_t sbb_wrapper_responder_accept_ping(SbbWrapperResponderState *state, uint32_t counter);
void sbb_wrapper_responder_note_pong_sent(SbbWrapperResponderState *state);
void sbb_wrapper_responder_note_malformed(SbbWrapperResponderState *state);
int sbb_wrapper_responder_is_complete(const SbbWrapperResponderState *state);
int sbb_wrapper_responder_succeeded(const SbbWrapperResponderState *state);

#endif

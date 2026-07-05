#ifndef SBB_WRAPPER_PING_PONG_PAYLOAD_H
#define SBB_WRAPPER_PING_PONG_PAYLOAD_H

#include <stddef.h>
#include <stdint.h>

#define SBB_WRAPPER_PING_TAG 0x03u
#define SBB_WRAPPER_PONG_TAG 0x04u
#define SBB_WRAPPER_PING_PONG_PAYLOAD_LEN 5u

typedef enum SbbWrapperPayloadResult {
    SBB_WRAPPER_PAYLOAD_OK = 0,
    SBB_WRAPPER_PAYLOAD_BUFFER_TOO_SMALL = 1,
    SBB_WRAPPER_PAYLOAD_INVALID_LENGTH = 2,
    SBB_WRAPPER_PAYLOAD_UNKNOWN_TAG = 3
} SbbWrapperPayloadResult;

typedef enum SbbWrapperPayloadKind {
    SBB_WRAPPER_PAYLOAD_KIND_PING = SBB_WRAPPER_PING_TAG,
    SBB_WRAPPER_PAYLOAD_KIND_PONG = SBB_WRAPPER_PONG_TAG
} SbbWrapperPayloadKind;

SbbWrapperPayloadResult sbb_wrapper_encode_ping(uint32_t counter, uint8_t *output, size_t capacity, size_t *written);
SbbWrapperPayloadResult sbb_wrapper_encode_pong(uint32_t counter, uint8_t *output, size_t capacity, size_t *written);
SbbWrapperPayloadResult sbb_wrapper_decode_ping_pong(
    const uint8_t *input,
    size_t length,
    SbbWrapperPayloadKind *kind,
    uint32_t *counter);

#endif

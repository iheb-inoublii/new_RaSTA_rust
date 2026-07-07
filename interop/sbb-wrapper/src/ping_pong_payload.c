#include "ping_pong_payload.h"

static SbbWrapperPayloadResult encode_payload(
    uint8_t tag,
    uint32_t counter,
    uint8_t *output,
    size_t capacity,
    size_t *written)
{
    if (capacity < SBB_WRAPPER_PING_PONG_PAYLOAD_LEN || output == 0) {
        return SBB_WRAPPER_PAYLOAD_BUFFER_TOO_SMALL;
    }

    output[0] = tag;
    output[1] = (uint8_t)(counter & 0xffu);
    output[2] = (uint8_t)((counter >> 8) & 0xffu);
    output[3] = (uint8_t)((counter >> 16) & 0xffu);
    output[4] = (uint8_t)((counter >> 24) & 0xffu);

    if (written != 0) {
        *written = SBB_WRAPPER_PING_PONG_PAYLOAD_LEN;
    }

    return SBB_WRAPPER_PAYLOAD_OK;
}

SbbWrapperPayloadResult sbb_wrapper_encode_ping(uint32_t counter, uint8_t *output, size_t capacity, size_t *written)
{
    return encode_payload(SBB_WRAPPER_PING_TAG, counter, output, capacity, written);
}

SbbWrapperPayloadResult sbb_wrapper_encode_pong(uint32_t counter, uint8_t *output, size_t capacity, size_t *written)
{
    return encode_payload(SBB_WRAPPER_PONG_TAG, counter, output, capacity, written);
}

SbbWrapperPayloadResult sbb_wrapper_decode_ping_pong(
    const uint8_t *input,
    size_t length,
    SbbWrapperPayloadKind *kind,
    uint32_t *counter)
{
    if (length != SBB_WRAPPER_PING_PONG_PAYLOAD_LEN || input == 0) {
        return SBB_WRAPPER_PAYLOAD_INVALID_LENGTH;
    }

    if (input[0] != SBB_WRAPPER_PING_TAG && input[0] != SBB_WRAPPER_PONG_TAG) {
        return SBB_WRAPPER_PAYLOAD_UNKNOWN_TAG;
    }

    if (kind != 0) {
        *kind = (SbbWrapperPayloadKind)input[0];
    }

    if (counter != 0) {
        *counter = ((uint32_t)input[1])
            | ((uint32_t)input[2] << 8)
            | ((uint32_t)input[3] << 16)
            | ((uint32_t)input[4] << 24);
    }

    return SBB_WRAPPER_PAYLOAD_OK;
}

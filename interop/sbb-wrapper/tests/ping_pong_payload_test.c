#include "ping_pong_payload.h"

#include <stdint.h>

static int expect_ping(void)
{
    uint8_t buffer[SBB_WRAPPER_PING_PONG_PAYLOAD_LEN] = {0};
    size_t written = 0;
    SbbWrapperPayloadKind kind = SBB_WRAPPER_PAYLOAD_KIND_PONG;
    uint32_t counter = 0;

    if (sbb_wrapper_encode_ping(0x01020304u, buffer, sizeof(buffer), &written) != SBB_WRAPPER_PAYLOAD_OK) {
        return 1;
    }
    if (written != SBB_WRAPPER_PING_PONG_PAYLOAD_LEN) {
        return 2;
    }
    if (buffer[0] != 0x03u || buffer[1] != 0x04u || buffer[2] != 0x03u || buffer[3] != 0x02u || buffer[4] != 0x01u) {
        return 3;
    }
    if (sbb_wrapper_decode_ping_pong(buffer, sizeof(buffer), &kind, &counter) != SBB_WRAPPER_PAYLOAD_OK) {
        return 4;
    }
    if (kind != SBB_WRAPPER_PAYLOAD_KIND_PING || counter != 0x01020304u) {
        return 5;
    }

    return 0;
}

static int expect_pong(void)
{
    uint8_t buffer[SBB_WRAPPER_PING_PONG_PAYLOAD_LEN] = {0};
    size_t written = 0;
    SbbWrapperPayloadKind kind = SBB_WRAPPER_PAYLOAD_KIND_PING;
    uint32_t counter = 0;

    if (sbb_wrapper_encode_pong(7u, buffer, sizeof(buffer), &written) != SBB_WRAPPER_PAYLOAD_OK) {
        return 10;
    }
    if (buffer[0] != 0x04u) {
        return 11;
    }
    if (sbb_wrapper_decode_ping_pong(buffer, sizeof(buffer), &kind, &counter) != SBB_WRAPPER_PAYLOAD_OK) {
        return 12;
    }
    if (kind != SBB_WRAPPER_PAYLOAD_KIND_PONG || counter != 7u) {
        return 13;
    }

    return 0;
}

static int expect_rejects(void)
{
    uint8_t short_buffer[2] = {0x03u, 0x01u};
    uint8_t unknown[5] = {0x99u, 0u, 0u, 0u, 0u};
    uint8_t output[4] = {0};

    if (sbb_wrapper_encode_ping(1u, output, sizeof(output), 0) != SBB_WRAPPER_PAYLOAD_BUFFER_TOO_SMALL) {
        return 20;
    }
    if (sbb_wrapper_decode_ping_pong(short_buffer, sizeof(short_buffer), 0, 0) != SBB_WRAPPER_PAYLOAD_INVALID_LENGTH) {
        return 21;
    }
    if (sbb_wrapper_decode_ping_pong(unknown, sizeof(unknown), 0, 0) != SBB_WRAPPER_PAYLOAD_UNKNOWN_TAG) {
        return 22;
    }

    return 0;
}

int main(void)
{
    int result = expect_ping();
    if (result != 0) {
        return result;
    }

    result = expect_pong();
    if (result != 0) {
        return result;
    }

    return expect_rejects();
}

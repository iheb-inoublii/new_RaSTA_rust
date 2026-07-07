#include "sbb_adapter.h"
#include "udp_transport.h"

#include <stdint.h>
#include <stdio.h>
#include <string.h>

static int expect_no_message(uint32_t channel)
{
    uint8_t buffer[32] = {0};
    uint16_t length = 99u;
    radef_RaStaReturnCode result = redtri_ReadMessage(channel, (uint16_t)sizeof(buffer), &length, buffer);
    if (result != radef_kNoMessageReceived || length != 0u) {
        printf(
            "sbb_transport_notification_test: expected no-message result=%d length=%u\n",
            result,
            length);
        return 1;
    }
    return 0;
}

int main(void)
{
    static const uint8_t payload[] = {0x52u, 0x61u, 0x53u, 0x54u, 0x41u};
    SbbWrapperUdpConfig udp_config = {
        .remote_ip = "127.0.0.1",
        .channel0 = {.local_port = 39400u, .remote_port = 39401u},
        .channel1 = {.local_port = 39401u, .remote_port = 39400u},
        .trace = 1,
    };
    uint8_t buffer[32] = {0};
    uint16_t length = 0u;
    radef_RaStaReturnCode read_result;

    if (sbb_wrapper_udp_init(&udp_config) != 0) {
        puts("sbb_transport_notification_test: UDP init failed");
        return 1;
    }

    redtri_Init();

    if (expect_no_message(1u) != 0) {
        sbb_wrapper_udp_close();
        return 1;
    }

    redtri_SendMessage(0u, (uint16_t)sizeof(payload), payload);

    if (sbb_wrapper_transport_poll_channel(1u) != 1) {
        puts("sbb_transport_notification_test: expected one received datagram");
        sbb_wrapper_udp_close();
        return 1;
    }

    if (sbb_wrapper_transport_pending_count(1u) != 1u) {
        puts("sbb_transport_notification_test: expected pending datagram");
        sbb_wrapper_udp_close();
        return 1;
    }

    read_result = redtri_ReadMessage(1u, (uint16_t)sizeof(buffer), &length, buffer);
    if (read_result != radef_kNoError || length != sizeof(payload) || memcmp(buffer, payload, sizeof(payload)) != 0) {
        printf(
            "sbb_transport_notification_test: read_result=%d length=%u\n",
            read_result,
            length);
        sbb_wrapper_udp_close();
        return 1;
    }

    if (sbb_wrapper_transport_pending_count(1u) != 0u) {
        puts("sbb_transport_notification_test: pending datagram was not consumed");
        sbb_wrapper_udp_close();
        return 1;
    }

    if (expect_no_message(1u) != 0) {
        sbb_wrapper_udp_close();
        return 1;
    }

    sbb_wrapper_udp_close();
    puts("sbb_transport_notification_test: passed");
    return 0;
}

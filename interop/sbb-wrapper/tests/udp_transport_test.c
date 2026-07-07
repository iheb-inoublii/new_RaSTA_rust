#include "udp_transport.h"

#include <stdint.h>
#include <string.h>
#include <sys/select.h>

static void wait_for_udp_delivery(void)
{
    struct timeval timeout;
    timeout.tv_sec = 0;
    timeout.tv_usec = 10000;
    (void)select(0, 0, 0, 0, &timeout);
}

static int receive_with_retry(uint32_t channel, uint8_t *buffer, size_t capacity, size_t *length)
{
    int attempt;
    for (attempt = 0; attempt < 50; attempt += 1) {
        SbbWrapperUdpResult result = sbb_wrapper_udp_receive(channel, buffer, capacity, length);
        if (result == SBB_WRAPPER_UDP_OK) {
            return 0;
        }
        if (result != SBB_WRAPPER_UDP_NO_MESSAGE) {
            return 10 + (int)result;
        }
        wait_for_udp_delivery();
    }
    return 100;
}

int main(void)
{
    static const uint8_t expected[] = {0x03u, 0x04u, 0x03u, 0x02u, 0x01u};
    uint8_t received[sizeof(expected)] = {0};
    size_t received_length = 0u;
    SbbWrapperUdpConfig config;
    SbbWrapperUdpResult result;

    memset(&config, 0, sizeof(config));
    config.remote_ip = "127.0.0.1";
    config.trace = 1;
    config.channel0.local_port = 39000u;
    config.channel0.remote_port = 39001u;
    config.channel1.local_port = 39001u;
    config.channel1.remote_port = 39000u;

    if (sbb_wrapper_udp_init(&config) != 0) {
        return 1;
    }

    result = sbb_wrapper_udp_receive(1u, received, sizeof(received), &received_length);
    if (result != SBB_WRAPPER_UDP_NO_MESSAGE || received_length != 0u) {
        sbb_wrapper_udp_close();
        return 2;
    }

    result = sbb_wrapper_udp_send(0u, expected, sizeof(expected));
    if (result != SBB_WRAPPER_UDP_OK) {
        sbb_wrapper_udp_close();
        return 3;
    }

    if (receive_with_retry(1u, received, sizeof(received), &received_length) != 0) {
        sbb_wrapper_udp_close();
        return 4;
    }

    if (received_length != sizeof(expected) || memcmp(received, expected, sizeof(expected)) != 0) {
        sbb_wrapper_udp_close();
        return 5;
    }

    result = sbb_wrapper_udp_receive(1u, received, sizeof(received), &received_length);
    if (result != SBB_WRAPPER_UDP_NO_MESSAGE || received_length != 0u) {
        sbb_wrapper_udp_close();
        return 6;
    }

    sbb_wrapper_udp_close();
    return 0;
}

#include "sbb_adapter.h"
#include "udp_transport.h"

#include <stdint.h>
#include <string.h>

int main(void)
{
    uint8_t dummy_sr_pdu[RADEF_SR_LAYER_MESSAGE_HEADER_SIZE] = {0};
    uint8_t read_buffer[RADEF_SR_LAYER_MESSAGE_HEADER_SIZE] = {0};
    uint16_t read_length = 0u;
    radef_RaStaReturnCode read_result;
    SbbWrapperUdpConfig config;

    memset(&config, 0, sizeof(config));
    config.remote_ip = "127.0.0.1";
    config.trace = 1;
    config.channel0.local_port = 39100u;
    config.channel0.remote_port = 39101u;
    config.channel1.local_port = 39101u;
    config.channel1.remote_port = 39100u;

    if (sbb_wrapper_udp_init(&config) != 0) {
        return 1;
    }

    redtri_Init();
    sradin_Init();
    sradin_OpenRedundancyChannel(0u);
    sradin_SendMessage(0u, (uint16_t)sizeof(dummy_sr_pdu), dummy_sr_pdu);

    read_result = sradin_ReadMessage(0u, (uint16_t)sizeof(read_buffer), &read_length, read_buffer);
    if (read_result != radef_kNoMessageReceived && read_result != radef_kNoError) {
        sradin_CloseRedundancyChannel(0u);
        sbb_wrapper_udp_close();
        return 2;
    }

    sradin_CloseRedundancyChannel(0u);
    sbb_wrapper_udp_close();
    return 0;
}

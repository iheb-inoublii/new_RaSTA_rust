#include "udp_transport.h"

#include <stdio.h>

void sbb_wrapper_udp_print_config(const SbbWrapperUdpConfig *config)
{
    if (config == 0) {
        puts("[sbb-wrapper] udp config: <null>");
        return;
    }

    printf("[sbb-wrapper] remote_ip=%s\n", config->remote_ip);
    printf(
        "[sbb-wrapper] channel0 local=%u remote=%u\n",
        config->channel0.local_port,
        config->channel0.remote_port);
    printf(
        "[sbb-wrapper] channel1 local=%u remote=%u\n",
        config->channel1.local_port,
        config->channel1.remote_port);
}

int sbb_wrapper_udp_init(const SbbWrapperUdpConfig *config)
{
    if (config == 0 || config->remote_ip == 0) {
        puts("[sbb-wrapper] udp init: invalid configuration");
        return -1;
    }

    puts("[sbb-wrapper] udp init: stubbed, no sockets opened in Step 8C");
    return 0;
}

void sbb_wrapper_udp_close(void)
{
    puts("[sbb-wrapper] udp close: stubbed");
}

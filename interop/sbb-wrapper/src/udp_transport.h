#ifndef SBB_WRAPPER_UDP_TRANSPORT_H
#define SBB_WRAPPER_UDP_TRANSPORT_H

#include <stdint.h>

typedef struct SbbWrapperUdpChannel {
    uint16_t local_port;
    uint16_t remote_port;
} SbbWrapperUdpChannel;

typedef struct SbbWrapperUdpConfig {
    const char *remote_ip;
    SbbWrapperUdpChannel channel0;
    SbbWrapperUdpChannel channel1;
} SbbWrapperUdpConfig;

void sbb_wrapper_udp_print_config(const SbbWrapperUdpConfig *config);
int sbb_wrapper_udp_init(const SbbWrapperUdpConfig *config);
void sbb_wrapper_udp_close(void);

#endif

#ifndef SBB_WRAPPER_UDP_TRANSPORT_H
#define SBB_WRAPPER_UDP_TRANSPORT_H

#include <stddef.h>
#include <stdint.h>

typedef struct SbbWrapperUdpChannel {
    uint16_t local_port;
    uint16_t remote_port;
} SbbWrapperUdpChannel;

typedef struct SbbWrapperUdpConfig {
    const char *remote_ip;
    int trace;
    SbbWrapperUdpChannel channel0;
    SbbWrapperUdpChannel channel1;
} SbbWrapperUdpConfig;

typedef enum SbbWrapperUdpResult {
    SBB_WRAPPER_UDP_OK = 0,
    SBB_WRAPPER_UDP_NO_MESSAGE = 1,
    SBB_WRAPPER_UDP_INVALID_PARAMETER = 2,
    SBB_WRAPPER_UDP_INVALID_CHANNEL = 3,
    SBB_WRAPPER_UDP_MESSAGE_TOO_LARGE = 4,
    SBB_WRAPPER_UDP_OS_ERROR = 5,
    SBB_WRAPPER_UDP_NOT_INITIALIZED = 6
} SbbWrapperUdpResult;

void sbb_wrapper_udp_print_config(const SbbWrapperUdpConfig *config);
int sbb_wrapper_udp_init(const SbbWrapperUdpConfig *config);
void sbb_wrapper_udp_close(void);
int sbb_wrapper_udp_is_initialized(void);
int sbb_wrapper_udp_trace_enabled(void);
SbbWrapperUdpResult sbb_wrapper_udp_send(uint32_t transport_channel_id, const uint8_t *message, size_t length);
SbbWrapperUdpResult sbb_wrapper_udp_receive(
    uint32_t transport_channel_id,
    uint8_t *buffer,
    size_t capacity,
    size_t *length);

#endif

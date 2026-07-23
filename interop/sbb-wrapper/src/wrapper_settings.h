#ifndef SBB_WRAPPER_WRAPPER_SETTINGS_H
#define SBB_WRAPPER_WRAPPER_SETTINGS_H

#include "udp_transport.h"

typedef enum WrapperRole {
    WRAPPER_ROLE_ACTIVE,
    WRAPPER_ROLE_PASSIVE
} WrapperRole;

typedef struct WrapperSettings {
    WrapperRole role;
    const char *remote_ip;
    unsigned int warmup;
    unsigned int rounds;
    const char *csv_path;
    unsigned int ping_delay_ms;
    unsigned int run_seconds;
    int trace;
    int debug_no_abort;
    SbbWrapperUdpConfig udp;
} WrapperSettings;

int sbb_wrapper_parse_settings(int argc, char **argv, WrapperSettings *settings);
unsigned int sbb_wrapper_total_exchanges(const WrapperSettings *settings);

#endif

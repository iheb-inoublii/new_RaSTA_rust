#ifndef SBB_WRAPPER_SBB_ENDPOINT_H
#define SBB_WRAPPER_SBB_ENDPOINT_H

#include "sbb_adapter.h"

#include <stdint.h>

#define SBB_WRAPPER_SAFRETL_NETWORK_ID 123456u
#define SBB_WRAPPER_SAFRETL_SENDER_ID 0x61u
#define SBB_WRAPPER_SAFRETL_RECEIVER_ID 0x62u
#define SBB_WRAPPER_SAFRETL_CONNECTION_ID 0u

typedef enum SbbEndpointRole {
    SBB_ENDPOINT_ROLE_ACTIVE = 0,
    SBB_ENDPOINT_ROLE_PASSIVE = 1
} SbbEndpointRole;

typedef struct SbbEndpoint {
    SbbEndpointRole role;
    uint32_t connection_id;
    int initialized;
    int open_requested;
    int trace;
    int last_state;
    uint32_t poll_count;
} SbbEndpoint;

void sbb_endpoint_configure(SbbEndpoint *endpoint, SbbEndpointRole role, int trace);
radef_RaStaReturnCode sbb_endpoint_init(SbbEndpoint *endpoint);
radef_RaStaReturnCode sbb_endpoint_open(SbbEndpoint *endpoint);
radef_RaStaReturnCode sbb_endpoint_poll(SbbEndpoint *endpoint);
radef_RaStaReturnCode sbb_endpoint_send_ping(SbbEndpoint *endpoint, uint32_t counter);
radef_RaStaReturnCode sbb_endpoint_read(SbbEndpoint *endpoint);
radef_RaStaReturnCode sbb_endpoint_close(SbbEndpoint *endpoint);
int sbb_endpoint_is_up(const SbbEndpoint *endpoint);
const char *sbb_endpoint_state_name(int state);

#endif

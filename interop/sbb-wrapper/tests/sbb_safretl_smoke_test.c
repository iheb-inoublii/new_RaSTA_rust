#include "sbb_endpoint.h"
#include "udp_transport.h"

#include <stdio.h>

int main(void)
{
#ifndef SBB_WRAPPER_HAS_SBB_REDL
    puts("sbb_safretl_smoke_test: skipped because SBB_ROOT was not provided");
    return 0;
#else
    SbbWrapperUdpConfig udp_config = {
        .remote_ip = "127.0.0.1",
        .channel0 = {.local_port = 39300u, .remote_port = 39301u},
        .channel1 = {.local_port = 39301u, .remote_port = 39300u},
        .trace = 0,
    };
    SbbEndpoint endpoint;
    radef_RaStaReturnCode result;

    if (sbb_wrapper_udp_init(&udp_config) != 0) {
        puts("sbb_safretl_smoke_test: UDP init failed");
        return 1;
    }

    redtri_Init();
    sbb_endpoint_configure(&endpoint, SBB_ENDPOINT_ROLE_ACTIVE, 0);

    result = sbb_endpoint_init(&endpoint);
    if (result != radef_kNoError) {
        printf("sbb_safretl_smoke_test: init result=%d\n", result);
        sbb_wrapper_udp_close();
        return 1;
    }

    result = sbb_endpoint_open(&endpoint);
    if (result != radef_kNoError) {
        printf("sbb_safretl_smoke_test: open result=%d\n", result);
        sbb_wrapper_udp_close();
        return 1;
    }

    result = sbb_endpoint_poll(&endpoint);
    if (result != radef_kNoError) {
        printf("sbb_safretl_smoke_test: poll result=%d\n", result);
        sbb_wrapper_udp_close();
        return 1;
    }

    result = sbb_endpoint_close(&endpoint);
    if (result != radef_kNoError) {
        printf("sbb_safretl_smoke_test: close result=%d\n", result);
        sbb_wrapper_udp_close();
        return 1;
    }

    sbb_wrapper_udp_close();
    puts("sbb_safretl_smoke_test: passed");
    return 0;
#endif
}

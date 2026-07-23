#include "sbb_endpoint.h"
#include "udp_transport.h"

#include <stdio.h>
#include <string.h>

int main(int argc, char **argv)
{
#ifndef SBB_WRAPPER_HAS_SBB_REDL
    (void)argc;
    (void)argv;
    puts("sbb_safretl_smoke_test: skipped because SBB_ROOT was not provided");
    return 0;
#else
    SbbWrapperUdpConfig udp_config;
    SbbEndpoint endpoint;
    SbbEndpointRole role;
    radef_RaStaReturnCode result;

    if (argc != 2 || (strcmp(argv[1], "active") != 0 && strcmp(argv[1], "passive") != 0)) {
        puts("usage: sbb_safretl_smoke_test <active|passive>");
        return 2;
    }

    role = strcmp(argv[1], "active") == 0 ? SBB_ENDPOINT_ROLE_ACTIVE : SBB_ENDPOINT_ROLE_PASSIVE;
    udp_config.remote_ip = "127.0.0.1";
    udp_config.channel0.local_port = role == SBB_ENDPOINT_ROLE_ACTIVE ? 39300u : 39310u;
    udp_config.channel0.remote_port = role == SBB_ENDPOINT_ROLE_ACTIVE ? 39301u : 39311u;
    udp_config.channel1.local_port = role == SBB_ENDPOINT_ROLE_ACTIVE ? 39301u : 39311u;
    udp_config.channel1.remote_port = role == SBB_ENDPOINT_ROLE_ACTIVE ? 39300u : 39310u;
    udp_config.trace = 0;

    if (sbb_wrapper_udp_init(&udp_config) != 0) {
        puts("sbb_safretl_smoke_test: UDP init failed");
        return 1;
    }

    redtri_Init();
    sbb_endpoint_configure(&endpoint, role, 0);

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
    printf("sbb_safretl_smoke_test: %s startup passed\n", argv[1]);
    return 0;
#endif
}

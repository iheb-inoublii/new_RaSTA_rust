#include "ping_pong_payload.h"
#include "sbb_adapter.h"
#include "udp_transport.h"

#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef enum WrapperRole {
    WRAPPER_ROLE_ACTIVE,
    WRAPPER_ROLE_PASSIVE
} WrapperRole;

typedef struct WrapperSettings {
    WrapperRole role;
    const char *remote_ip;
    unsigned int rounds;
    unsigned int run_seconds;
    int trace;
    SbbWrapperUdpConfig udp;
} WrapperSettings;

static void print_usage(const char *program)
{
    printf("usage: %s <active|passive> <remote-ip> [options]\n", program);
    puts("");
    puts("options:");
    puts("  --rounds <N>");
    puts("  --run-seconds <N>");
    puts("  --trace");
    puts("  --channel0-local <PORT>");
    puts("  --channel0-remote <PORT>");
    puts("  --channel1-local <PORT>");
    puts("  --channel1-remote <PORT>");
}

static int parse_u16(const char *text, uint16_t *value)
{
    char *end = 0;
    unsigned long parsed = strtoul(text, &end, 10);
    if (text[0] == '\0' || *end != '\0' || parsed > 65535ul) {
        return -1;
    }
    *value = (uint16_t)parsed;
    return 0;
}

static int parse_uint(const char *text, unsigned int *value)
{
    char *end = 0;
    unsigned long parsed = strtoul(text, &end, 10);
    if (text[0] == '\0' || *end != '\0' || parsed > (unsigned long)UINT_MAX) {
        return -1;
    }
    *value = (unsigned int)parsed;
    return 0;
}

static void apply_role_defaults(WrapperSettings *settings)
{
    if (settings->role == WRAPPER_ROLE_ACTIVE) {
        settings->udp.channel0.local_port = 7100u;
        settings->udp.channel0.remote_port = 7000u;
        settings->udp.channel1.local_port = 7101u;
        settings->udp.channel1.remote_port = 7001u;
    } else {
        settings->udp.channel0.local_port = 7000u;
        settings->udp.channel0.remote_port = 7100u;
        settings->udp.channel1.local_port = 7001u;
        settings->udp.channel1.remote_port = 7101u;
    }
}

static int parse_settings(int argc, char **argv, WrapperSettings *settings)
{
    int i = 3;

    if (argc < 3) {
        return -1;
    }

    memset(settings, 0, sizeof(*settings));

    if (strcmp(argv[1], "active") == 0) {
        settings->role = WRAPPER_ROLE_ACTIVE;
    } else if (strcmp(argv[1], "passive") == 0) {
        settings->role = WRAPPER_ROLE_PASSIVE;
    } else {
        return -1;
    }

    settings->remote_ip = argv[2];
    settings->udp.remote_ip = argv[2];
    settings->rounds = 10u;
    settings->run_seconds = 40u;
    apply_role_defaults(settings);

    while (i < argc) {
        if (strcmp(argv[i], "--trace") == 0) {
            settings->trace = 1;
            i += 1;
        } else if (strcmp(argv[i], "--rounds") == 0 && i + 1 < argc) {
            if (parse_uint(argv[i + 1], &settings->rounds) != 0) {
                return -1;
            }
            i += 2;
        } else if (strcmp(argv[i], "--run-seconds") == 0 && i + 1 < argc) {
            if (parse_uint(argv[i + 1], &settings->run_seconds) != 0) {
                return -1;
            }
            i += 2;
        } else if (strcmp(argv[i], "--channel0-local") == 0 && i + 1 < argc) {
            if (parse_u16(argv[i + 1], &settings->udp.channel0.local_port) != 0) {
                return -1;
            }
            i += 2;
        } else if (strcmp(argv[i], "--channel0-remote") == 0 && i + 1 < argc) {
            if (parse_u16(argv[i + 1], &settings->udp.channel0.remote_port) != 0) {
                return -1;
            }
            i += 2;
        } else if (strcmp(argv[i], "--channel1-local") == 0 && i + 1 < argc) {
            if (parse_u16(argv[i + 1], &settings->udp.channel1.local_port) != 0) {
                return -1;
            }
            i += 2;
        } else if (strcmp(argv[i], "--channel1-remote") == 0 && i + 1 < argc) {
            if (parse_u16(argv[i + 1], &settings->udp.channel1.remote_port) != 0) {
                return -1;
            }
            i += 2;
        } else {
            return -1;
        }
    }

    return 0;
}

static void print_settings(const WrapperSettings *settings)
{
    printf("[sbb-wrapper] role=%s\n", settings->role == WRAPPER_ROLE_ACTIVE ? "active" : "passive");
    printf("[sbb-wrapper] rounds=%u\n", settings->rounds);
    printf("[sbb-wrapper] run_seconds=%u\n", settings->run_seconds);
    printf("[sbb-wrapper] trace=%s\n", settings->trace ? "true" : "false");
    sbb_wrapper_udp_print_config(&settings->udp);
}

static void run_stub_smoke_checks(void)
{
    uint8_t payload[SBB_WRAPPER_PING_PONG_PAYLOAD_LEN] = {0};
    uint8_t receive_buffer[128] = {0};
    size_t payload_length = 0;
    size_t received_length = 0;
    RadefReturnCode result;

    if (sbb_wrapper_encode_ping(1u, payload, sizeof(payload), &payload_length) == SBB_WRAPPER_PAYLOAD_OK) {
        result = sradin_SendMessage(0u, payload, payload_length);
        printf("[sbb-wrapper] sradin_SendMessage smoke result=%d\n", result);

        result = redtri_SendMessage(0u, payload, payload_length);
        printf("[sbb-wrapper] redtri_SendMessage smoke result=%d\n", result);
    }

    result = sradin_ReadMessage(0u, receive_buffer, sizeof(receive_buffer), &received_length);
    printf(
        "[sbb-wrapper] sradin_ReadMessage smoke result=%d length=%zu\n",
        result,
        received_length);

    result = redtri_ReadMessage(0u, receive_buffer, sizeof(receive_buffer), &received_length);
    printf(
        "[sbb-wrapper] redtri_ReadMessage smoke result=%d length=%zu\n",
        result,
        received_length);
}

int main(int argc, char **argv)
{
    WrapperSettings settings;

    if (argc == 2 && strcmp(argv[1], "--help") == 0) {
        print_usage(argv[0]);
        return 0;
    }

    if (parse_settings(argc, argv, &settings) != 0) {
        print_usage(argv[0]);
        return 2;
    }

    puts("[sbb-wrapper] Step 8C skeleton only; no SBB interop is claimed");
    print_settings(&settings);

    if (sbb_wrapper_udp_init(&settings.udp) != 0) {
        return 1;
    }

    if (sradin_Init() != radef_kOk || redtri_Init() != radef_kOk) {
        return 1;
    }

    run_stub_smoke_checks();

    puts("[sbb-wrapper] exiting after skeleton initialization");
    sbb_wrapper_udp_close();
    return 0;
}

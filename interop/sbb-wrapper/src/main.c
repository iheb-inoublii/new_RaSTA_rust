#define _POSIX_C_SOURCE 200809L

#include "sbb_diagnostics.h"
#include "sbb_endpoint.h"
#include "udp_transport.h"

#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

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
    int debug_no_abort;
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
    puts("  --debug-no-abort");
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
            settings->udp.trace = 1;
            i += 1;
        } else if (strcmp(argv[i], "--debug-no-abort") == 0) {
            settings->debug_no_abort = 1;
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
    printf("[sbb-wrapper] debug_no_abort=%s\n", settings->debug_no_abort ? "true" : "false");
    sbb_wrapper_udp_print_config(&settings->udp);
}

static uint32_t monotonic_millis(void)
{
    struct timespec now;
    clock_gettime(CLOCK_MONOTONIC, &now);
    return (uint32_t)((now.tv_sec * 1000LL) + (now.tv_nsec / 1000000L));
}

static void sleep_millis(long millis)
{
    struct timespec delay;
    delay.tv_sec = millis / 1000L;
    delay.tv_nsec = (millis % 1000L) * 1000000L;
    nanosleep(&delay, 0);
}

static int passive_smoke_success_ready(const WrapperSettings *settings)
{
    return settings->role == WRAPPER_ROLE_PASSIVE &&
           sbb_wrapper_diag_has_reached_up() &&
           sbb_wrapper_diag_heartbeat_count() >= 1u;
}

int main(int argc, char **argv)
{
    WrapperSettings settings;
    SbbEndpoint endpoint;
    radef_RaStaReturnCode result;
    uint32_t end_time;
    unsigned int next_ping = 1u;

    if (argc == 2 && strcmp(argv[1], "--help") == 0) {
        print_usage(argv[0]);
        return 0;
    }

    if (parse_settings(argc, argv, &settings) != 0) {
        print_usage(argv[0]);
        return 2;
    }

    puts("[sbb-wrapper] Step 8H SBB-to-SBB baseline smoke only; no Rust-to-SBB interop is claimed");
    print_settings(&settings);
    sbb_wrapper_diag_set_debug_no_abort(settings.debug_no_abort);

    sbb_wrapper_diag_set_phase("main:udp_init");
    if (sbb_wrapper_udp_init(&settings.udp) != 0) {
        return 1;
    }

    sbb_wrapper_diag_set_phase("main:redtri_Init");
    redtri_Init();
    sbb_endpoint_configure(
        &endpoint,
        settings.role == WRAPPER_ROLE_ACTIVE ? SBB_ENDPOINT_ROLE_ACTIVE : SBB_ENDPOINT_ROLE_PASSIVE,
        settings.trace);

    sbb_wrapper_diag_set_phase("main:sbb_endpoint_init");
    result = sbb_endpoint_init(&endpoint);
    if (result != radef_kNoError) {
        printf("[sbb-wrapper] SafRetL init failed result=%d(%s)\n", result, sbb_wrapper_rasta_return_code_name(result));
        sbb_wrapper_udp_close();
        return 1;
    }

    sbb_wrapper_diag_set_phase("main:sbb_endpoint_open");
    result = sbb_endpoint_open(&endpoint);
    if (result != radef_kNoError) {
        printf("[sbb-wrapper] SafRetL open failed result=%d(%s)\n", result, sbb_wrapper_rasta_return_code_name(result));
        sbb_wrapper_udp_close();
        return 1;
    }

    end_time = monotonic_millis() + (settings.run_seconds * 1000u);
    while ((int32_t)(end_time - monotonic_millis()) > 0) {
        sbb_wrapper_diag_set_phase("main:poll");
        result = sbb_endpoint_poll(&endpoint);
        if (result != radef_kNoError) {
            printf("[sbb-wrapper] SafRetL poll returned result=%d(%s)\n", result, sbb_wrapper_rasta_return_code_name(result));
            break;
        }
        if (sbb_wrapper_diag_has_fatal()) {
            result = sbb_wrapper_diag_fatal_reason();
            printf(
                "[sbb-wrapper] recorded fatal after poll result=%d(%s); exiting diagnostic run\n",
                result,
                sbb_wrapper_rasta_return_code_name(result));
            break;
        }
        if (sbb_endpoint_is_closed_after_up(&endpoint)) {
            puts("[sbb-wrapper] connection closed after Up; graceful SBB-to-SBB smoke complete");
            break;
        }
        if (passive_smoke_success_ready(&settings)) {
            sbb_wrapper_diag_mark_smoke_complete();
            puts("[sbb-wrapper] passive observed Up and heartbeat; SBB-to-SBB smoke complete");
            puts("[sbb-wrapper] passive smoke success condition reached");
            puts("[sbb-wrapper] stopping SafRetL/RedL polling");
            break;
        }

        sbb_wrapper_diag_set_phase("main:read");
        result = sbb_endpoint_read(&endpoint);
        if (result != radef_kNoError && result != radef_kNoMessageReceived) {
            printf("[sbb-wrapper] SafRetL read returned result=%d(%s)\n", result, sbb_wrapper_rasta_return_code_name(result));
        }
        if (sbb_wrapper_diag_has_fatal()) {
            result = sbb_wrapper_diag_fatal_reason();
            printf(
                "[sbb-wrapper] recorded fatal after read result=%d(%s); exiting diagnostic run\n",
                result,
                sbb_wrapper_rasta_return_code_name(result));
            break;
        }

        if (settings.role == WRAPPER_ROLE_ACTIVE && sbb_endpoint_is_up(&endpoint) && next_ping <= settings.rounds) {
            sbb_wrapper_diag_set_phase("main:send_ping");
            result = sbb_endpoint_send_ping(&endpoint, next_ping);
            if (result == radef_kNoError) {
                printf("[sbb-wrapper] sent Ping(%u)\n", next_ping);
                next_ping += 1u;
            } else if (settings.trace) {
                printf(
                    "[sbb-wrapper] Ping(%u) not sent result=%d(%s)\n",
                    next_ping,
                    result,
                    sbb_wrapper_rasta_return_code_name(result));
            }
        }

        sleep_millis(10L);
    }

    sbb_wrapper_diag_set_phase("main:close");
    if (sbb_wrapper_diag_smoke_complete()) {
        puts("[sbb-wrapper] SafRetL close skipped because smoke already complete");
    } else {
        result = sbb_endpoint_close(&endpoint);
        if (result != radef_kNoError && settings.trace) {
            printf("[sbb-wrapper] SafRetL close returned result=%d(%s)\n", result, sbb_wrapper_rasta_return_code_name(result));
        }
    }

    puts("[sbb-wrapper] exiting after SBB-to-SBB baseline smoke");
    sbb_wrapper_udp_close();
    return 0;
}

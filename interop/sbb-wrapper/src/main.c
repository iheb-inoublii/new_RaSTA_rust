#define _POSIX_C_SOURCE 200809L

#include "sbb_diagnostics.h"
#include "sbb_endpoint.h"
#include "ping_pong_responder.h"
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

    if (settings->rounds == 0u) {
        return -1;
    }

    return 0;
}

static void print_settings(const WrapperSettings *settings)
{
    printf(
        "[sbb-wrapper] startup: role=%s requested_rounds=%u run_seconds=%u trace=%s remote=%s channel0=%u->%u channel1=%u->%u\n",
        settings->role == WRAPPER_ROLE_ACTIVE ? "active" : "passive",
        settings->rounds,
        settings->run_seconds,
        settings->trace ? "true" : "false",
        settings->remote_ip,
        settings->udp.channel0.local_port,
        settings->udp.channel0.remote_port,
        settings->udp.channel1.local_port,
        settings->udp.channel1.remote_port);
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

int main(int argc, char **argv)
{
    WrapperSettings settings;
    SbbEndpoint endpoint;
    radef_RaStaReturnCode result;
    uint32_t end_time;
    unsigned int next_ping = 1u;
    unsigned int expected_pong = 1u;
    unsigned int sent_pings = 0u;
    unsigned int received_pongs = 0u;
    SbbWrapperResponderState responder;
    int application_success = 0;

    if (argc == 2 && strcmp(argv[1], "--help") == 0) {
        print_usage(argv[0]);
        return 0;
    }

    if (parse_settings(argc, argv, &settings) != 0) {
        print_usage(argv[0]);
        return 2;
    }

    print_settings(&settings);
    sbb_wrapper_diag_set_debug_no_abort(settings.debug_no_abort);
    sbb_wrapper_responder_init(&responder, settings.rounds);

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
            if (settings.trace) {
                printf("[sbb-wrapper] SafRetL poll returned result=%d(%s)\n", result, sbb_wrapper_rasta_return_code_name(result));
            }
            break;
        }
        if (sbb_wrapper_diag_has_fatal()) {
            result = sbb_wrapper_diag_fatal_reason();
            if (settings.trace) {
                printf(
                    "[sbb-wrapper] recorded fatal after poll result=%d(%s); exiting diagnostic run\n",
                    result,
                    sbb_wrapper_rasta_return_code_name(result));
            }
            break;
        }
        if (sbb_endpoint_is_closed_after_up(&endpoint)) {
            if (settings.trace) {
                puts("[sbb-wrapper] peer or protocol closed connection after Up before requested rounds completed");
            }
            break;
        }
        sbb_wrapper_diag_set_phase("main:read");
        {
            SbbEndpointAppMessage message;
            result = sbb_endpoint_read_message(&endpoint, &message);
            if (result == radef_kNoError && message.kind == SBB_ENDPOINT_APP_PONG && settings.role == WRAPPER_ROLE_ACTIVE) {
                if (message.counter == expected_pong) {
                    received_pongs += 1u;
                    expected_pong += 1u;
                } else if (settings.trace) {
                    printf(
                        "[sbb-wrapper] unexpected Pong order: expected=%u received=%u\n",
                        expected_pong,
                        (unsigned int)message.counter);
                }
            } else if (result == radef_kNoError && message.kind == SBB_ENDPOINT_APP_PING && settings.role == WRAPPER_ROLE_PASSIVE) {
                uint32_t expected_counter = responder.expected_counter;
                uint32_t pong_counter = sbb_wrapper_responder_accept_ping(&responder, message.counter);
                if (message.counter != expected_counter && settings.trace) {
                    printf(
                        "[sbb-wrapper] unexpected Ping order: expected=%u received=%u\n",
                        expected_counter,
                        (unsigned int)message.counter);
                }
                sbb_wrapper_diag_set_phase("main:send_pong");
                result = sbb_endpoint_send_pong(&endpoint, pong_counter);
                if (result == radef_kNoError) {
                    sbb_wrapper_responder_note_pong_sent(&responder);
                    sbb_wrapper_diag_observe_successful_ping(pong_counter);
                    if (settings.trace) {
                        printf("[sbb-wrapper] sent Pong(%u)\n", (unsigned int)pong_counter);
                    }
                } else if (settings.trace) {
                    printf(
                        "[sbb-wrapper] Pong(%u) not sent result=%d(%s)\n",
                        (unsigned int)message.counter,
                        result,
                        sbb_wrapper_rasta_return_code_name(result));
                }
            } else if (result == radef_kNoError && settings.role == WRAPPER_ROLE_PASSIVE) {
                sbb_wrapper_responder_note_malformed(&responder);
            } else if (result != radef_kNoError && result != radef_kNoMessageReceived && settings.trace) {
                printf("[sbb-wrapper] SafRetL read returned result=%d(%s)\n", result, sbb_wrapper_rasta_return_code_name(result));
            }
        }
        if (sbb_wrapper_diag_has_fatal()) {
            result = sbb_wrapper_diag_fatal_reason();
            if (settings.trace) {
                printf(
                    "[sbb-wrapper] recorded fatal after read result=%d(%s); exiting diagnostic run\n",
                    result,
                    sbb_wrapper_rasta_return_code_name(result));
            }
            break;
        }

        if (settings.role == WRAPPER_ROLE_ACTIVE && sbb_endpoint_is_up(&endpoint) && next_ping <= settings.rounds) {
            sbb_wrapper_diag_set_phase("main:send_ping");
            result = sbb_endpoint_send_ping(&endpoint, next_ping);
            if (result == radef_kNoError) {
                if (settings.trace) {
                    printf("[sbb-wrapper] sent Ping(%u)\n", next_ping);
                }
                sent_pings += 1u;
                next_ping += 1u;
            } else if (settings.trace) {
                printf(
                    "[sbb-wrapper] Ping(%u) not sent result=%d(%s)\n",
                    next_ping,
                    result,
                    sbb_wrapper_rasta_return_code_name(result));
            }
        }

        if (settings.role == WRAPPER_ROLE_ACTIVE && sent_pings == settings.rounds && received_pongs == settings.rounds) {
            application_success = 1;
            break;
        }
        if (settings.role == WRAPPER_ROLE_PASSIVE && sbb_wrapper_responder_is_complete(&responder)) {
            application_success = sbb_wrapper_responder_succeeded(&responder);
            sbb_wrapper_diag_mark_application_complete();
            break;
        }

        if (!sbb_endpoint_is_up(&endpoint)) {
            sleep_millis(10L);
        }
    }

    if (settings.role == WRAPPER_ROLE_ACTIVE) {
        printf(
            "[sbb-wrapper] active summary: sent_pings=%u received_pongs=%u success=%s\n",
            sent_pings,
            received_pongs,
            application_success ? "true" : "false");
    } else {
        sbb_wrapper_diag_print_final(
            responder.requested_rounds,
            responder.received_pings,
            responder.sent_pongs,
            responder.malformed_or_mismatched,
            application_success);
    }

    sbb_wrapper_diag_set_phase("main:close");
    if (sbb_wrapper_diag_application_complete()) {
        if (settings.trace) {
            puts("[sbb-wrapper] SafRetL close skipped after configured application rounds completed");
        }
    } else {
        result = sbb_endpoint_close(&endpoint);
        if (result != radef_kNoError && settings.trace) {
            printf("[sbb-wrapper] SafRetL close returned result=%d(%s)\n", result, sbb_wrapper_rasta_return_code_name(result));
        }
    }

    sbb_wrapper_udp_close();
    return application_success ? 0 : 1;
}

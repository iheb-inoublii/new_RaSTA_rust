#include "wrapper_settings.h"

#include <limits.h>
#include <stdlib.h>
#include <string.h>

#define SBB_WRAPPER_DEFAULT_WARMUP 100u
#define SBB_WRAPPER_DEFAULT_ROUNDS 5000u
#define SBB_WRAPPER_DEFAULT_CSV_PATH "sbb-ping-pong-rtt.csv"

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
        settings->warmup = SBB_WRAPPER_DEFAULT_WARMUP;
        settings->udp.channel0.local_port = 7100u;
        settings->udp.channel0.remote_port = 7000u;
        settings->udp.channel1.local_port = 7101u;
        settings->udp.channel1.remote_port = 7001u;
    } else {
        /*
         * Keep legacy passive "--rounds N" meaning N total exchanges unless
         * the caller explicitly supplies --warmup.
         */
        settings->warmup = 0u;
        settings->udp.channel0.local_port = 7000u;
        settings->udp.channel0.remote_port = 7100u;
        settings->udp.channel1.local_port = 7001u;
        settings->udp.channel1.remote_port = 7101u;
    }
}

int sbb_wrapper_parse_settings(int argc, char **argv, WrapperSettings *settings)
{
    int i = 3;

    if (argc < 3 || settings == 0) {
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
    settings->rounds = SBB_WRAPPER_DEFAULT_ROUNDS;
    settings->csv_path = SBB_WRAPPER_DEFAULT_CSV_PATH;
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
        } else if (strcmp(argv[i], "--warmup") == 0 && i + 1 < argc) {
            if (parse_uint(argv[i + 1], &settings->warmup) != 0) {
                return -1;
            }
            i += 2;
        } else if (strcmp(argv[i], "--rounds") == 0 && i + 1 < argc) {
            if (parse_uint(argv[i + 1], &settings->rounds) != 0) {
                return -1;
            }
            i += 2;
        } else if (strcmp(argv[i], "--csv") == 0 && i + 1 < argc) {
            if (argv[i + 1][0] == '\0') {
                return -1;
            }
            settings->csv_path = argv[i + 1];
            i += 2;
        } else if (strcmp(argv[i], "--ping-delay-ms") == 0 && i + 1 < argc) {
            if (parse_uint(argv[i + 1], &settings->ping_delay_ms) != 0) {
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

    if (settings->rounds == 0u || settings->run_seconds == 0u ||
        settings->warmup >= UINT_MAX - settings->rounds) {
        return -1;
    }
    return 0;
}

unsigned int sbb_wrapper_total_exchanges(const WrapperSettings *settings)
{
    return settings->warmup + settings->rounds;
}

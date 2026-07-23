#include "wrapper_settings.h"

#include <string.h>

static int expect_active_defaults(void)
{
    char *argv[] = {"sbb-rasta-wrapper", "active", "127.0.0.1"};
    WrapperSettings settings;

    if (sbb_wrapper_parse_settings(3, argv, &settings) != 0) {
        return 1;
    }
    if (settings.role != WRAPPER_ROLE_ACTIVE ||
        settings.warmup != 100u ||
        settings.rounds != 5000u ||
        settings.ping_delay_ms != 0u ||
        strcmp(settings.csv_path, "sbb-ping-pong-rtt.csv") != 0 ||
        sbb_wrapper_total_exchanges(&settings) != 5100u) {
        return 2;
    }
    return 0;
}

static int expect_explicit_benchmark_settings(void)
{
    char *argv[] = {
        "sbb-rasta-wrapper",
        "active",
        "127.0.0.1",
        "--warmup",
        "100",
        "--rounds",
        "5000",
        "--csv",
        "benchmark-results/sbb.csv",
        "--ping-delay-ms",
        "0",
        "--run-seconds",
        "600"};
    WrapperSettings settings;

    if (sbb_wrapper_parse_settings(13, argv, &settings) != 0) {
        return 10;
    }
    if (settings.role != WRAPPER_ROLE_ACTIVE ||
        settings.warmup != 100u ||
        settings.rounds != 5000u ||
        settings.ping_delay_ms != 0u ||
        settings.run_seconds != 600u ||
        strcmp(settings.csv_path, "benchmark-results/sbb.csv") != 0 ||
        sbb_wrapper_total_exchanges(&settings) != 5100u) {
        return 11;
    }
    return 0;
}

static int expect_passive_benchmark_total(void)
{
    char *argv[] = {
        "sbb-rasta-wrapper",
        "passive",
        "127.0.0.1",
        "--warmup",
        "100",
        "--rounds",
        "5000"};
    WrapperSettings settings;

    if (sbb_wrapper_parse_settings(7, argv, &settings) != 0) {
        return 15;
    }
    return sbb_wrapper_total_exchanges(&settings) == 5100u ? 0 : 16;
}

static int expect_legacy_passive_round_count(void)
{
    char *argv[] = {
        "sbb-rasta-wrapper",
        "passive",
        "127.0.0.1",
        "--rounds",
        "12"};
    WrapperSettings settings;

    if (sbb_wrapper_parse_settings(5, argv, &settings) != 0) {
        return 20;
    }
    return settings.warmup == 0u &&
               sbb_wrapper_total_exchanges(&settings) == 12u
        ? 0
        : 21;
}

int main(void)
{
    int result = expect_active_defaults();
    if (result != 0) {
        return result;
    }
    result = expect_explicit_benchmark_settings();
    if (result != 0) {
        return result;
    }
    result = expect_passive_benchmark_total();
    if (result != 0) {
        return result;
    }
    return expect_legacy_passive_round_count();
}

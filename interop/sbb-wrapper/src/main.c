#define _POSIX_C_SOURCE 200809L

#include "sbb_diagnostics.h"
#include "sbb_endpoint.h"
#include "ping_pong_responder.h"
#include "rtt_benchmark.h"
#include "udp_transport.h"
#include "wrapper_settings.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static void print_usage(const char *program)
{
    printf("usage: %s <active|passive> <remote-ip> [options]\n", program);
    puts("");
    puts("options:");
    puts("  --warmup <N>");
    puts("  --rounds <N>");
    puts("  --csv <PATH>");
    puts("  --ping-delay-ms <N>");
    puts("  --run-seconds <N>");
    puts("  --trace");
    puts("  --debug-no-abort");
    puts("  --channel0-local <PORT>");
    puts("  --channel0-remote <PORT>");
    puts("  --channel1-local <PORT>");
    puts("  --channel1-remote <PORT>");
}

static void print_settings(const WrapperSettings *settings)
{
    printf(
        "[sbb-wrapper] startup: role=%s warmup=%u rounds=%u ping_delay_ms=%u run_seconds=%u trace=%s remote=%s channel0=%u->%u channel1=%u->%u\n",
        settings->role == WRAPPER_ROLE_ACTIVE ? "active" : "passive",
        settings->warmup,
        settings->rounds,
        settings->ping_delay_ms,
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
    uint64_t *rtt_samples = 0;
    uint64_t ping_delay_ns;
    SbbRttBenchmark benchmark;
    SbbWrapperResponderState responder;
    unsigned int passive_successful_measured = 0u;
    int application_success = 0;

    if (argc == 2 && strcmp(argv[1], "--help") == 0) {
        print_usage(argv[0]);
        return 0;
    }

    if (sbb_wrapper_parse_settings(argc, argv, &settings) != 0) {
        print_usage(argv[0]);
        return 2;
    }

    if (settings.trace) {
        print_settings(&settings);
    }
    if (settings.role == WRAPPER_ROLE_ACTIVE) {
        if ((size_t)settings.rounds > SIZE_MAX / sizeof(*rtt_samples)) {
            return 1;
        }
        rtt_samples = malloc((size_t)settings.rounds * sizeof(*rtt_samples));
        if (rtt_samples == 0) {
            return 1;
        }
    }
    sbb_rtt_benchmark_init(&benchmark, settings.warmup, settings.rounds, rtt_samples);
    ping_delay_ns = (uint64_t)settings.ping_delay_ms * 1000000u;
    sbb_wrapper_diag_set_debug_no_abort(settings.debug_no_abort);
    sbb_wrapper_responder_init(&responder, sbb_wrapper_total_exchanges(&settings));

    sbb_wrapper_diag_set_phase("main:udp_init");
    if (sbb_wrapper_udp_init(&settings.udp) != 0) {
        free(rtt_samples);
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
        free(rtt_samples);
        return 1;
    }

    sbb_wrapper_diag_set_phase("main:sbb_endpoint_open");
    result = sbb_endpoint_open(&endpoint);
    if (result != radef_kNoError) {
        printf("[sbb-wrapper] SafRetL open failed result=%d(%s)\n", result, sbb_wrapper_rasta_return_code_name(result));
        sbb_wrapper_udp_close();
        free(rtt_samples);
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
                if (benchmark.awaiting_pong && message.counter == sbb_rtt_next_counter(&benchmark)) {
                    struct timespec received_at;
                    if (clock_gettime(CLOCK_MONOTONIC, &received_at) == 0) {
                        (void)sbb_rtt_accept_pong(
                            &benchmark,
                            message.counter,
                            &received_at,
                            ping_delay_ns);
                    }
                } else if (settings.trace) {
                    printf(
                        "[sbb-wrapper] unexpected Pong order: expected=%u received=%u\n",
                        sbb_rtt_next_counter(&benchmark),
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
                    if (message.counter == expected_counter &&
                        pong_counter > settings.warmup &&
                        passive_successful_measured < settings.rounds) {
                        passive_successful_measured += 1u;
                    }
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

        if (settings.role == WRAPPER_ROLE_ACTIVE && sbb_endpoint_is_up(&endpoint)) {
            struct timespec now;
            if (clock_gettime(CLOCK_MONOTONIC, &now) == 0) {
                uint64_t now_ns = sbb_rtt_timespec_ns(&now);
                if (sbb_rtt_pong_timed_out(&benchmark, now_ns, 5000000000u)) {
                    sbb_rtt_note_timeout(&benchmark, now_ns, ping_delay_ns);
                }
                if (sbb_rtt_should_send(&benchmark, now_ns)) {
                    uint32_t counter = sbb_rtt_next_counter(&benchmark);
                    struct timespec started_at;
                    sbb_wrapper_diag_set_phase("main:send_ping");
                    result = sbb_endpoint_send_ping_timed(&endpoint, counter, &started_at);
                    if (result == radef_kNoError) {
                        sbb_rtt_note_ping_sent(&benchmark, &started_at);
                        if (settings.trace) {
                            printf("[sbb-wrapper] sent Ping(%u)\n", (unsigned int)counter);
                        }
                    } else {
                        sbb_rtt_note_send_failed(&benchmark, now_ns, ping_delay_ns);
                        if (settings.trace) {
                            printf(
                                "[sbb-wrapper] Ping(%u) not sent result=%d(%s)\n",
                                (unsigned int)counter,
                                result,
                                sbb_wrapper_rasta_return_code_name(result));
                        }
                    }
                }
            }
        }

        if (settings.role == WRAPPER_ROLE_ACTIVE && sbb_rtt_is_complete(&benchmark)) {
            application_success = benchmark.sample_count == settings.rounds;
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

    if (settings.role == WRAPPER_ROLE_ACTIVE && application_success) {
        if (sbb_rtt_write_csv(
                settings.csv_path,
                rtt_samples,
                benchmark.sample_count) != 0) {
            application_success = 0;
        }
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
    if (settings.role == WRAPPER_ROLE_ACTIVE) {
        printf(
            "benchmark summary: warmup_rounds=%u measured_rounds=%u successful_rounds=%u failed_or_timed_out_rounds=%u csv_output_path=%s\n",
            settings.warmup,
            settings.rounds,
            benchmark.sample_count,
            settings.rounds - benchmark.sample_count,
            settings.csv_path);
    } else {
        printf(
            "benchmark summary: warmup_rounds=%u measured_rounds=%u successful_rounds=%u failed_or_timed_out_rounds=%u csv_output_path=not-applicable\n",
            settings.warmup,
            settings.rounds,
            passive_successful_measured,
            settings.rounds - passive_successful_measured);
    }
    free(rtt_samples);
    return application_success ? 0 : 1;
}

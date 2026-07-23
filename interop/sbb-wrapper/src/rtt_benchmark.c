#define _POSIX_C_SOURCE 200809L

#include "rtt_benchmark.h"

#include <stdio.h>
#include <string.h>
#include <time.h>

static uint32_t total_rounds(const SbbRttBenchmark *benchmark)
{
    return benchmark->warmup_rounds + benchmark->measured_rounds;
}

static void advance(SbbRttBenchmark *benchmark, uint64_t now_ns, uint64_t delay_ns)
{
    benchmark->awaiting_pong = 0;
    benchmark->next_counter += 1u;
    benchmark->next_send_at_ns = now_ns + delay_ns;
}

void sbb_rtt_benchmark_init(
    SbbRttBenchmark *benchmark,
    uint32_t warmup_rounds,
    uint32_t measured_rounds,
    uint64_t *samples)
{
    memset(benchmark, 0, sizeof(*benchmark));
    benchmark->warmup_rounds = warmup_rounds;
    benchmark->measured_rounds = measured_rounds;
    benchmark->next_counter = 1u;
    benchmark->samples = samples;
}

uint64_t sbb_rtt_timespec_ns(const struct timespec *value)
{
    return ((uint64_t)value->tv_sec * 1000000000u) + (uint64_t)value->tv_nsec;
}

int sbb_rtt_should_send(const SbbRttBenchmark *benchmark, uint64_t now_ns)
{
    return !benchmark->awaiting_pong &&
           benchmark->next_counter <= total_rounds(benchmark) &&
           now_ns >= benchmark->next_send_at_ns;
}

uint32_t sbb_rtt_next_counter(const SbbRttBenchmark *benchmark)
{
    return benchmark->next_counter;
}

void sbb_rtt_note_ping_sent(SbbRttBenchmark *benchmark, const struct timespec *started_at)
{
    benchmark->sent_at_ns = sbb_rtt_timespec_ns(started_at);
    benchmark->awaiting_pong = 1;
}

void sbb_rtt_note_send_failed(SbbRttBenchmark *benchmark, uint64_t now_ns, uint64_t delay_ns)
{
    advance(benchmark, now_ns, delay_ns);
}

int sbb_rtt_accept_pong(
    SbbRttBenchmark *benchmark,
    uint32_t counter,
    const struct timespec *received_at,
    uint64_t delay_ns)
{
    uint64_t received_at_ns;

    if (!benchmark->awaiting_pong || counter != benchmark->next_counter) {
        return 0;
    }

    received_at_ns = sbb_rtt_timespec_ns(received_at);
    if (counter > benchmark->warmup_rounds &&
        benchmark->sample_count < benchmark->measured_rounds) {
        benchmark->samples[benchmark->sample_count] = received_at_ns - benchmark->sent_at_ns;
        benchmark->sample_count += 1u;
    }
    advance(benchmark, received_at_ns, delay_ns);
    return 1;
}

int sbb_rtt_pong_timed_out(const SbbRttBenchmark *benchmark, uint64_t now_ns, uint64_t timeout_ns)
{
    return benchmark->awaiting_pong &&
           now_ns - benchmark->sent_at_ns >= timeout_ns;
}

void sbb_rtt_note_timeout(SbbRttBenchmark *benchmark, uint64_t now_ns, uint64_t delay_ns)
{
    advance(benchmark, now_ns, delay_ns);
}

int sbb_rtt_is_complete(const SbbRttBenchmark *benchmark)
{
    return !benchmark->awaiting_pong && benchmark->next_counter > total_rounds(benchmark);
}

int sbb_rtt_write_csv(const char *path, const uint64_t *samples, uint32_t sample_count)
{
    uint32_t index;
    FILE *output = fopen(path, "w");
    if (output == 0) {
        return -1;
    }

    if (fprintf(output, "round,rtt_ns,rtt_us,rtt_ms\n") < 0) {
        fclose(output);
        return -1;
    }
    for (index = 0u; index < sample_count; index += 1u) {
        if (fprintf(
                output,
                "%u,%llu,%.3f,%.6f\n",
                (unsigned int)(index + 1u),
                (unsigned long long)samples[index],
                (double)samples[index] / 1000.0,
                (double)samples[index] / 1000000.0) < 0) {
            fclose(output);
            return -1;
        }
    }
    return fclose(output) == 0 ? 0 : -1;
}

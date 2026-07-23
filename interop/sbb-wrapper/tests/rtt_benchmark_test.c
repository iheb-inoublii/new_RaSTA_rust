#define _POSIX_C_SOURCE 200809L

#include "rtt_benchmark.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

static struct timespec at_ns(uint64_t nanoseconds)
{
    struct timespec value;
    value.tv_sec = (time_t)(nanoseconds / 1000000000u);
    value.tv_nsec = (long)(nanoseconds % 1000000000u);
    return value;
}

static int accept_exchange(SbbRttBenchmark *benchmark, uint32_t counter, uint64_t start_ns, uint64_t stop_ns)
{
    struct timespec start = at_ns(start_ns);
    struct timespec stop = at_ns(stop_ns);
    sbb_rtt_note_ping_sent(benchmark, &start);
    return sbb_rtt_accept_pong(benchmark, counter, &stop, 0u);
}

static int expect_matching_and_warmup_exclusion(uint64_t *samples)
{
    SbbRttBenchmark benchmark;
    struct timespec wrong = at_ns(150u);
    uint32_t counter;

    sbb_rtt_benchmark_init(&benchmark, 2u, 3u, samples);
    {
        struct timespec start = at_ns(100u);
        sbb_rtt_note_ping_sent(&benchmark, &start);
    }
    if (sbb_rtt_accept_pong(&benchmark, 99u, &wrong, 0u) != 0 ||
        !benchmark.awaiting_pong || benchmark.sample_count != 0u) {
        return 1;
    }
    if (!sbb_rtt_accept_pong(&benchmark, 1u, &wrong, 0u)) {
        return 2;
    }
    if (!accept_exchange(&benchmark, 2u, 200u, 260u)) {
        return 3;
    }

    for (counter = 3u; counter <= 5u; counter += 1u) {
        uint64_t start_ns = (uint64_t)counter * 100u;
        if (!accept_exchange(&benchmark, counter, start_ns, start_ns + counter)) {
            return 4;
        }
    }

    if (!sbb_rtt_is_complete(&benchmark) ||
        benchmark.sample_count != 3u ||
        samples[0] != 3u ||
        samples[1] != 4u ||
        samples[2] != 5u) {
        return 5;
    }
    return 0;
}

static int expect_csv(const uint64_t *samples)
{
    const char *header = "round,rtt_ns,rtt_us,rtt_ms\n";
    char path[] = "/tmp/sbb-rtt-benchmark-XXXXXX";
    char contents[512] = {0};
    FILE *input;
    int descriptor = mkstemp(path);
    size_t bytes_read;
    unsigned int newline_count = 0u;
    size_t index;

    if (descriptor < 0) {
        return 10;
    }
    close(descriptor);
    if (sbb_rtt_write_csv(path, samples, 3u) != 0) {
        unlink(path);
        return 11;
    }

    input = fopen(path, "r");
    if (input == 0) {
        unlink(path);
        return 12;
    }
    bytes_read = fread(contents, 1u, sizeof(contents) - 1u, input);
    contents[bytes_read] = '\0';
    fclose(input);
    unlink(path);

    if (strncmp(contents, header, strlen(header)) != 0) {
        return 13;
    }
    for (index = 0u; index < bytes_read; index += 1u) {
        if (contents[index] == '\n') {
            newline_count += 1u;
        }
    }
    return newline_count == 4u ? 0 : 14;
}

int main(void)
{
    uint64_t samples[3] = {0u, 0u, 0u};
    int result = expect_matching_and_warmup_exclusion(samples);
    if (result != 0) {
        return result;
    }
    return expect_csv(samples);
}

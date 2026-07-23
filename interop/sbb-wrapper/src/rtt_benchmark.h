#ifndef SBB_WRAPPER_RTT_BENCHMARK_H
#define SBB_WRAPPER_RTT_BENCHMARK_H

#include <stdint.h>

struct timespec;

typedef struct SbbRttBenchmark {
    uint32_t warmup_rounds;
    uint32_t measured_rounds;
    uint32_t next_counter;
    uint32_t sample_count;
    uint64_t *samples;
    uint64_t sent_at_ns;
    uint64_t next_send_at_ns;
    int awaiting_pong;
} SbbRttBenchmark;

void sbb_rtt_benchmark_init(
    SbbRttBenchmark *benchmark,
    uint32_t warmup_rounds,
    uint32_t measured_rounds,
    uint64_t *samples);
uint64_t sbb_rtt_timespec_ns(const struct timespec *value);
int sbb_rtt_should_send(const SbbRttBenchmark *benchmark, uint64_t now_ns);
uint32_t sbb_rtt_next_counter(const SbbRttBenchmark *benchmark);
void sbb_rtt_note_ping_sent(SbbRttBenchmark *benchmark, const struct timespec *started_at);
void sbb_rtt_note_send_failed(SbbRttBenchmark *benchmark, uint64_t now_ns, uint64_t delay_ns);
int sbb_rtt_accept_pong(
    SbbRttBenchmark *benchmark,
    uint32_t counter,
    const struct timespec *received_at,
    uint64_t delay_ns);
int sbb_rtt_pong_timed_out(const SbbRttBenchmark *benchmark, uint64_t now_ns, uint64_t timeout_ns);
void sbb_rtt_note_timeout(SbbRttBenchmark *benchmark, uint64_t now_ns, uint64_t delay_ns);
int sbb_rtt_is_complete(const SbbRttBenchmark *benchmark);
int sbb_rtt_write_csv(const char *path, const uint64_t *samples, uint32_t sample_count);

#endif

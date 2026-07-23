#define _POSIX_C_SOURCE 200809L

#include "sbb_diagnostics.h"
#include "sbb_timeout_instrumentation.h"

#include <stdio.h>
#include <string.h>
#include <unistd.h>

int main(void)
{
    char output[4096] = {0};
    FILE *capture = tmpfile();
    int saved_stdout;
    long buffered_size;
    size_t output_size;

    if (capture == 0) {
        return 1;
    }
    saved_stdout = dup(STDOUT_FILENO);
    if (saved_stdout < 0 || dup2(fileno(capture), STDOUT_FILENO) < 0) {
        fclose(capture);
        return 2;
    }

    sbb_wrapper_diag_set_context("passive", 0u, 0x61u, 0x62u);
    sbb_wrapper_diag_observe_connection_snapshot(4, 3u, 2u, 20u);
    sbb_wrapper_diag_observe_connection_snapshot(4, 1u, 0u, 20u);
    sbb_wrapper_diag_observe_check_timings_result(radef_kNoError);
    sbb_wrapper_diag_observe_read_data_result(radef_kNoMessageReceived);
    sbb_wrapper_diag_observe_send_data_result(radef_kNoError);
    sbb_wrapper_diag_observe_protocol_counters(1u, 2u, 3u, 4u, 5u);
    sbb_wrapper_diag_observe_successful_ping(238u);
    sbb_wrapper_diag_note_timeout_branch(0u, SBB_WRAPPER_TIMEOUT_UP_EVENT);
    sbb_wrapper_diag_observe_connection_notification(1, 0u, 0u, 20u, 4, 0u);

    fflush(stdout);
    buffered_size = ftell(capture);
    if (buffered_size != 0L) {
        return 3;
    }

    sbb_wrapper_diag_print_final(300u, 238u, 238u, 0u, 0);
    fflush(stdout);
    rewind(capture);
    output_size = fread(output, 1u, sizeof(output) - 1u, capture);
    output[output_size] = '\0';

    (void)dup2(saved_stdout, STDOUT_FILENO);
    close(saved_stdout);
    fclose(capture);

    if (strstr(output, "symbolic=sraty_kDiscReasonTimeout numeric=4 detailed=0") == 0 ||
        strstr(output, "last_successful_ping_counter=238") == 0 ||
        strstr(output, "max_send_used=3") == 0 ||
        strstr(output, "last=up_event_timeout") == 0 ||
        strstr(output, "ec_csn=5") == 0) {
        return 4;
    }
    return 0;
}

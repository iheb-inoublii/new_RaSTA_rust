#define _POSIX_C_SOURCE 200809L

#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#include "sbb_diagnostics.h"

#ifdef SBB_WRAPPER_HAS_SBB_REDL
#include "rasta_common/rasys_rasta_system_adapter.h"
#else
#include "sbb_adapter.h"
#endif

uint32_t rasys_GetTimerValue(void)
{
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now) != 0) {
        return 0u;
    }

    return (uint32_t)((now.tv_sec * 1000u) + ((uint32_t)now.tv_nsec / 1000000u));
}

uint32_t rasys_GetTimerGranularity(void)
{
    return 1u;
}

uint32_t rasys_GetRandomNumber(void)
{
    return 0x5bb8f00du;
}

void rasys_FatalError(const radef_RaStaReturnCode error_reason)
{
    sbb_wrapper_diag_record_fatal(error_reason);
    fprintf(
        stderr,
        "[sbb-wrapper] SBB rasys_FatalError called: reason=%u(%s) role=%s connection_id=%u sender_id=0x%02x receiver_id=0x%02x phase=%s debug_no_abort=%s\n",
        (unsigned int)error_reason,
        sbb_wrapper_rasta_return_code_name(error_reason),
        sbb_wrapper_diag_role(),
        (unsigned int)sbb_wrapper_diag_connection_id(),
        (unsigned int)sbb_wrapper_diag_sender_id(),
        (unsigned int)sbb_wrapper_diag_receiver_id(),
        sbb_wrapper_diag_phase(),
        sbb_wrapper_diag_debug_no_abort() ? "true" : "false");
    fflush(stdout);
    fflush(stderr);
    if (sbb_wrapper_diag_debug_no_abort()) {
        return;
    }
    abort();
}

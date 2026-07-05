#define _POSIX_C_SOURCE 200809L

#include <stdio.h>
#include <stdlib.h>
#include <time.h>

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
    fprintf(stderr, "[sbb-wrapper] rasys_FatalError: reason=%u\n", error_reason);
    abort();
}

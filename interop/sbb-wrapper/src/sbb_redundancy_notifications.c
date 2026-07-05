#include "udp_transport.h"

#ifdef SBB_WRAPPER_HAS_SBB_REDL
#include "rasta_redundancy/rednot_red_notifications.h"
#else
#include "sbb_adapter.h"
#endif

#include <stdio.h>

void rednot_MessageReceivedNotification(const uint32_t red_channel_id)
{
    if (sbb_wrapper_udp_trace_enabled()) {
        printf("[sbb-wrapper] rednot_MessageReceivedNotification: red_channel=%u\n", red_channel_id);
    }
}

void rednot_DiagnosticNotification(
    const uint32_t red_channel_id,
    const uint32_t tr_channel_id,
    const radef_TransportChannelDiagnosticData transport_channel_diagnostic_data)
{
    if (sbb_wrapper_udp_trace_enabled()) {
        printf(
            "[sbb-wrapper] rednot_DiagnosticNotification: red_channel=%u transport=%u n_diagnosis=%u n_missed=%u t_drift=%u t_drift2=%u\n",
            red_channel_id,
            tr_channel_id,
            transport_channel_diagnostic_data.n_diagnosis,
            transport_channel_diagnostic_data.n_missed,
            transport_channel_diagnostic_data.t_drift,
            transport_channel_diagnostic_data.t_drift2);
    }
}
